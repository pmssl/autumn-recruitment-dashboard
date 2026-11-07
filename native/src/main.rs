#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use anyhow::{bail, Context, Result};
use autumn_native::{
    api,
    domain::{valid_date, Application, ApplicationState, Stage, StageStatus},
    local_api, note_links,
    store::Store,
    views,
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, io::Read, path::PathBuf, rc::Rc};
mod click_stats;
use click_stats::ClickTracker;

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
    chrono::Utc::now().to_rfc3339()
}
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
fn id(prefix: &str) -> String {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{prefix}_{}_{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
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
    store: Store,
    tracker: ClickTracker,
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
fn save_form(ui: &AppWindow, state: &mut State) -> Result<()> {
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
    state.store.save(&next)?;
    state.apps = next;
    state.editing = None;
    ui.set_form_mode(0);
    ui.set_notice("已保存到新版独立数据目录".into());
    refresh(ui, state);
    Ok(())
}

fn main() -> Result<()> {
    if let Err(error) = run() {
        eprintln!("启动失败：{error:#}");
        if std::env::args().any(|a| a == "--api") {
            return Err(error);
        }
        // GUI builds have no console: startup failures must remain visible.
        if let Ok(dialog) = StartupErrorWindow::new() {
            dialog.set_message(format!("{error:#}\n\n如果已有预览版在运行，请在旧窗口点击“退出”，或右键托盘图标退出后重试。关闭主窗口可能只会隐藏到托盘。").into());
            dialog.on_quit(|| {
                let _ = slint::quit_event_loop();
            });
            let _ = dialog.run();
        }
        return Err(error);
    }
    Ok(())
}

fn run() -> Result<()> {
    let mut data_dir = None;
    let mut import = None;
    let mut smoke = false;
    let mut api_request = false;
    #[cfg(debug_assertions)]
    let mut capture: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => {
                data_dir = Some(PathBuf::from(args.next().context("--data-dir 需要目录")?))
            }
            "--import-legacy" => {
                import = Some(PathBuf::from(
                    args.next().context("--import-legacy 需要 JSON 文件")?,
                ))
            }
            "--smoke-test" => smoke = true,
            "--api" => api_request = true,
            #[cfg(debug_assertions)]
            "--capture-ui" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .context("--capture-ui needs an output directory")?,
                ));
                smoke = true;
            }
            "--help" => {
                println!("autumn-native [--data-dir DIR] [--import-legacy JSON] [--smoke-test] [--api]\n--api 从标准输入读取一条 JSON 请求，经本机接口交给运行中的程序处理，结果写到标准输出。");
                return Ok(());
            }
            _ => bail!("未知参数：{arg}"),
        }
    }
    let smoke_dir = if smoke {
        Some(tempfile::tempdir()?)
    } else {
        None
    };
    let directory = if let Some(temp) = &smoke_dir {
        temp.path().to_owned()
    } else {
        match data_dir {
            Some(directory) => directory,
            None => std::env::current_exe()
                .context("无法定位程序所在目录")?
                .parent()
                .context("程序路径没有父目录")?
                .join("data"),
        }
    };
    if api_request {
        if smoke || import.is_some() {
            bail!("--api 不能与导入或测试模式同时使用");
        }
        let mut raw = Vec::new();
        std::io::stdin()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut raw)?;
        if raw.len() > 1024 * 1024 {
            bail!("API 请求超过 1 MiB");
        }
        let request: serde_json::Value =
            serde_json::from_slice(&raw).context("标准输入不是有效 JSON")?;
        let response = local_api::call(&directory, &request)?;
        println!("{}", serde_json::to_string(&response)?);
        return Ok(());
    }
    let mut store = Store::open(&directory)?;
    if let Some(source) = import {
        if smoke {
            bail!("测试模式不接受真实导入路径");
        }
        let count = store.import_legacy(&source)?;
        println!("已导入副本：{count} 条投递；源文件未修改");
    }
    let apps = store.load()?;
    let tracker = ClickTracker::open(&directory)?;
    let ui = AppWindow::new()?;
    ui.set_data_location(store.path().display().to_string().into());
    let state = Rc::new(RefCell::new(State {
        store,
        tracker,
        apps,
        editing: None,
        deleting: None,
        month: today()[..7].to_owned(),
        selected_date: today(),
    }));
    refresh(&ui, &state.borrow());
    macro_rules! mark {
        ($s:ident, $u:ident, $name:expr) => {
            if let Err(error) = $s.tracker.record($name) {
                $u.set_notice(format!("点击统计保存失败：{error:#}").into());
            }
        };
    }
    macro_rules! bind {($method:ident, |$u:ident,$s:ident $(,$arg:ident : $typ:ty)*| $body:block)=>{{
        let weak=ui.as_weak();let state=state.clone();
        ui.$method(move |$($arg:$typ),*| {if let Some($u)=weak.upgrade(){let mut guard=state.borrow_mut();let $s=&mut *guard;$body}});
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
    {
        let state = state.clone();
        let weak = ui.as_weak();
        ui.on_track_action(move |name| {
            if let Some(u) = weak.upgrade() {
                if let Err(error) = state.borrow_mut().tracker.record(name.as_str()) {
                    u.set_notice(format!("点击统计保存失败：{error:#}").into());
                }
            }
        });
    }
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
    bind!(on_save_form, |u, s| {
        let mode = u.get_form_mode();
        match save_form(&u, s) {
            Ok(()) => mark!(
                s,
                u,
                if mode == 1 {
                    "投递表单/保存"
                } else {
                    "进展表单/保存"
                }
            ),
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
    bind!(on_confirm_delete, |u, s| {
        let mut next = s.apps.clone();
        let selected = u.get_selected_id();
        let deleting_stage = s.deleting.is_some();
        if let Some(sid) = &s.deleting {
            if let Some(a) = next.iter_mut().find(|a| a.id == selected.as_str()) {
                a.stages.retain(|st| &st.id != sid);
                a.updated_at = timestamp();
            }
        } else {
            next.retain(|a| a.id != selected.as_str());
        }
        match s.store.save(&next) {
            Ok(()) => {
                s.apps = next;
                u.set_notice("删除已保存".into());
                refresh(&u, s);
                mark!(
                    s,
                    u,
                    if deleting_stage {
                        "删除确认/删除进展"
                    } else {
                        "删除确认/删除投递"
                    }
                );
            }
            Err(e) => u.set_notice(format!("删除失败：{e:#}").into()),
        };
        u.set_confirm_visible(false);
    });
    bind!(on_open_link, |u, s| {
        if let Some(url) = s
            .apps
            .iter()
            .find(|a| a.id == u.get_selected_id().as_str())
            .and_then(|a| a.url.as_deref())
            .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        {
            if let Err(e) = open::that(url) {
                u.set_notice(format!("无法打开链接：{e}").into());
            } else {
                mark!(s, u, "投递详情/打开投递链接");
            }
        } else {
            u.set_notice("未填写有效的 http/https 投递链接".into());
        }
    });
    bind!(on_open_note_link, |u, s, url: SharedString| {
        if url.starts_with("https://") || url.starts_with("http://") {
            if let Err(e) = open::that(url.as_str()) {
                u.set_notice(format!("无法打开备注链接：{e}").into());
            } else {
                mark!(
                    s,
                    u,
                    if u.get_selected_id().is_empty() {
                        "日历/打开备注链接"
                    } else {
                        "投递详情/打开备注链接"
                    }
                );
            }
        } else {
            u.set_notice("仅支持打开 http/https 备注链接".into());
        }
    });
    bind!(on_open_folder, |u, s| {
        if let Err(e) = open::that(s.store.path().parent().unwrap()) {
            u.set_notice(format!("无法打开目录：{e}").into());
        } else {
            mark!(s, u, "导航/数据目录");
        }
    });
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_clear_notice(move || {
            if let Some(u) = weak.upgrade() {
                u.set_notice("".into());
                if let Err(error) = state.borrow_mut().tracker.record("提示/关闭") {
                    u.set_notice(format!("点击统计保存失败：{error:#}").into());
                }
            }
        });
    }
    let quit_state = state.clone();
    ui.on_quit(move || {
        if let Err(error) = quit_state.borrow_mut().tracker.record("导航/退出") {
            eprintln!("点击统计保存失败：{error:#}");
        }
        let _ = slint::quit_event_loop();
    });

    let tray = AppTray::new()?;
    let mut pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(32, 32);
    for (i, pixel) in pixels.make_mut_slice().iter_mut().enumerate() {
        let x = (i % 32) as i32;
        let y = (i / 32) as i32;
        let white = (y - 16).abs() < 2 && (6..26).contains(&x)
            || [8, 16, 24]
                .iter()
                .any(|cx| (x - cx).pow(2) + (y - 16).pow(2) < 12);
        *pixel = if white {
            slint::Rgba8Pixel::new(255, 255, 255, 255)
        } else {
            slint::Rgba8Pixel::new(40, 85, 217, 255)
        };
    }
    tray.set_tray_image(slint::Image::from_rgba8(pixels));
    let weak = ui.as_weak();
    tray.on_restore(move || {
        if let Some(u) = weak.upgrade() {
            let _ = u.show();
            u.window().set_minimized(false);
        }
    });
    tray.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
    // The preview does not hide on Linux until tray-host availability is verified.
    if cfg!(target_os = "windows") || cfg!(target_os = "macos") {
        let weak = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(u) = weak.upgrade() {
                let _ = u.hide();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    } else {
        ui.window().on_close_requested(|| {
            // A tray object can keep the loop alive without a usable tray host.
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        });
    }
    #[cfg(debug_assertions)]
    if let Some(output) = capture {
        std::fs::create_dir_all(&output)?;
        let mut fixture: Vec<Application> =
            serde_json::from_str(include_str!("../tests/legacy.json"))?;
        fixture[0].company = "示例科技".into();
        fixture[0].position = "后端开发工程师".into();
        fixture[0].stages[1].date = Some("2026-10-15".into());
        fixture[0].stages[1].notes = Some("测评链接：https://example.com/test\n考试时间：19:00-21:00\n请使用电脑和摄像头。\n试卷有选择、填空、编程题。\n每部分完成后不可返回。\n请保持网络稳定。\n如遇故障可重新打开链接。\n请按时完成。".into());
        let mut after_note = fixture[0].stages[1].clone();
        after_note.id = "preview-after-note".into();
        after_note.title = "后续面试".into();
        after_note.notes = None;
        after_note.sort_order = 30.0;
        fixture[0].stages.push(after_note);
        let mut second = fixture[0].clone();
        second.id = "preview-2".into();
        second.company = "示例网络".into();
        second.position = "基础架构工程师".into();
        second.stages[1].title = "Offer".into();
        second.stages[1].kind = "offer".into();
        second.stages[1].status = StageStatus::Completed;
        for (index, node) in second.stages.iter_mut().enumerate() {
            node.id = format!("preview-stage-{index}");
            node.application_id = second.id.clone();
        }
        let mut postponed = second.stages[1].clone();
        postponed.id = "preview-postponed".into();
        postponed.kind = "written_test".into();
        postponed.title = "笔试".into();
        postponed.date = Some("2026-10-14".into());
        postponed.status = StageStatus::Postponed;
        postponed.sort_order = 15.0;
        second.stages.push(postponed);
        fixture.push(second);
        for kind in 0..3 {
            for index in 0..10 {
                let mut extra = fixture[if kind == 1 { 1 } else { 0 }].clone();
                extra.id = format!("preview-list-{kind}-{index}");
                extra.company = format!("示例企业 {kind}-{index:02}");
                if kind == 2 {
                    extra.stages[1].status = StageStatus::Failed;
                }
                for (node_index, node) in extra.stages.iter_mut().enumerate() {
                    node.id = format!("preview-list-stage-{kind}-{index}-{node_index}");
                    node.application_id = extra.id.clone();
                }
                fixture.push(extra);
            }
        }
        state.borrow_mut().apps = fixture;
        set_month(&ui, &mut state.borrow_mut(), "2026-10".into());
        state.borrow_mut().selected_date = "2026-10-15".into();
        refresh(&ui, &state.borrow());
        ui.show()?;
        for step in 0..10 {
            let weak = ui.as_weak();
            let output = output.clone();
            slint::Timer::single_shot(
                std::time::Duration::from_millis(500 + step * 700),
                move || {
                    let Some(u) = weak.upgrade() else { return };
                    u.set_selected_id("".into());
                    u.set_form_mode(0);
                    u.set_notice("".into());
                    match step {
                        0..=2 => u.set_page(step as i32),
                        3 | 9 => {
                            u.set_page(3);
                            u.set_visible_stats(ModelRc::new(VecModel::from(
                                views::STAGE_TYPES
                                    .iter()
                                    .enumerate()
                                    .map(|(index, (_, title))| StatRow {
                                        title: (*title).into(),
                                        count: (index + 1) as i32,
                                        ratio: (index + 1) as f32 / 9.0,
                                    })
                                    .collect::<Vec<_>>(),
                            )));
                        }
                        4 => {
                            u.set_page(0);
                            u.invoke_open_application("app-test".into());
                        }
                        5 => {
                            u.invoke_open_application("app-test".into());
                            u.invoke_edit_stage("stage-2".into());
                        }
                        6 => {
                            u.window().set_size(slint::LogicalSize::new(900., 700.));
                            u.set_page(0);
                        }
                        7 => {
                            u.window().set_size(slint::LogicalSize::new(1280., 820.));
                            u.set_page(0);
                            u.set_query("不存在的公司".into());
                            u.invoke_filter_changed();
                        }
                        8 => {
                            u.window().set_size(slint::LogicalSize::new(900., 700.));
                        }
                        _ => {}
                    }
                    let weak = u.as_weak();
                    slint::Timer::single_shot(std::time::Duration::from_millis(200), move || {
                        if let Some(u) = weak.upgrade() {
                            let pixels =
                                u.window().take_snapshot().expect("capture rendered window");
                            use std::io::Write;
                            let mut file =
                                std::fs::File::create(output.join(format!("page-{step}.ppm")))
                                    .unwrap();
                            write!(file, "P6\n{} {}\n255\n", pixels.width(), pixels.height())
                                .unwrap();
                            let rgb = pixels
                                .as_slice()
                                .iter()
                                .flat_map(|p| [p.r, p.g, p.b])
                                .collect::<Vec<_>>();
                            file.write_all(&rgb).unwrap();
                            if step == 5 {
                                use slint::platform::{PointerEventButton, WindowEvent};
                                let click = |x, y| {
                                    u.window().dispatch_event(WindowEvent::PointerPressed {
                                        position: slint::LogicalPosition::new(x, y),
                                        button: PointerEventButton::Left,
                                    });
                                    u.window().dispatch_event(WindowEvent::PointerReleased {
                                        position: slint::LogicalPosition::new(x, y),
                                        button: PointerEventButton::Left,
                                    });
                                };
                                click(500., 300.);
                                u.window().dispatch_event(WindowEvent::KeyPressed {
                                    text: "测试输入".into(),
                                });
                                u.window().dispatch_event(WindowEvent::KeyReleased {
                                    text: "测试输入".into(),
                                });
                                assert!(
                                    u.get_stage_title().contains("测试输入"),
                                    "custom text field receives pointer focus and text"
                                );
                                click(823., 649.);
                                assert_eq!(
                                    u.get_form_mode(),
                                    0,
                                    "save button above drawer receives pointer event"
                                );
                                println!(
                                    "PASS: custom field pointer/text input and modal save click"
                                );
                            }
                        }
                        if step == 9 {
                            let _ = slint::quit_event_loop();
                        }
                    });
                },
            );
        }
        slint::run_event_loop()?;
        return Ok(());
    }
    if smoke {
        ui.invoke_select_page(2);
        ui.invoke_select_page(2);
        ui.invoke_select_page(0);
        ui.invoke_select_filter(1);
        ui.invoke_select_filter(1);
        ui.invoke_new_application();
        ui.set_company("中文测试公司".into());
        ui.set_position("开发工程师".into());
        ui.invoke_save_form();
        assert_eq!(state.borrow().apps.len(), 1);
        let aid = state.borrow().apps[0].id.clone();
        ui.invoke_open_application(aid.into());
        ui.invoke_new_stage();
        ui.set_stage_title("一面".into());
        ui.set_date("".into());
        ui.invoke_save_form();
        assert_eq!(state.borrow().apps[0].stages.len(), 2);
        let sid = state.borrow().apps[0].stages[1].id.clone();
        ui.invoke_edit_stage(sid.into());
        ui.set_stage_title("二面".into());
        ui.invoke_save_form();
        assert_eq!(state.borrow().store.load()?[0].stages[1].title, "二面");
        ui.set_filter_index(2);
        ui.invoke_filter_changed();
        use slint::Model;
        assert_eq!(ui.get_applications().row_count(), 1);
        assert_eq!(
            ui.get_applications().row_data(0).unwrap().current.as_str(),
            "二面"
        );
        ui.set_page(2);
        ui.set_month("2028-02".into());
        ui.invoke_month_select();
        assert_eq!(ui.get_month().as_str(), "2028-02");
        ui.invoke_select_date("2028-02-29".into());
        let sid = state.borrow().apps[0].stages[1].id.clone();
        ui.invoke_edit_stage(sid.into());
        ui.set_date("2028-02-29".into());
        ui.invoke_save_form();
        assert_eq!(ui.get_agenda().row_count(), 1);
        assert!(ui
            .get_days()
            .iter()
            .any(|d| d.date.as_str() == "2028-02-29" && d.count == 1));
        assert_eq!(ui.get_stats().row_data(3).unwrap().count, 1);
        assert_eq!(ui.get_visible_stats().row_count(), 2);
        ui.set_pending_only(true);
        ui.invoke_filter_changed();
        assert_eq!(ui.get_agenda().row_count(), 1);
        let sid = state.borrow().apps[0].stages[1].id.clone();
        ui.invoke_edit_stage(sid.clone().into());
        ui.set_stage_status(0);
        ui.invoke_save_form();
        assert_eq!(ui.get_agenda().row_count(), 0);
        ui.set_pending_only(false);
        ui.invoke_filter_changed();
        assert_eq!(ui.get_agenda().row_count(), 1);
        ui.invoke_edit_stage(sid.clone().into());
        ui.set_stage_status(3);
        ui.invoke_save_form();
        assert_eq!(
            state.borrow().store.load()?[0].stages[1].status,
            StageStatus::Postponed
        );
        assert_eq!(state.borrow().apps[0].state(), ApplicationState::Active);
        assert!(ui
            .get_agenda()
            .row_data(0)
            .unwrap()
            .title
            .contains("已推迟"));
        ui.invoke_edit_stage(sid.into());
        assert_eq!(ui.get_stage_status(), 3);
        ui.invoke_cancel_form();
        ui.set_calendar_query("不存在".into());
        ui.invoke_filter_changed();
        assert_eq!(ui.get_agenda().row_count(), 0);
        ui.set_calendar_query("".into());
        ui.invoke_filter_changed();
        ui.set_month("invalid".into());
        ui.invoke_month_select();
        assert_eq!(ui.get_month().as_str(), "2028-02");
        ui.invoke_month_move(1);
        assert_eq!(ui.get_month().as_str(), "2028-03");
        ui.invoke_calendar_today();
        assert_eq!(ui.get_selected_date().as_str(), today());
        ui.invoke_edit_application();
        ui.set_company("".into());
        ui.invoke_save_form();
        assert_eq!(ui.get_form_mode(), 1);
        assert_eq!(state.borrow().apps[0].company, "中文测试公司");
        ui.set_company("编辑后的公司".into());
        ui.invoke_save_form();
        assert_eq!(state.borrow().store.load()?[0].company, "编辑后的公司");
        let sid = state.borrow().apps[0].stages[1].id.clone();
        ui.invoke_delete_stage(sid.into());
        ui.invoke_confirm_delete();
        assert_eq!(state.borrow().store.load()?[0].stages.len(), 1);
        assert_eq!(ui.get_applications().row_count(), 0);
        assert_eq!(ui.get_stats().row_data(3).unwrap().count, 0);
        ui.invoke_select_date("2028-02-29".into());
        assert_eq!(ui.get_agenda().row_count(), 0);
        ui.invoke_delete_application();
        ui.invoke_confirm_delete();
        assert!(state.borrow().store.load()?.is_empty());
        assert_eq!(ui.get_visible_stats().row_count(), 0);
        #[cfg(feature = "click-stats")]
        {
            let counts: serde_json::Value =
                serde_json::from_slice(&std::fs::read(directory.join("click-stats.json"))?)?;
            assert_eq!(counts["counts"]["导航/日历"], 1);
            assert_eq!(counts["counts"]["招聘进度/筛选/进行中"], 1);
            assert_eq!(counts["counts"]["投递表单/保存"], 2);
            assert_eq!(counts["counts"]["进展表单/保存"], 5);
        }
        #[cfg(not(feature = "click-stats"))]
        assert!(!directory.join("click-stats.json").exists());
        println!("PASS: Chinese model, application/stage CRUD, required fields, persistence, pending filter, calendar refresh/navigation/search, stage statistics");
        slint::Timer::single_shot(std::time::Duration::from_millis(1200), || {
            let _ = slint::quit_event_loop();
        });
    }
    ui.show()?;
    let (api_server, api_receiver) = local_api::start(&directory)?;
    let api_timer = slint::Timer::default();
    {
        let weak = ui.as_weak();
        let state = state.clone();
        api_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(100),
            move || {
                while let Ok(incoming) = api_receiver.try_recv() {
                    let Some(ui) = weak.upgrade() else {
                        break;
                    };
                    let mut state = state.borrow_mut();
                    let mut next = state.apps.clone();
                    let result =
                        api::execute(&incoming.request, &mut next).and_then(|(data, changed)| {
                            if changed {
                                state.store.save(&next)?;
                                state.apps = next;
                                refresh(&ui, &state);
                            }
                            Ok(data)
                        });
                    let _ = incoming.reply.send(api::response(result));
                }
            },
        );
    }
    slint::run_event_loop()?;
    drop(api_timer);
    drop(api_server);
    drop(tray);
    Ok(())
}
