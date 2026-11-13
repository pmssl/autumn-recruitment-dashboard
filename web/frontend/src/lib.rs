use anyhow::{bail, Context, Result};
#[path = "../../../native/src/domain.rs"]
mod domain;
#[path = "../../../native/src/note_links.rs"]
mod note_links;
#[path = "../../../native/src/views.rs"]
mod views;
use domain::{valid_date, validate, Application, ApplicationState, Stage, StageStatus};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
slint::include_modules!();
const TYPES: [&str; 9] = [
    "applied",
    "assessment",
    "written_test",
    "interview",
    "hr",
    "oc",
    "offer",
    "rejected",
    "other",
];
fn timestamp() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}
fn today() -> String {
    let date = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    )
}
fn id(prefix: &str) -> String {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{prefix}_{}_{}",
        js_sys::Date::now() as i64,
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
fn state_label(state: ApplicationState) -> &'static str {
    match state {
        ApplicationState::Active => "进行中",
        ApplicationState::Offer => "Offer",
        ApplicationState::Ended => "已结束",
    }
}
fn stage_label(state: StageStatus) -> &'static str {
    match state {
        StageStatus::Completed => "已完成",
        StageStatus::Pending => "待进行",
        StageStatus::Failed => "已失败",
        StageStatus::Postponed => "已推迟",
    }
}
fn status_index(state: StageStatus) -> i32 {
    match state {
        StageStatus::Completed => 0,
        StageStatus::Pending => 1,
        StageStatus::Failed => 2,
        StageStatus::Postponed => 3,
    }
}
struct State {
    revision: i64,
    loaded: bool,
    pending: bool,
    apps: Vec<Application>,
    editing: Option<String>,
    deleting: Option<String>,
    month: String,
    selected_date: String,
}

