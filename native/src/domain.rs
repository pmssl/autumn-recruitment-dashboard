use anyhow::{bail, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: String,
    pub company: String,
    pub position: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub stages: Vec<Stage>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub id: String,
    pub application_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub status: StageStatus,
    pub sort_order: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Completed,
    Pending,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationState {
    Active,
    Offer,
    Ended,
}

impl Application {
    pub fn state(&self) -> ApplicationState {
        if self
            .stages
            .iter()
            .any(|s| s.kind == "offer" && s.status == StageStatus::Completed)
        {
            ApplicationState::Offer
        } else if self
            .stages
            .iter()
            .any(|s| s.kind == "rejected" || s.status == StageStatus::Failed)
        {
            ApplicationState::Ended
        } else {
            ApplicationState::Active
        }
    }
    pub fn has_pending(&self) -> bool {
        self.stages.iter().any(|s| s.status == StageStatus::Pending)
    }
    pub fn matches(&self, query: &str, filter: i32) -> bool {
        let matches_query = format!("{} {}", self.company, self.position)
            .to_lowercase()
            .contains(&query.trim().to_lowercase());
        matches_query
            && match filter {
                1 => self.state() == ApplicationState::Active,
                2 => self.has_pending(),
                3 => self.state() == ApplicationState::Offer,
                4 => self.state() == ApplicationState::Ended,
                _ => true,
            }
    }
    pub fn chain(&self) -> String {
        let mut stages: Vec<_> = self.stages.iter().collect();
        stages.sort_by(|a, b| a.sort_order.total_cmp(&b.sort_order));
        stages
            .iter()
            .map(|s| {
                format!(
                    "{} {}",
                    match s.status {
                        StageStatus::Completed => "✓",
                        StageStatus::Pending => "○",
                        StageStatus::Failed => "×",
                    },
                    s.title
                )
            })
            .collect::<Vec<_>>()
            .join("  →  ")
    }
}

pub fn valid_date(value: &str) -> bool {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == value)
}

pub fn validate(apps: &[Application]) -> Result<()> {
    let mut app_ids = HashSet::new();
    let mut stage_ids = HashSet::new();
    for app in apps {
        if app.id.is_empty() || !app_ids.insert(&app.id) {
            bail!("投递 ID 为空或重复：{}", app.id);
        }
        if app.company.trim().is_empty() || app.position.trim().is_empty() {
            bail!("公司或岗位为空：{}", app.id);
        }
        for s in &app.stages {
            if s.id.is_empty() || !stage_ids.insert(&s.id) {
                bail!("节点 ID 为空或重复：{}", s.id);
            }
            if s.application_id != app.id {
                bail!("节点 {} 不属于当前投递", s.id);
            }
            if !s.sort_order.is_finite() {
                bail!("节点顺序不是有效数值");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn fixture() -> Vec<Application> {
        serde_json::from_str(include_str!("../tests/legacy.json")).unwrap()
    }
    #[test]
    fn legacy_round_trip_retains_unknown_fields() {
        let mut original: Value =
            serde_json::from_str(include_str!("../tests/legacy.json")).unwrap();
        let apps = fixture();
        validate(&apps).unwrap();
        // JavaScript numbers have no integer/float distinction. Normalize only
        // this modeled numeric field; unknown fields must remain exactly equal.
        for app in original.as_array_mut().unwrap() {
            for stage in app["stages"].as_array_mut().unwrap() {
                stage["sortOrder"] = serde_json::json!(stage["sortOrder"].as_f64().unwrap());
            }
        }
        assert_eq!(
            serde_json::from_slice::<Vec<Application>>(&serde_json::to_vec(&apps).unwrap())
                .unwrap(),
            apps
        );
        assert_eq!(serde_json::to_value(apps).unwrap(), original);
    }
    #[test]
    fn pending_is_independent_of_result() {
        let mut app = fixture().remove(0);
        assert!(app.matches("测试", 2));
        app.stages[0].status = StageStatus::Failed;
        assert_eq!(app.state(), ApplicationState::Ended);
        assert!(app.matches("开发", 2));
        assert!(!app.matches("不存在", 2));
        app.stages[0].kind = "offer".into();
        app.stages[0].status = StageStatus::Completed;
        assert_eq!(app.state(), ApplicationState::Offer);
    }
    #[test]
    fn reject_wrong_owner_duplicate_ids_and_unknown_status() {
        let mut apps = fixture();
        apps[0].stages[0].application_id = "wrong".into();
        assert!(validate(&apps).is_err());
        let apps = fixture();
        assert!(validate(&[apps[0].clone(), apps[0].clone()]).is_err());
        assert!(serde_json::from_str::<Vec<Application>>(
            &include_str!("../tests/legacy.json").replace("pending", "unknown")
        )
        .is_err());
    }
    #[test]
    fn dates_require_real_calendar_days() {
        assert!(valid_date("2028-02-29"));
        assert!(!valid_date("2026-02-29"));
        assert!(!valid_date("2026-2-01"));
        assert!(!valid_date(""));
    }
}
