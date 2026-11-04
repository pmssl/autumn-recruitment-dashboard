# 原生查询投影与界面

**已确认。** `native/src/views.rs` 提供月历、阶段统计及当前/待进行进展摘要。日历按公司或岗位搜索、可只显示 pending 节点，排除无效日期并统计未定日期节点；月份范围为 1900–2100。统计按投递去重。证据：`calendar`、`parse_month`、`stage_counts`、`pending_stage_titles` 及 `calendar_filters_nodes_and_handles_invalid_dates`、`stats_deduplicate_stages_per_application`、`pending_titles_follow_flow_order_and_exclude_completed`。

**已确认。** `native/src/main.rs` 将领域数据投影为 Slint 模型，处理表单回调、保存、筛选、日历和托盘；`native/ui/app.slint` 定义招聘进度、全部记录、日历、统计和详情。`native/src/note_links.rs` 将 HTTP/HTTPS 备注地址转为可点击展示文本，不改变原文存储；测试 `links_are_added_without_reinterpreting_note_text` 覆盖转换边界。

**当前基线已确认**：招聘流程和日历详情的进展行随内容调整高度；建立基线时这是 `native/ui/app.slint`、`native/README.md` 相对 `f7d9e56` 的工作区改动，不代表现有发布包已包含。见 `decisions/0001-current-baseline.md`。

**当前源码已确认**：岗位详情进展正文在右侧预留 24px，使日期和操作按钮避开 `ListView` 的纵向滚动条；长备注仍按可用宽度换行，流程线仍随行高延伸。证据：`native/ui/app.slint`；2026-11-04 的 Debug 详情截图已检查有滚动条和长备注的布局。此修改随 v2.0.7 发布。

**未知**：跨平台真实中文输入、托盘和窗口行为仍未完成实机验收；不能把回调冒烟或截图视为这些体验的验证。证据：`native/README.md`、`docs/REWRITE_PLAN.md`。