fn refresh(ui: &AppWindow, state: &State) {
    let pending_filter = ui.get_filter_index() == 2;
    let mut visible: Vec<_> = state
        .apps
        .iter()
        .filter(|a| a.matches(&ui.get_query(), ui.get_filter_index()))
        .collect();
    visible.sort_by(|a, b| views::latest_date(b).cmp(views::latest_date(a)));
    let rows: Vec<_> = visible
        .into_iter()
        .map(|a| ApplicationRow {
            nodes: {
                let mut nodes = a.stages.iter().collect::<Vec<_>>();
                nodes.sort_by(|a, b| a.sort_order.total_cmp(&b.sort_order));
                ModelRc::new(VecModel::from(
                    nodes
                        .into_iter()
                        .map(|s| StageRow {
                            id: s.id.as_str().into(),
                            title: s.title.as_str().into(),
                            date: s.date.as_deref().unwrap_or_default().into(),
                            state: stage_label(s.status).into(),
                            notes: Default::default(),
                            has_notes: false,
                        })
                        .collect::<Vec<_>>(),
                ))
            },
            id: a.id.as_str().into(),
            company: a.company.as_str().into(),
            position: a.position.as_str().into(),
            chain: a.chain().into(),
            current: if pending_filter {
                views::pending_stage_titles(a)
            } else {
                views::current_stage(a).to_owned()
            }
            .into(),
            latest: views::latest_date(a)
                .chars()
                .take(10)
                .collect::<String>()
                .into(),
            state: state_label(a.state()).into(),
        })
        .collect();
    ui.set_applications(ModelRc::new(VecModel::from(rows)));
    refresh_views(ui, state);
    ui.set_counts(ModelRc::new(VecModel::from(vec![
        state.apps.len() as i32,
        state
            .apps
            .iter()
            .filter(|a| a.state() == ApplicationState::Active)
            .count() as i32,
        state.apps.iter().filter(|a| a.has_pending()).count() as i32,
        state
            .apps
            .iter()
            .filter(|a| a.state() == ApplicationState::Offer)
            .count() as i32,
        state
            .apps
            .iter()
            .filter(|a| a.state() == ApplicationState::Ended)
            .count() as i32,
    ])));
    ui.set_summary(
        format!(
            "总投递 {}    进行中 {}    Offer {}    已结束 {}    有未完成进展 {}",
            state.apps.len(),
            state
                .apps
                .iter()
                .filter(|a| a.state() == ApplicationState::Active)
                .count(),
            state
                .apps
                .iter()
                .filter(|a| a.state() == ApplicationState::Offer)
                .count(),
            state
                .apps
                .iter()
                .filter(|a| a.state() == ApplicationState::Ended)
                .count(),
            state.apps.iter().filter(|a| a.has_pending()).count()
        )
        .into(),
    );
    if let Some(a) = state
        .apps
        .iter()
        .find(|a| a.id == ui.get_selected_id().as_str())
    {
        ui.set_selected_company(a.company.as_str().into());
        ui.set_selected_position(a.position.as_str().into());
        ui.set_selected_title(format!("{} / {}", a.company, a.position).into());
        ui.set_selected_notes(note_links::styled_notes(
            a.notes.as_deref().unwrap_or_default(),
        ));
        ui.set_selected_has_notes(
            a.notes
                .as_deref()
                .is_some_and(|notes| !notes.trim().is_empty()),
        );
        let mut stages = a.stages.clone();
        stages.sort_by(|a, b| a.sort_order.total_cmp(&b.sort_order));
        ui.set_stages(ModelRc::new(VecModel::from(
            stages
                .iter()
                .map(|s| StageRow {
                    id: s.id.as_str().into(),
                    title: s.title.as_str().into(),
                    date: s
                        .date
                        .as_deref()
                        .filter(|d| !d.is_empty())
                        .unwrap_or("日期未填写")
                        .into(),
                    state: stage_label(s.status).into(),
                    notes: note_links::styled_notes(s.notes.as_deref().unwrap_or_default()),
                    has_notes: s
                        .notes
                        .as_deref()
                        .is_some_and(|notes| !notes.trim().is_empty()),
                })
                .collect::<Vec<_>>(),
        )));
    } else {
        ui.set_selected_id("".into());
        ui.set_stages(ModelRc::default());
    }
}
fn refresh_views(ui: &AppWindow, state: &State) {
    let calendar = views::calendar(&state.apps, &ui.get_calendar_query(), ui.get_pending_only());
    let today = today();
    ui.set_month(state.month.as_str().into());
    ui.set_selected_date(state.selected_date.as_str().into());
    let days = views::month_grid(&state.month)
        .unwrap_or_default()
        .iter()
        .map(|date| {
            let key = date.to_string();
            let count = calendar.days.get(&key).map_or(0, Vec::len);
            let previews = calendar
                .days
                .get(&key)
                .into_iter()
                .flatten()
                .take(2)
                .map(|(a, s)| StageRow {
                    id: s.id.as_str().into(),
                    title: format!("{} · {}", a.company, s.title).into(),
                    date: key.as_str().into(),
                    state: stage_label(s.status).into(),
                    notes: Default::default(),
                    has_notes: false,
                })
                .collect::<Vec<_>>();
            DayCell {
                previews: ModelRc::new(VecModel::from(previews)),
                label: if key == today {
                    "今天".into()
                } else {
                    key[8..].to_owned().into()
                },
                count: count as i32,
                outside: !key.starts_with(&state.month),
                today: key == today,
                date: key.into(),
            }
        })
        .collect::<Vec<_>>();
    ui.set_days(ModelRc::new(VecModel::from(days)));
    let month_count: usize = calendar
        .days
        .iter()
        .filter(|(date, _)| date.starts_with(&state.month))
        .map(|(_, events)| events.len())
        .sum();
    ui.set_calendar_summary(
        format!(
            "本月 {month_count} 项进展 · {} 项缺少有效日期 · 浅蓝底日期有进展",
            calendar.undated
        )
        .into(),
    );
    let agenda = calendar
        .days
        .get(&state.selected_date)
        .into_iter()
        .flatten()
        .map(|(a, s)| AgendaRow {
            app_id: a.id.as_str().into(),
            title: format!("{} · {}", s.title, stage_label(s.status)).into(),
            detail: note_links::styled_notes(&format!(
                "{} / {}{}",
                a.company,
                a.position,
                s.notes
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .map(|n| format!(" · {n}"))
                    .unwrap_or_default()
            )),
        })
        .collect::<Vec<_>>();
    ui.set_agenda(ModelRc::new(VecModel::from(agenda)));
    let stats = views::stage_counts(&state.apps)
        .into_iter()
        .map(|(title, count)| StatRow {
            title: title.into(),
            count: count as i32,
            ratio: count as f32 / state.apps.len().max(1) as f32,
        })
        .collect::<Vec<_>>();
    ui.set_visible_stats(ModelRc::new(VecModel::from(
        stats
            .iter()
            .filter(|row| row.count > 0)
            .cloned()
            .collect::<Vec<_>>(),
    )));
    ui.set_stats(ModelRc::new(VecModel::from(stats)));
}
fn set_month(ui: &AppWindow, state: &mut State, month: String) {
    if views::parse_month(&month).is_none() {
        ui.set_notice("月份请填写 1900-01 至 2100-12 范围内的 YYYY-MM".into());
        ui.set_month(state.month.as_str().into());
        return;
    }
    let today = today();
    state.selected_date = if today.starts_with(&month) {
        today
    } else {
        format!("{month}-01")
    };
    state.month = month;
    ui.set_notice("".into());
    refresh_views(ui, state);
}
fn clear_form(ui: &AppWindow, state: &mut State) {
    ui.set_editing_application(false);
    state.editing = None;
    ui.set_company("".into());
    ui.set_position("".into());
    ui.set_url("".into());
    ui.set_notes("".into());
    ui.set_date(today().into());
    ui.set_stage_title("面试".into());
    ui.set_stage_type(3);
    ui.set_stage_status(1);
    ui.set_notice("".into());
}
fn save_form(ui: &AppWindow, state: &State) -> Result<Vec<Application>> {
    let date = ui.get_date().to_string();
    if !date.is_empty() && !valid_date(&date) {
        bail!("日期请填写有效的 YYYY-MM-DD，或留空");
    }
    let mut next = state.apps.clone();
    let now = timestamp();
    if ui.get_form_mode() == 1 {
        let company = ui.get_company().trim().to_owned();
        let position = ui.get_position().trim().to_owned();
        if company.is_empty() || position.is_empty() {
            bail!("公司和岗位不能为空");
        }
        if let Some(editing) = &state.editing {
            let app = next
                .iter_mut()
                .find(|a| &a.id == editing)
                .context("投递不存在")?;
            app.company = company;
            app.position = position;
            app.url = Some(ui.get_url().to_string());
            app.notes = Some(ui.get_notes().to_string());
            app.updated_at = now;
        } else {
            let aid = id("app");
            next.push(Application {
                id: aid.clone(),
                company,
                position,
                url: Some(ui.get_url().to_string()),
                notes: Some(ui.get_notes().to_string()),
                created_at: now.clone(),
                updated_at: now.clone(),
                extra: Default::default(),
                stages: vec![Stage {
                    id: id("stage"),
                    application_id: aid,
                    kind: "applied".into(),
                    title: "投递".into(),
                    date: Some(date),
                    status: StageStatus::Completed,
                    sort_order: 10.,
                    notes: None,
                    created_at: now.clone(),
                    updated_at: now,
                    extra: Default::default(),
                }],
            });
        }
    } else {
        let title = ui.get_stage_title().trim().to_owned();
        if title.is_empty() {
            bail!("进展名称不能为空");
        }
        let app = next
            .iter_mut()
            .find(|a| a.id == ui.get_selected_id().as_str())
            .context("请选择投递")?;
        let kind = TYPES
            .get(ui.get_stage_type() as usize)
            .context("请选择进展类型")?
            .to_string();
        let status = match ui.get_stage_status() {
            0 => StageStatus::Completed,
            1 => StageStatus::Pending,
            2 => StageStatus::Failed,
            3 => StageStatus::Postponed,
            _ => bail!("请选择进展状态"),
        };
        if let Some(editing) = &state.editing {
            let s = app
                .stages
                .iter_mut()
                .find(|s| &s.id == editing)
                .context("进展不存在")?;
            s.kind = kind;
            s.title = title;
            s.date = Some(date);
            s.status = status;
            s.notes = Some(ui.get_notes().to_string());
            s.updated_at = now.clone();
        } else {
            let order = app.stages.iter().map(|s| s.sort_order).fold(0., f64::max) + 10.;
            app.stages.push(Stage {
                id: id("stage"),
                application_id: app.id.clone(),
                kind,
                title,
                date: Some(date),
                status,
                sort_order: order,
                notes: Some(ui.get_notes().to_string()),
                created_at: now.clone(),
                updated_at: now.clone(),
                extra: Default::default(),
            });
        }
        app.updated_at = now;
    }
    validate(&next)?;
    Ok(next)
}
fn wire_ui(ui: &AppWindow, state: Rc<RefCell<State>>) {
    macro_rules! mark {
        ($s:ident, $u:ident, $name:expr) => {{
            let _ = (&$s, &$u, &$name);
        }};
    }
    macro_rules! bind {($method:ident, |$u:ident,$s:ident $(,$arg:ident : $typ:ty)*| $body:block)=>{{
        let weak=ui.as_weak();let state=state.clone();
        ui.$method(move |$($arg:$typ),*| {if let Some($u)=weak.upgrade(){let mut guard=state.borrow_mut();let $s=&mut *guard; let _ = &$s; $body}});
    }}}
    bind!(on_select_page, |u, s, index: i32| {
        const PAGES: [&str; 4] = [
            "导航/招聘进度",
            "导航/全部记录",
            "导航/日历",
            "导航/数据统计",
        ];
        if let Some(name) = PAGES.get(index as usize) {
            if u.get_page() != index {
                u.set_page(index);
                mark!(s, u, name);
            }
        }
    });
    bind!(on_select_filter, |u, s, index: i32| {
        const FILTERS: [&str; 5] = ["全部", "进行中", "有未完成进展", "Offer", "已结束"];
        if let Some(name) = FILTERS.get(index as usize) {
            if u.get_filter_index() != index {
                u.set_filter_index(index);
                refresh(&u, s);
                mark!(
                    s,
                    u,
                    &format!(
                        "{}/筛选/{name}",
                        if u.get_page() == 0 {
                            "招聘进度"
                        } else {
                            "全部记录"
                        }
                    )
                );
            }
        }
    });
    ui.on_track_action(|_| {});
    bind!(on_month_move, |u, s, delta: i32| {
        if let Some(month) = views::move_month(&s.month, delta) {
            let changed = month != s.month;
            set_month(&u, s, month);
            if changed {
                mark!(
                    s,
                    u,
                    if delta < 0 {
                        "日历/上个月"
                    } else {
                        "日历/下个月"
                    }
                );
            }
        }
    });
    bind!(on_month_select, |u, s| {
        let changed = u.get_month().as_str() != s.month;
        set_month(&u, s, u.get_month().to_string());
        if changed && u.get_month().as_str() == s.month {
            mark!(s, u, "日历/跳转");
        }
    });
    bind!(on_calendar_today, |u, s| {
        let changed = s.month != &today()[..7];
        set_month(&u, s, today()[..7].to_owned());
        if changed {
            mark!(s, u, "日历/今天");
        }
    });
    bind!(on_select_date, |u, s, date: SharedString| {
        if valid_date(&date) && views::parse_month(&date[..7]).is_some() {
            let changed = s.selected_date != date.as_str();
            s.selected_date = date.to_string();
            s.month = date[..7].to_owned();
            refresh_views(&u, s);
            if changed {
                mark!(s, u, "日历/选择日期");
            }
        }
    });
    bind!(on_filter_changed, |u, s| {
        refresh(&u, s);
    });
    bind!(on_open_application, |u, s, aid: SharedString| {
        if s.apps.iter().any(|a| a.id == aid.as_str()) {
            let changed = u.get_selected_id() != aid;
            u.set_selected_id(aid);
            refresh(&u, s);
            if changed {
                let page = match u.get_page() {
                    0 => "招聘进度",
                    1 => "全部记录",
                    2 => "日历",
                    _ => "其他",
                };
                mark!(s, u, &format!("{page}/查看投递详情"));
            }
        }
    });
    bind!(on_new_application, |u, s| {
        clear_form(&u, s);
        u.set_form_heading("新增投递".into());
        u.set_form_mode(1);
        mark!(
            s,
            u,
            &format!(
                "{}/新增投递",
                if u.get_page() == 0 {
                    "招聘进度"
                } else {
                    "全部记录"
                }
            )
        );
    });
    bind!(on_edit_application, |u, s| {
        if let Some(a) = s
            .apps
            .iter()
            .find(|a| a.id == u.get_selected_id().as_str())
            .cloned()
        {
            clear_form(&u, s);
            s.editing = Some(a.id);
            u.set_company(a.company.into());
            u.set_position(a.position.into());
            u.set_url(a.url.unwrap_or_default().into());
            u.set_notes(a.notes.unwrap_or_default().into());
            u.set_date("".into());
            u.set_form_heading("编辑投递".into());
            u.set_editing_application(true);
            u.set_form_mode(1);
            mark!(s, u, "投递详情/编辑投递");
        }
    });
    bind!(on_new_stage, |u, s| {
        clear_form(&u, s);
        u.set_form_heading("添加进展".into());
        u.set_form_mode(2);
        mark!(s, u, "投递详情/添加进展");
    });
    bind!(on_new_stage_from_card, |u, s, aid: SharedString| {
        if s.apps.iter().any(|a| a.id == aid.as_str()) {
            u.set_selected_id(aid);
            refresh(&u, s);
            clear_form(&u, s);
            u.set_form_heading("添加进展".into());
            u.set_form_mode(2);
            mark!(s, u, "招聘进度/卡片添加进展");
        }
    });
    bind!(on_edit_stage, |u, s, sid: SharedString| {
        if let Some(stage) = s
            .apps
            .iter()
            .find(|a| a.id == u.get_selected_id().as_str())
            .and_then(|a| a.stages.iter().find(|st| st.id == sid.as_str()))
            .cloned()
        {
            clear_form(&u, s);
            s.editing = Some(stage.id);
            u.set_stage_title(stage.title.into());
            u.set_stage_type(TYPES.iter().position(|t| *t == stage.kind).unwrap_or(8) as i32);
            u.set_stage_status(status_index(stage.status));
            u.set_date(stage.date.unwrap_or_default().into());
            u.set_notes(stage.notes.unwrap_or_default().into());
            u.set_form_heading("编辑进展".into());
            u.set_form_mode(2);
            mark!(s, u, "投递详情/编辑进展");
        }
    });
    let save_state = state.clone();
    bind!(on_save_form, |u, s| {
        if !s.loaded {
            u.set_notice("云端数据尚未读取完成".into());
            return;
        }
        if s.pending {
            u.set_notice("正在保存，请稍候".into());
            return;
        }
        match save_form(&u, s) {
            Ok(next) => {
                s.pending = true;
                submit_change(u.as_weak(), save_state.clone(), s.revision, next, false);
            }
            Err(error) => u.set_notice(format!("保存失败：{error:#}").into()),
        }
    });
    bind!(on_cancel_form, |u, s| {
        let mode = u.get_form_mode();
        s.editing = None;
        u.set_form_mode(0);
        u.set_notice("".into());
        if mode != 0 {
            mark!(
                s,
                u,
                if mode == 1 {
                    "投递表单/取消"
                } else {
                    "进展表单/取消"
                }
            );
        }
    });
    bind!(on_close_form, |u, s| {
        let mode = u.get_form_mode();
        s.editing = None;
        u.set_form_mode(0);
        u.set_notice("".into());
        if mode != 0 {
            mark!(
                s,
                u,
                if mode == 1 {
                    "投递表单/关闭"
                } else {
                    "进展表单/关闭"
                }
            );
        }
    });
    bind!(on_delete_stage, |u, s, sid: SharedString| {
        s.deleting = Some(sid.to_string());
        u.set_confirm_text("删除这个进展节点？其他节点保留。".into());
        u.set_confirm_visible(true);
        mark!(s, u, "投递详情/请求删除进展");
    });
    bind!(on_delete_application, |u, s| {
        s.deleting = None;
        u.set_confirm_text("删除当前投递及其全部进展？".into());
        u.set_confirm_visible(true);
        mark!(s, u, "投递详情/请求删除投递");
    });
    let delete_state = state.clone();
    bind!(on_confirm_delete, |u, s| {
        if !s.loaded {
            u.set_notice("云端数据尚未读取完成".into());
            return;
        }
        if s.pending {
            u.set_notice("正在保存，请稍候".into());
            return;
        }
        let mut next = s.apps.clone();
        let selected = u.get_selected_id();
        if let Some(sid) = &s.deleting {
            if let Some(a) = next.iter_mut().find(|a| a.id == selected.as_str()) {
                a.stages.retain(|st| &st.id != sid);
                a.updated_at = timestamp();
            }
        } else {
            next.retain(|a| a.id != selected.as_str());
        }
        if let Err(error) = validate(&next) {
            u.set_notice(format!("删除失败：{error:#}").into());
        } else {
            s.pending = true;
            submit_change(u.as_weak(), delete_state.clone(), s.revision, next, true);
        }
    });
    bind!(on_open_link, |u, s| {
        if let Some(url) = s
            .apps
            .iter()
            .find(|a| a.id == u.get_selected_id().as_str())
            .and_then(|a| a.url.as_deref())
            .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        {
            let _ = web_sys::window().unwrap().open_with_url(url);
        } else {
            u.set_notice("未填写有效的 http/https 投递链接".into());
        }
    });
    bind!(on_open_note_link, |u, s, url: SharedString| {
        if url.starts_with("https://") || url.starts_with("http://") {
            let _ = web_sys::window().unwrap().open_with_url(url.as_str());
        } else {
            u.set_notice("仅支持打开 http/https 备注链接".into());
        }
    });
    bind!(on_open_folder, |u, s| {
        u.set_notice("数据保存在 Cloudflare D1".into());
    });
    {
        let weak = ui.as_weak();
        ui.on_clear_notice(move || {
            if let Some(u) = weak.upgrade() {
                u.set_notice("".into());
            }
        });
    }
    ui.on_quit(move || {
        let _ = web_sys::window().unwrap().location().reload();
    });
}

