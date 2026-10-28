use crate::domain::{valid_date, Application, Stage, StageStatus};
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::BTreeMap;

pub const STAGE_TYPES: [(&str, &str); 9] = [
    ("applied", "投递"),
    ("assessment", "测评"),
    ("written_test", "笔试"),
    ("interview", "面试"),
    ("hr", "HR面"),
    ("oc", "OC"),
    ("offer", "Offer"),
    ("rejected", "已挂"),
    ("other", "其他"),
];

pub struct Calendar<'a> {
    pub days: BTreeMap<String, Vec<(&'a Application, &'a Stage)>>,
    pub undated: usize,
}

pub fn calendar<'a>(apps: &'a [Application], query: &str, pending_only: bool) -> Calendar<'a> {
    let mut result = Calendar {
        days: BTreeMap::new(),
        undated: 0,
    };
    for app in apps.iter().filter(|a| a.matches(query, 0)) {
        for stage in &app.stages {
            if pending_only && stage.status != StageStatus::Pending {
                continue;
            }
            match stage.date.as_deref().filter(|d| valid_date(d)) {
                Some(date) => result
                    .days
                    .entry(date.to_owned())
                    .or_default()
                    .push((app, stage)),
                None => result.undated += 1,
            }
        }
    }
    for entries in result.days.values_mut() {
        entries.sort_by(|(a, s), (b, t)| {
            (t.status == StageStatus::Pending)
                .cmp(&(s.status == StageStatus::Pending))
                .then_with(|| a.company.cmp(&b.company))
                .then_with(|| s.sort_order.total_cmp(&t.sort_order))
        });
    }
    result
}

pub fn parse_month(month: &str) -> Option<NaiveDate> {
    let text = format!("{month}-01");
    let date = NaiveDate::parse_from_str(&text, "%Y-%m-%d").ok()?;
    (valid_date(&text) && (1900..=2100).contains(&date.year())).then_some(date)
}
pub fn move_month(month: &str, delta: i32) -> Option<String> {
    let date = parse_month(month)?;
    let index = date
        .year()
        .checked_mul(12)?
        .checked_add(date.month0() as i32)?
        .checked_add(delta)?;
    let next = NaiveDate::from_ymd_opt(index.div_euclid(12), index.rem_euclid(12) as u32 + 1, 1)?;
    let text = next.format("%Y-%m").to_string();
    parse_month(&text).map(|_| text)
}
pub fn month_grid(month: &str) -> Option<Vec<NaiveDate>> {
    let first = parse_month(month)?;
    let start = first - Duration::days(first.weekday().num_days_from_monday() as i64);
    Some((0..42).map(|i| start + Duration::days(i)).collect())
}
// Count applications containing each type, not the number of stage nodes.
pub fn stage_counts(apps: &[Application]) -> Vec<(&'static str, usize)> {
    STAGE_TYPES
        .iter()
        .map(|(kind, title)| {
            (
                *title,
                apps.iter()
                    .filter(|a| a.stages.iter().any(|s| s.kind == *kind))
                    .count(),
            )
        })
        .collect()
}
pub fn latest_date(app: &Application) -> &str {
    app.stages
        .iter()
        .filter_map(|s| s.date.as_deref())
        .filter(|d| !d.is_empty())
        .max()
        .unwrap_or(&app.updated_at)
}
pub fn current_stage(app: &Application) -> &str {
    app.stages
        .iter()
        .filter(|s| s.status == StageStatus::Completed)
        .max_by(|a, b| a.sort_order.total_cmp(&b.sort_order))
        .or_else(|| {
            app.stages
                .iter()
                .filter(|s| s.date.as_ref().is_some_and(|d| !d.is_empty()))
                .max_by(|a, b| a.date.cmp(&b.date))
        })
        .map(|s| s.title.as_str())
        .unwrap_or("—")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Application> {
        serde_json::from_str(include_str!("../tests/legacy.json")).unwrap()
    }
    #[test]
    fn leap_year_and_month_boundaries() {
        let days = month_grid("2028-02").unwrap();
        assert_eq!(days.len(), 42);
        assert_eq!(days[0].weekday(), chrono::Weekday::Mon);
        assert!(days.iter().any(|d| d.to_string() == "2028-02-29"));
        assert_eq!(move_month("2026-12", 1).as_deref(), Some("2027-01"));
        assert_eq!(move_month("2027-01", -1).as_deref(), Some("2026-12"));
        assert!(move_month("1900-01", -1).is_none());
        assert!(move_month("2100-12", 1).is_none());
        assert!(parse_month("2026-2").is_none());
    }
    #[test]
    fn calendar_filters_nodes_and_handles_invalid_dates() {
        let mut apps = fixture();
        apps[0].stages[1].date = Some("2026-10-01".into());
        let c = calendar(&apps, "测试", false);
        assert_eq!(c.days["2026-10-01"].len(), 2);
        assert_eq!(c.days["2026-10-01"][0].1.status, StageStatus::Pending);
        assert_eq!(calendar(&apps, "", true).days["2026-10-01"].len(), 1);
        assert!(calendar(&apps, "不存在", false).days.is_empty());
        apps[0].stages[1].date = Some("2026-02-29".into());
        let c = calendar(&apps, "", true);
        assert_eq!(c.undated, 1);
        assert!(c.days.is_empty());
    }
    #[test]
    fn stats_deduplicate_stages_per_application() {
        let mut apps = fixture();
        let mut duplicate = apps[0].stages[1].clone();
        duplicate.id = "another-interview".into();
        apps[0].stages.push(duplicate);
        assert_eq!(stage_counts(&apps)[3], ("面试", 1));
        assert_eq!(current_stage(&apps[0]), "投递");
        assert_eq!(latest_date(&apps[0]), "2026-10-01");
        assert!(stage_counts(&[]).iter().all(|(_, n)| *n == 0));
    }
}
