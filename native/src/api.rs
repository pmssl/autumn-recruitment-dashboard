use crate::domain::{valid_date, validate, Application, Stage, StageStatus};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

fn new_id(prefix: &str) -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}_{}_{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .with_context(|| format!("缺少非空字符串字段 {key}"))
}
fn optional(value: &Value, key: &str) -> Result<Option<String>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => bail!("{key} 必须是字符串或 null"),
    }
}
fn patch_optional(value: &Value, key: &str, target: &mut Option<String>) -> Result<()> {
    if value.get(key).is_some() {
        *target = optional(value, key)?;
    }
    Ok(())
}
fn date(value: &Value) -> Result<Option<String>> {
    let result = optional(value, "date")?;
    if let Some(d) = &result {
        if !d.is_empty() && !valid_date(d) {
            bail!("date 应为有效的 YYYY-MM-DD");
        }
    }
    Ok(result)
}
fn kind(value: &str) -> Result<String> {
    if ![
        "applied",
        "assessment",
        "written_test",
        "interview",
        "hr",
        "oc",
        "offer",
        "rejected",
        "other",
    ]
    .contains(&value)
    {
        bail!("无效进展类型：{value}");
    }
    Ok(value.to_owned())
}
fn status(value: &Value) -> Result<StageStatus> {
    serde_json::from_value(value.clone())
        .context("status 应为 completed、pending、failed 或 postponed")
}

/// Execute a local request against a copy of the live state. Caller persists changed state.
pub fn execute(request: &Value, apps: &mut Vec<Application>) -> Result<(Value, bool)> {
    let op = required(request, "op")?;
    let result = match op {
        "list_applications" => {
            let query = optional(request, "query")?.unwrap_or_default();
            let found: Vec<_> = apps.iter().filter(|a| a.matches(&query, 0)).collect();
            return Ok((serde_json::to_value(found)?, false));
        }
        "get_application" => {
            let id = required(request, "id")?;
            return Ok((
                serde_json::to_value(apps.iter().find(|a| a.id == id).context("投递不存在")?)?,
                false,
            ));
        }
        "add_application" => {
            let company = required(request, "company")?.trim().to_owned();
            let position = required(request, "position")?.trim().to_owned();
            let created = now();
            let id = new_id("app");
            let applied_date = if request.get("date").is_some() {
                date(request)?
            } else {
                Some(chrono::Local::now().format("%Y-%m-%d").to_string())
            };
            let app = Application {
                id: id.clone(),
                company,
                position,
                url: optional(request, "url")?,
                notes: optional(request, "notes")?,
                created_at: created.clone(),
                updated_at: created.clone(),
                stages: vec![Stage {
                    id: new_id("stage"),
                    application_id: id,
                    kind: "applied".into(),
                    title: "投递".into(),
                    date: applied_date,
                    status: StageStatus::Completed,
                    sort_order: 10.,
                    notes: None,
                    created_at: created.clone(),
                    updated_at: created,
                    extra: Default::default(),
                }],
                extra: Default::default(),
            };
            apps.push(app.clone());
            serde_json::to_value(app)?
        }
        "update_application" => {
            let id = required(request, "id")?;
            let app = apps.iter_mut().find(|a| a.id == id).context("投递不存在")?;
            for key in ["company", "position"] {
                if request.get(key).is_some() {
                    let new_value = required(request, key)?.trim().to_owned();
                    if key == "company" {
                        app.company = new_value;
                    } else {
                        app.position = new_value;
                    }
                }
            }
            patch_optional(request, "url", &mut app.url)?;
            patch_optional(request, "notes", &mut app.notes)?;
            app.updated_at = now();
            serde_json::to_value(app)?
        }
        "add_stage" => {
            let id = required(request, "applicationId")?;
            let app = apps.iter_mut().find(|a| a.id == id).context("投递不存在")?;
            let created = now();
            let stage = Stage {
                id: new_id("stage"),
                application_id: app.id.clone(),
                kind: kind(required(request, "type")?)?,
                title: required(request, "title")?.trim().to_owned(),
                date: date(request)?,
                status: status(request.get("status").context("缺少 status")?)?,
                sort_order: app.stages.iter().map(|s| s.sort_order).fold(0., f64::max) + 10.,
                notes: optional(request, "notes")?,
                created_at: created.clone(),
                updated_at: created,
                extra: Default::default(),
            };
            app.stages.push(stage.clone());
            app.updated_at = now();
            serde_json::to_value(stage)?
        }
        "update_stage" => {
            let app_id = required(request, "applicationId")?;
            let stage_id = required(request, "id")?;
            let app = apps
                .iter_mut()
                .find(|a| a.id == app_id)
                .context("投递不存在")?;
            let stage = app
                .stages
                .iter_mut()
                .find(|s| s.id == stage_id)
                .context("进展不存在")?;
            if request.get("type").is_some() {
                stage.kind = kind(required(request, "type")?)?;
            }
            if request.get("title").is_some() {
                stage.title = required(request, "title")?.trim().to_owned();
            }
            if request.get("date").is_some() {
                stage.date = date(request)?;
            }
            if let Some(value) = request.get("status") {
                stage.status = status(value)?;
            }
            patch_optional(request, "notes", &mut stage.notes)?;
            stage.updated_at = now();
            app.updated_at = now();
            serde_json::to_value(stage)?
        }
        _ => bail!("未知操作：{op}"),
    };
    validate(apps)?;
    Ok((result, true))
}

pub fn response(result: Result<Value>) -> Value {
    match result {
        Ok(data) => json!({"ok": true, "data": data}),
        Err(e) => json!({"ok": false, "error": format!("{e:#}")}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_rejected_changes() {
        let mut apps = vec![];
        let (app, changed) = execute(&json!({"op":"add_application","company":"测试公司","position":"开发","date":"2026-11-03"}), &mut apps).unwrap();
        assert!(changed);
        let aid = app["id"].as_str().unwrap();
        let (stage, _) = execute(&json!({"op":"add_stage","applicationId":aid,"type":"written_test","title":"笔试","date":"2026-11-04","status":"pending"}), &mut apps).unwrap();
        let sid = stage["id"].as_str().unwrap();
        execute(
            &json!({"op":"update_stage","applicationId":aid,"id":sid,"status":"postponed"}),
            &mut apps,
        )
        .unwrap();
        assert_eq!(apps[0].stages[1].status, StageStatus::Postponed);
        assert_eq!(
            execute(&json!({"op":"list_applications","query":"测试"}), &mut apps)
                .unwrap()
                .0
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(execute(
            &json!({"op":"update_stage","applicationId":aid,"id":sid,"date":"2026-02-30"}),
            &mut apps
        )
        .is_err());
    }
}
