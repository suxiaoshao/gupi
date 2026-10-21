use super::*;
use gpui_kit::component::{Icon, kbd::Kbd};
use std::time::Instant;
const LAUNCHER_MAX_REM: f32 = 42.;
const TOOL_MIN_REM: f32 = 12.;

#[derive(Clone, PartialEq)]
pub(super) enum Destination {
    File(PathBuf),
    Changes(String),
}
#[derive(Clone)]
pub(super) struct Recent {
    destination: Destination,
    title: String,
    opened: Instant,
}
impl HomeView {
    fn remember(&mut self, destination: Destination, title: String) {
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            view.workspace
                .recent
                .retain(|entry| entry.destination != destination);
            view.workspace.recent.insert(
                0,
                Recent {
                    destination,
                    title,
                    opened: Instant::now(),
                },
            );
            view.workspace.recent.truncate(RECENT_LIMIT);
        }
    }
    pub(super) fn remember_file(&mut self, path: PathBuf) {
        let title = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        self.remember(Destination::File(path), title);
    }
    pub(super) fn remember_change(&mut self, id: String, title: String) {
        self.remember(Destination::Changes(id), title);
    }
    pub(super) fn new_workspace_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        view.workspace.next_new += 1;
        let id = format!("{NEW_TAB}:{}", view.workspace.next_new);
        self.select_workspace_tab(Some(id), window, cx);
        self.focus_panel(Panel::Workspace, window, cx);
    }
    fn latest_change(&self) -> Option<(String, String)> {
        let view = self.session_view()?;
        view.rows.iter().rev().find_map(|row| {
            view.edited_files
                .files(&row.id)
                .first()
                .map(|file| (row.id.clone(), file.path().to_owned()))
        })
    }
    fn recent_change(&self, id: &str) -> Option<(String, String)> {
        let view = self.session_view()?;
        let run = view.tools.owner(id)?;
        Some((
            run.to_owned(),
            view.edited_files.files(run).first()?.path().to_owned(),
        ))
    }
    pub(super) fn render_launcher(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let latest = self.latest_change();
        let owner_key = self.shown_key.clone();
        let tool = |icon, label: String| {
            h_flex()
                .w_full()
                .gap_2()
                .text_sm()
                .child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(div().flex_1().min_w_0().truncate().child(label))
        };
        let section = |label: String| {
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(cx.theme().muted_foreground)
                .child(label)
        };
        let files =
            tool(IconName::FolderTree, t(cx, "files-title")).children(Kbd::binding_for_action(
                &actions::Run::new(actions::Kind::ProjectFiles),
                Some("Gupi"),
                window,
            ));
        let no_changes = latest.is_none();
        let tools = h_flex()
            .w_full()
            .gap_2()
            .flex_wrap()
            .child(
                div().flex_1().min_w(rems(TOOL_MIN_REM)).child(
                    Button::new("launcher-files")
                        .debug_selector(|| "launcher-files".into())
                        .accessibility_label(t(cx, "files-title"))
                        .h_10()
                        .px_3()
                        .w_full()
                        .child(files)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.select_workspace_tab(None, window, cx);
                            this.focus_panel(Panel::Workspace, window, cx);
                        })),
                ),
            )
            .child(
                div().flex_1().min_w(rems(TOOL_MIN_REM)).child(
                    Button::new("launcher-changes")
                        .debug_selector(|| "launcher-changes".into())
                        .accessibility_label(t(cx, "workspace-changes"))
                        .h_10()
                        .px_3()
                        .w_full()
                        .disabled(no_changes)
                        .when(no_changes, |button| {
                            button.tooltip(t(cx, "workspace-no-changes"))
                        })
                        .child(tool(IconName::FileDiff, t(cx, "workspace-changes")))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let (Some(key), Some((run, path))) = (&owner_key, &latest) {
                                this.navigate_changes(key, run, path, window, cx);
                            }
                        })),
                ),
            );
        let mut page = v_flex()
            .w_full()
            .max_w(rems(LAUNCHER_MAX_REM))
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child(section(t(cx, "workspace-tools")))
                    .child(tools)
                    .when(no_changes, |column| {
                        column.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "workspace-no-changes")),
                        )
                    }),
            );
        if let Some(view) = self
            .session_view()
            .filter(|v| !v.workspace.recent.is_empty())
        {
            let recent = v_flex().gap_2().child(section(t(cx, "workspace-recent")));
            let mut rows = v_flex().gap_0p5();
            for entry in &view.workspace.recent {
                let row_id = match &entry.destination {
                    Destination::File(path) => format!("recent-file-{}", path.display()),
                    Destination::Changes(id) => format!("recent-changes-{id}"),
                };
                let is_file = matches!(entry.destination, Destination::File(_));
                let available = match &entry.destination {
                    Destination::File(_) => true,
                    Destination::Changes(id) => self.recent_change(id).is_some(),
                };
                let mut args = fluent_bundle::FluentArgs::new();
                let minutes = entry.opened.elapsed().as_secs() / 60;
                args.set("count", minutes as i64);
                let age = if minutes == 0 {
                    t(cx, "workspace-just-now")
                } else {
                    gupi_settings::i18n::t_with_args(cx, "workspace-minutes-ago", &args)
                };
                let destination = entry.destination.clone();
                let row = h_flex()
                    .w_full()
                    .gap_3()
                    .child(
                        div()
                            .flex_none()
                            .size_8()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().secondary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(if is_file {
                                    IconName::File
                                } else {
                                    IconName::FileDiff
                                })
                                .size_4()
                                .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .child(div().truncate().child(entry.title.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t(
                                        cx,
                                        if is_file {
                                            "files-title"
                                        } else {
                                            "workspace-changes"
                                        },
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(age),
                    );
                rows = rows.child(
                    Button::new(row_id)
                        .accessibility_label(entry.title.clone())
                        .ghost()
                        .h_auto()
                        .w_full()
                        .px_2()
                        .py_1p5()
                        .disabled(!available)
                        .tooltip(if available {
                            entry.title.clone()
                        } else {
                            t(cx, "workspace-recent-unavailable")
                        })
                        .child(row)
                        .on_click(cx.listener(move |this, _, window, cx| match &destination {
                            Destination::File(path) => {
                                this.open_source(path.clone(), true, window, cx)
                            }
                            Destination::Changes(id) => {
                                if let (Some(key), Some((run, path))) =
                                    (this.shown_key.clone(), this.recent_change(id))
                                {
                                    this.navigate_changes(&key, &run, &path, window, cx);
                                }
                            }
                        })),
                );
            }
            page = page.child(recent.child(rows));
        }
        div()
            .id("workspace-launcher")
            .size_full()
            .min_w_0()
            .overflow_y_scroll()
            .child(v_flex().px_5().py_5().child(page))
            .into_any_element()
    }
}