fn js_error(value: JsValue) -> anyhow::Error {
    anyhow::anyhow!(value.as_string().unwrap_or_else(|| "浏览器请求失败".into()))
}

async fn api_request(method: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value> {
    let window = web_sys::window().context("浏览器窗口不可用")?;
    let token = window
        .session_storage()
        .map_err(js_error)?
        .context("浏览器会话存储不可用")?
        .get_item("dashboard-token")
        .map_err(js_error)?
        .context("请重新输入访问口令")?;
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    if let Some(body) = body {
        init.set_body(&JsValue::from_str(&body.to_string()));
    }
    let request = web_sys::Request::new_with_str_and_init("/api/data", &init).map_err(js_error)?;
    request
        .headers()
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(js_error)?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(js_error)?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(js_error)?
        .dyn_into()
        .map_err(js_error)?;
    let raw = JsFuture::from(response.text().map_err(js_error)?)
        .await
        .map_err(js_error)?;
    let value: serde_json::Value = serde_json::from_str(&raw.as_string().unwrap_or_default())
        .context("服务端返回了无效数据")?;
    if !response.ok() {
        bail!(
            "{}",
            value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("请求失败")
        );
    }
    Ok(value)
}

fn submit_change(
    weak: slint::Weak<AppWindow>,
    state: Rc<RefCell<State>>,
    revision: i64,
    next: Vec<Application>,
    deleting: bool,
) {
    if let Some(ui) = weak.upgrade() {
        ui.set_notice("正在保存到云端…".into());
    }
    spawn_local(async move {
        let result = api_request(
            "PUT",
            Some(serde_json::json!({
                "revision": revision,
                "applications": next,
            })),
        )
        .await;
        let mut current = state.borrow_mut();
        current.pending = false;
        if let Some(ui) = weak.upgrade() {
            match result {
                Ok(value) => {
                    current.revision = value["revision"].as_i64().unwrap_or(revision + 1);
                    current.apps = next;
                    current.editing = None;
                    current.deleting = None;
                    ui.set_form_mode(0);
                    if deleting {
                        ui.set_confirm_visible(false);
                    }
                    ui.set_notice(
                        if deleting {
                            "删除已保存到云端"
                        } else {
                            "已保存到云端"
                        }
                        .into(),
                    );
                    refresh(&ui, &current);
                }
                Err(error) => ui.set_notice(format!("保存失败：{error:#}").into()),
            }
        }
    });
}

fn load_data(weak: slint::Weak<AppWindow>, state: Rc<RefCell<State>>) {
    spawn_local(async move {
        let result = api_request("GET", None).await.and_then(|value| {
            let revision = value["revision"].as_i64().context("缺少数据版本号")?;
            let apps: Vec<Application> = serde_json::from_value(value["applications"].clone())?;
            validate(&apps)?;
            Ok((revision, apps))
        });
        if let Some(ui) = weak.upgrade() {
            match result {
                Ok((revision, apps)) => {
                    let mut current = state.borrow_mut();
                    current.revision = revision;
                    current.loaded = true;
                    current.apps = apps;
                    ui.set_notice("".into());
                    refresh(&ui, &current);
                }
                Err(error) => ui.set_notice(format!("读取云端数据失败：{error:#}").into()),
            }
        }
    });
}

#[wasm_bindgen(start)]
pub fn start() {
    let ui = AppWindow::new().expect("create Slint window");
    ui.set_data_location("Cloudflare D1".into());
    ui.set_notice("正在读取云端数据…".into());
    let date = today();
    let state = Rc::new(RefCell::new(State {
        revision: 0,
        loaded: false,
        pending: false,
        apps: Vec::new(),
        editing: None,
        deleting: None,
        month: date[..7].into(),
        selected_date: date,
    }));
    refresh(&ui, &state.borrow());
    wire_ui(&ui, state.clone());
    load_data(ui.as_weak(), state);
    ui.run().expect("run Slint window");
}
