//! Independent, scope-local drafts for models.json and mcp.json.
mod editor;
use editor::EntryEditor;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_resources::pi_config_files::{Entry, Error, File, Kind};
use gupi_resources::pi_settings::{Scope, ordered_eq};
use gupi_settings::{
    assets::IconName,
    i18n::{t, t_with_args},
};
use serde_json::Value;
use std::path::PathBuf;

type Continue = Box<dyn FnOnce(&mut Window, &mut App)>;
#[derive(Default)]
struct Draft {
    file: Option<File>,
    edits: Vec<(String, Option<Entry>)>,
    auto: Option<Option<Value>>,
    error: Option<Error>,
}
impl Draft {
    fn dirty(&self) -> bool {
        !self.edits.is_empty() || self.auto.is_some()
    }
    fn entry(&self, name: &str) -> Option<Entry> {
        self.edits
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, e)| e.clone())
            .unwrap_or_else(|| self.file.as_ref().and_then(|f| f.entry(name)))
    }
    fn names(&self) -> Result<Vec<String>, Error> {
        let mut names = self.file.as_ref().ok_or(Error::Read)?.names()?;
        for (name, entry) in &self.edits {
            if entry.is_none() {
                names.retain(|n| n != name)
            } else if !names.contains(name) {
                names.push(name.clone())
            }
        }
        Ok(names)
    }
    fn edit(&mut self, name: String, entry: Option<Entry>) {
        let baseline = self.file.as_ref().and_then(|f| f.entry(&name));
        let unchanged = match (&baseline, &entry) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same(b),
            _ => false,
        };
        if let Some(ix) = self.edits.iter().position(|(n, _)| n == &name) {
            if unchanged {
                self.edits.remove(ix);
            } else {
                self.edits[ix].1 = entry;
            }
        } else if !unchanged {
            self.edits.push((name, entry));
        }
    }
}
pub(super) struct FilePages {
    agent: Result<PathBuf, Error>,
    scope: Scope,
    models: Draft,
    mcp: Draft,
    global_mcp: Option<Result<File, Error>>,
    generation: u64,
    loading: Option<Task<()>>,
    saving: Option<Task<()>>,
    continuation: Option<Continue>,
}
impl FilePages {
    pub(super) fn new(_: &mut Context<Self>) -> Self {
        Self {
            agent: gupi_resources::pi_resources::agent_dir().map_err(|_| Error::Read),
            scope: Scope::Global,
            models: Draft::default(),
            mcp: Draft::default(),
            global_mcp: None,
            generation: 0,
            loading: None,
            saving: None,
            continuation: None,
        }
    }
    #[cfg(test)]
    pub(super) fn set_agent_for_test(&mut self, agent: PathBuf) {
        self.agent = Ok(agent);
    }
    fn draft(&self, kind: Kind) -> &Draft {
        match kind {
            Kind::Models => &self.models,
            Kind::Mcp => &self.mcp,
        }
    }
    fn draft_mut(&mut self, kind: Kind) -> &mut Draft {
        match kind {
            Kind::Models => &mut self.models,
            Kind::Mcp => &mut self.mcp,
        }
    }
    pub(super) fn dirty(&self) -> bool {
        self.models.dirty() || self.mcp.dirty()
    }
    pub(super) fn busy(&self) -> bool {
        self.saving.is_some()
    }
    pub(super) fn set_scope(&mut self, scope: Scope, cx: &mut Context<Self>) {
        self.scope = scope;
        self.models = Draft::default();
        self.mcp = Draft::default();
        self.loading = None;
        self.generation += 1;
        let _ = cx;
    }
    pub(super) fn activate(&mut self, cx: &mut Context<Self>) {
        if (self.models.file.is_some() || self.mcp.file.is_some()) && !self.dirty() && !self.busy()
        {
            self.reload(cx)
        }
    }
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Ok(agent) = self.agent.clone() else {
            return;
        };
        if self.busy() {
            return;
        }
        self.generation += 1;
        let generation = self.generation;
        let scope = self.scope.clone();
        let read = cx.background_spawn(async move {
            let models = File::load(Kind::Models, &Kind::Models.file(&agent, &Scope::Global));
            let mcp = File::load(Kind::Mcp, &Kind::Mcp.file(&agent, &scope));
            let global = if matches!(scope, Scope::Project(_)) {
                Some(File::load(
                    Kind::Mcp,
                    &Kind::Mcp.file(&agent, &Scope::Global),
                ))
            } else {
                None
            };
            (models, mcp, global)
        });
        self.loading = Some(cx.spawn(async move |this, cx| {
            let (models, mcp, global) = read.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.loading = None;
                for (kind, result) in [(Kind::Models, models), (Kind::Mcp, mcp)] {
                    let draft = this.draft_mut(kind);
                    match result {
                        Ok(file) => {
                            draft.file = Some(file);
                            draft.error = None
                        }
                        Err(e) => {
                            draft.file = None;
                            draft.error = Some(e)
                        }
                    }
                }
                this.global_mcp = global;
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn save(
        &mut self,
        kind: Option<Kind>,
        then: Option<Continue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        let Ok(agent) = self.agent.clone() else {
            return;
        };
        let scope = self.scope.clone();
        let mut requests = vec![];
        for k in [Kind::Models, Kind::Mcp] {
            let draft = self.draft(k);
            if kind.is_none_or(|kind| kind == k)
                && draft.dirty()
                && let Some(file) = &draft.file
            {
                requests.push((k, file.clone(), draft.edits.clone(), draft.auto.clone()));
            }
        }
        let task = cx.background_spawn(async move {
            requests
                .into_iter()
                .map(|(k, file, edits, auto)| {
                    let scope = if k == Kind::Models {
                        Scope::Global
                    } else {
                        scope.clone()
                    };
                    (k, file.commit(&agent, &scope, &edits, auto))
                })
                .collect::<Vec<_>>()
        });
        self.saving = Some(cx.spawn_in(window, async move |this, cx| {
            let results = task.await;
            let next = this.update(cx, |this, cx| {
                this.saving = None;
                let mut good = true;
                for (kind, result) in results {
                    let draft = this.draft_mut(kind);
                    match result {
                        Ok(file) => {
                            draft.file = Some(file);
                            draft.edits.clear();
                            draft.auto = None;
                            draft.error = None
                        }
                        Err(e) => {
                            draft.error = Some(e);
                            good = false
                        }
                    }
                }
                cx.notify();
                if good {
                    this.continuation.take().or(then)
                } else {
                    this.continuation = None;
                    None
                }
            });
            if let Ok(Some(next)) = next {
                let _ = cx.update(|window, cx| next(window, cx));
            }
        }));
        cx.notify();
    }
    pub(super) fn confirm_leave(
        entity: &Entity<Self>,
        proceed: Continue,
        window: &mut Window,
        cx: &mut App,
    ) {
        if entity.read(cx).busy() {
            entity.update(cx, |this, _| this.continuation = Some(proceed));
            return;
        }
        if !entity.read(cx).dirty() {
            proceed(window, cx);
            return;
        }
        let entity = entity.clone();
        let proceed = std::rc::Rc::new(std::cell::RefCell::new(Some(proceed)));
        window.open_dialog(cx, move |dialog, _, cx| {
            let discard = (entity.clone(), proceed.clone());
            let save = (entity.clone(), proceed.clone());
            dialog
                .title(t(cx, "pi-settings-unsaved-title"))
                .child(t(cx, "pi-settings-unsaved-body"))
                .footer(
                    gpui_kit::component::dialog::DialogFooter::new()
                        .child(
                            Button::new("file-cancel")
                                .label(t(cx, "action-cancel"))
                                .on_click(|_, w, cx| w.close_dialog(cx)),
                        )
                        .child(
                            Button::new("file-discard")
                                .label(t(cx, "pi-settings-discard"))
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    discard.0.update(cx, |this, cx| {
                                        this.models.edits.clear();
                                        this.models.auto = None;
                                        this.mcp.edits.clear();
                                        this.mcp.auto = None;
                                        cx.notify()
                                    });
                                    if let Some(next) = discard.1.borrow_mut().take() {
                                        next(w, cx)
                                    }
                                }),
                        )
                        .child(
                            Button::new("file-save")
                                .primary()
                                .label(t(cx, "pi-settings-save"))
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    let next = save.1.borrow_mut().take();
                                    save.0.update(cx, |this, cx| this.save(None, next, w, cx));
                                }),
                        ),
                )
        });
    }
    pub(super) fn page(
        entity: &Entity<Self>,
        kind: Kind,
        config: &Entity<super::pi_config::PiConfig>,
        cx: &App,
    ) -> SettingPage {
        let e = entity.clone();
        let h = entity.clone();
        let config = config.clone();
        let page = SettingPage::new(t(
            cx,
            if kind == Kind::Models {
                "pi-files-models"
            } else {
                "pi-files-mcp"
            },
        ))
        .icon(if kind == Kind::Models {
            IconName::Boxes
        } else {
            IconName::Server
        })
        .resettable(false)
        .title_suffix(move |_, cx| {
            let e = h.clone();
            let dirty = h.read(cx).draft(kind).dirty();
            let busy = h.read(cx).busy();
            h_flex()
                .gap_2()
                .child(config.update(cx, |this, cx| this.render_scope(cx)))
                .when(dirty, |row| {
                    row.child(
                        Button::new("file-reset")
                            .small()
                            .label(t(cx, "pi-settings-discard"))
                            .disabled(busy)
                            .on_click({
                                let e = e.clone();
                                move |_, _, cx| {
                                    e.update(cx, |this, cx| {
                                        let draft = this.draft_mut(kind);
                                        draft.edits.clear();
                                        draft.auto = None;
                                        cx.notify()
                                    })
                                }
                            }),
                    )
                    .child(
                        Button::new("file-save-page")
                            .small()
                            .primary()
                            .label(t(cx, "pi-settings-save"))
                            .disabled(busy)
                            .on_click({
                                let e = e.clone();
                                move |_, w, cx| {
                                    e.update(cx, |this, cx| this.save(Some(kind), None, w, cx))
                                }
                            }),
                    )
                })
                .child(
                    Button::new("file-reload")
                        .small()
                        .ghost()
                        .icon(IconName::RotateCw)
                        .accessibility_label(t(cx, "pi-settings-reload"))
                        .disabled(busy || dirty)
                        .on_click(move |_, _, cx| e.update(cx, |this, cx| this.reload(cx))),
                )
                .into_any_element()
        })
        .group(
            SettingGroup::new().item(SettingItem::render(move |_, window, cx| {
                e.update(cx, |this, cx| this.render_page(kind, window, cx))
            })),
        );
        if kind == Kind::Mcp {
            let e = entity.clone();
            page.group(
                SettingGroup::new()
                    .title(t(cx, "pi-files-automation"))
                    .item(SettingItem::render(move |_, _, cx| {
                        e.update(cx, |this, cx| this.render_auto(cx))
                    })),
            )
        } else {
            page
        }
    }

    fn render_page(&mut self, kind: Kind, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if kind == Kind::Models && matches!(self.scope, Scope::Project(_)) {
            return div()
                .text_sm()
                .child(t(cx, "pi-files-global-only"))
                .into_any_element();
        }
        if self.draft(kind).file.is_none()
            && self.draft(kind).error.is_none()
            && self.loading.is_none()
        {
            let entity = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = entity.update(cx, |this, cx| {
                    if this.loading.is_none() {
                        this.reload(cx)
                    }
                });
            });
        }
        let draft = self.draft(kind);
        let mut body = v_flex().w_full().max_w(rems(56.)).gap_2();
        let add = cx.entity();
        body = body.child(
            h_flex()
                .w_full()
                .items_start()
                .gap_2()
                .child(
                    v_flex()
                        .flex_1()
                        .gap_1()
                        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(t(
                            cx,
                            if kind == Kind::Models {
                                "pi-files-providers"
                            } else {
                                "pi-files-servers"
                            },
                        )))
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "pi-files-startup-only")),
                        ),
                )
                .child(
                    Button::new("file-add")
                        .small()
                        .label(t(cx, "pi-files-add"))
                        .disabled(self.busy() || self.loading.is_some() || draft.names().is_err())
                        .on_click(move |_, window, cx| {
                            Self::open_editor(&add, kind, None, window, cx)
                        }),
                ),
        );
        if self.loading.is_some() {
            return body.child(t(cx, "pi-settings-loading")).into_any_element();
        }
        if let Some(error) = draft.error {
            body = body.child(
                div()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error_key(error))),
            );
        }
        let Ok(names) = draft.names() else {
            return body
                .child(t(cx, "pi-resource-unsupported"))
                .into_any_element();
        };
        if draft.file.as_ref().is_some_and(File::comments) {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-files-comments")),
            );
        }
        let busy = self.busy();
        if names.is_empty() {
            body = body.child(
                div()
                    .py_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-files-empty")),
            );
        }
        for name in names {
            let Some(entry) = draft.entry(&name) else {
                continue;
            };
            let label = if kind == Kind::Mcp {
                if entry.partial() {
                    t(cx, "pi-files-override")
                } else {
                    entry
                        .visible(&["type".into()])
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_else(|| {
                            if entry.present(&["url".into()]) {
                                "http".into()
                            } else {
                                "stdio".into()
                            }
                        })
                }
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                match entry.len(&["models".into()]) {
                    Ok(count) => args.set("models", count as i64),
                    Err(_) => args.set("models", "—"),
                };
                match entry.keys(&["modelOverrides".into()]) {
                    Ok(keys) => args.set("overrides", keys.len() as i64),
                    Err(_) => args.set("overrides", "—"),
                };
                t_with_args(cx, "pi-files-provider-summary", &args)
            };
            let label = if entry.disabled() {
                format!("{label} · {}", t(cx, "pi-files-disabled"))
            } else {
                label
            };
            let e = cx.entity();
            let open = (e.clone(), name.clone());
            let row_name = name.clone();
            body = body.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new(SharedString::from(format!("file-row-{name}")))
                            .ghost()
                            .small()
                            .flex_1()
                            .min_w_0()
                            .accessibility_label(name.clone())
                            .disabled(busy)
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_left()
                                            .truncate()
                                            .child(name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(label),
                                    ),
                            )
                            .on_click(move |_, window, cx| {
                                Self::open_editor(&open.0, kind, Some(open.1.clone()), window, cx)
                            }),
                    )
                    .child(
                        Button::new(SharedString::from(format!("file-menu-{name}")))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Ellipsis)
                            .accessibility_label(t(cx, "pi-field-actions"))
                            .disabled(busy)
                            .dropdown_menu(move |menu, _, cx| {
                                let edit = (e.clone(), row_name.clone());
                                let delete = (e.clone(), row_name.clone());
                                menu.item(PopupMenuItem::new(t(cx, "pi-files-edit")).on_click(
                                    move |_, w, cx| {
                                        Self::open_editor(
                                            &edit.0,
                                            kind,
                                            Some(edit.1.clone()),
                                            w,
                                            cx,
                                        )
                                    },
                                ))
                                .item(
                                    PopupMenuItem::new(t(cx, "pi-files-delete")).on_click(
                                        move |_, _, cx| {
                                            delete.0.update(cx, |this, cx| {
                                                this.draft_mut(kind).edit(delete.1.clone(), None);
                                                cx.notify()
                                            })
                                        },
                                    ),
                                )
                            }),
                    ),
            );
        }
        body.into_any_element()
    }
    fn set_auto(&mut self, value: Option<Value>, cx: &mut Context<Self>) {
        let original = self.mcp.file.as_ref().and_then(File::auto_enable);
        self.mcp.auto = if ordered_eq(original.as_ref(), value.as_ref()) {
            None
        } else {
            Some(value)
        };
        cx.notify();
    }
    fn render_auto(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self
            .mcp
            .auto
            .clone()
            .unwrap_or_else(|| self.mcp.file.as_ref().and_then(File::auto_enable));
        let label = match current {
            None => t(cx, "pi-value-unset"),
            Some(Value::Bool(true)) => t(cx, "pi-files-on"),
            Some(Value::Bool(false)) => t(cx, "pi-files-off"),
            _ => t(cx, "pi-resource-unsupported"),
        };
        let e = cx.entity();
        let menu_e = e.clone();
        let pending = self.mcp.auto.is_some();
        let busy = self.busy() || self.mcp.file.is_none();
        h_flex()
            .w_full()
            .max_w(rems(56.))
            .gap_2()
            .items_center()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(editor::field_label("autoEnableCodemode", cx))
                    .when(pending, |label| {
                        label.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "pi-unsaved")),
                        )
                    }),
            )
            .child(
                Button::new("file-auto")
                    .small()
                    .outline()
                    .w_80()
                    .label(label)
                    .dropdown_caret(true)
                    .disabled(busy)
                    .accessibility_label(t(cx, "pi-files-label-autoEnableCodemode"))
                    .dropdown_menu(move |menu, _, cx| {
                        [
                            ("pi-value-unset", None),
                            ("pi-files-on", Some(Value::Bool(true))),
                            ("pi-files-off", Some(Value::Bool(false))),
                        ]
                        .into_iter()
                        .fold(menu, |menu, (label, value)| {
                            let e = e.clone();
                            menu.item(PopupMenuItem::new(t(cx, label)).on_click(move |_, _, cx| {
                                e.update(cx, |this, cx| this.set_auto(value.clone(), cx))
                            }))
                        })
                    }),
            )
            .child(
                Button::new("file-auto-menu")
                    .w_6()
                    .ghost()
                    .xsmall()
                    .icon(IconName::Ellipsis)
                    .accessibility_label(t(cx, "pi-field-actions"))
                    .disabled(busy)
                    .dropdown_menu(move |menu, _, cx| {
                        let e = menu_e.clone();
                        let clear = menu_e.clone();
                        menu.when(pending, |menu| {
                            menu.item(PopupMenuItem::new(t(cx, "pi-menu-revert")).on_click(
                                move |_, _, cx| {
                                    e.update(cx, |this, cx| {
                                        this.mcp.auto = None;
                                        cx.notify()
                                    })
                                },
                            ))
                        })
                        .item(
                            PopupMenuItem::new(t(cx, "pi-value-unset")).on_click(
                                move |_, _, cx| {
                                    clear.update(cx, |this, cx| this.set_auto(None, cx))
                                },
                            ),
                        )
                    }),
            )
            .into_any_element()
    }
    fn open_editor(
        entity: &Entity<Self>,
        kind: Kind,
        name: Option<String>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let this = entity.read(cx);
        let entry = name
            .as_deref()
            .and_then(|n| this.draft(kind).entry(n))
            .unwrap_or_else(|| Entry::new(kind));
        let project = matches!(this.scope, Scope::Project(_));
        let editing = name.is_some();
        let editor = cx.new(|cx| {
            EntryEditor::new(
                kind,
                name.unwrap_or_default(),
                entry,
                editing,
                project,
                window,
                cx,
            )
        });
        let parent = entity.clone();
        window.open_dialog(cx, move |dialog, window, cx| {
            let editor = editor.clone();
            let save = (parent.clone(), editor.clone());
            dialog
                .title(t(
                    cx,
                    if kind == Kind::Models {
                        "pi-files-models"
                    } else {
                        "pi-files-mcp"
                    },
                ))
                .width(window.rem_size() * 42.)
                .overlay_closable(false)
                .content(move |content, _, _| content.min_h_0().child(editor.clone()))
                .footer(super::dialog_buttons("pi-files-stage", false, false, cx))
                .on_ok(move |_, window, cx| {
                    let result = save.1.update(cx, |editor, cx| editor.prepare(cx));
                    let Ok((name, entry)) = result else {
                        return false;
                    };
                    let check = save.0.read(cx);
                    let result = if editing {
                        Ok(())
                    } else {
                        check.check_new(kind, &name, &entry)
                    };
                    if let Err(e) = result {
                        save.1.update(cx, |editor, cx| editor.show_error(e, cx));
                        return false;
                    }
                    save.0.update(cx, |this, cx| {
                        this.draft_mut(kind).edit(name, Some(entry));
                        cx.notify()
                    });
                    let _ = window;
                    true
                })
        });
    }
    fn check_new(&self, kind: Kind, name: &str, entry: &Entry) -> Result<(), Error> {
        let draft = self.draft(kind);
        let file = draft.file.as_ref().ok_or(Error::Read)?;
        let global = match &self.global_mcp {
            Some(Ok(file)) => Some(file),
            Some(Err(error)) => return Err(*error),
            None => None,
        };
        file.check_name(name, global)?;
        let names = draft.names()?;
        if names.iter().any(|n| {
            n == name || (kind == Kind::Mcp && n.replace('-', "_") == name.replace('-', "_"))
        }) {
            return Err(Error::Duplicate);
        }
        if kind == Kind::Mcp
            && matches!(self.scope, Scope::Project(_))
            && entry.partial()
            && !global.is_some_and(|f| f.entry(name).is_some())
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
fn error_key(error: Error) -> &'static str {
    match error {
        Error::Conflict => "pi-settings-conflict",
        Error::Duplicate => "pi-files-duplicate",
        _ => "pi-files-invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::{Draft, Entry, File, Kind};
    use serde_json::json;
    #[test]
    fn staged_edits_track_order_and_keep_unchanged_entries_out_of_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path,r#"{"mcpServers":{"a":{"command":"x","toolExposure":{"first":"direct","second":"hidden"}}}}"#).unwrap();
        let file = File::load(Kind::Mcp, &path).unwrap();
        let entry = file.entry("a").unwrap();
        let mut draft = Draft {
            file: Some(file),
            ..Default::default()
        };
        draft.edit("a".into(), Some(entry.clone()));
        assert!(!draft.dirty());
        let mut reordered = entry.clone();
        reordered.move_item(&["toolExposure".into()], 0, 1).unwrap();
        draft.edit("a".into(), Some(reordered));
        assert!(draft.dirty());
        draft.edit("a".into(), Some(entry));
        assert!(!draft.dirty());
        let mut new = Entry::new(Kind::Mcp);
        new.set(&["command".into()], Some(json!("new"))).unwrap();
        draft.edit("b".into(), Some(new));
        assert_eq!(draft.names().unwrap(), ["a", "b"]);
        draft.edit("b".into(), None);
        assert_eq!(draft.names().unwrap(), ["a"]);
        assert!(!draft.dirty());
    }
    struct Mounted {
        files: gpui_kit::Entity<super::FilePages>,
        kind: Kind,
    }
    impl gpui_kit::Render for Mounted {
        fn render(
            &mut self,
            window: &mut gpui_kit::Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            self.files
                .update(cx, |files, cx| files.render_page(self.kind, window, cx))
        }
    }
    #[gpui_kit::test]
    fn add_entry_button_opens_both_editors_without_reentering_owner(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::AppContext as _;
        use gpui_kit::component::WindowExt as _;
        use gpui_kit::test::TestWindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let agent = tempfile::tempdir().unwrap();
        for kind in [Kind::Models, Kind::Mcp] {
            let (_, cx) = cx.add_window_view(|window, cx| {
                let files = cx.new(|cx| {
                    let mut files = super::FilePages::new(cx);
                    files.agent = Ok(agent.path().to_owned());
                    files.draft_mut(kind).file = Some(
                        File::load(
                            kind,
                            &kind.file(agent.path(), &gupi_resources::pi_settings::Scope::Global),
                        )
                        .unwrap(),
                    );
                    files
                });
                let mounted = cx.new(|_| Mounted { files, kind });
                gpui_kit::component::Root::new(mounted, window, cx)
            });
            cx.update(|window, cx| {
                window.render_frame(cx);
                window.click("file-add", cx);
                window.render_frame(cx);
                assert!(window.has_active_dialog(cx));
            });
        }
    }
}
