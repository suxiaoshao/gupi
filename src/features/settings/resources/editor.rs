use super::*;
use gpui_kit::component::dialog::Cancel;
use gpui_kit::component::dialog::Confirm;
use gpui_kit::component::dialog::DialogButtonProps;

impl ResourcesView {
    fn show_editor_dialog(&self, window: &mut Window, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, window, cx| {
            let save = owner.clone();
            let cancel = owner.clone();
            let close = owner.clone();
            let Some(entity) = owner.upgrade() else {
                return dialog;
            };
            entity.update(cx, |this, cx| {
                let Some(editor) = &this.editor else {
                    return dialog;
                };
                let busy = this.busy(cx);
                let dirty = editor.form.read(cx).is_dirty();
                let saving = editor.owner.read(cx).mutation().is_running();
                let changed = editor.changed;
                let reopen = Open {
                    path: editor.path.clone(),
                    kind: editor.kind,
                    editable: editor.editable,
                    create: false,
                    owner: editor.owner.clone(),
                };
                let reload = owner.clone();
                let path = editor.path.display().to_string();
                dialog
                    .title(
                        editor
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    )
                    .width(px(720.).min(window.viewport_size().width - px(32.)))
                    .overlay_closable(false)
                    .close_button(!busy)
                    .child(
                        v_flex()
                            .gap_3()
                            .debug_selector(|| "resource-editor-dialog".into())
                            .child(
                                div()
                                    .id("editor-path")
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .truncate()
                                    .child(path.clone())
                                    .tooltip(move |window, cx| {
                                        gpui_kit::component::tooltip::Tooltip::new(path.clone())
                                            .build(window, cx)
                                    }),
                            )
                            .when(changed, |content| {
                                content.child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_sm()
                                                .text_color(cx.theme().danger)
                                                .child(t(cx, "settings-editor-changed")),
                                        )
                                        .child(
                                            Button::new("resource-editor-reload")
                                                .small()
                                                .label(t(cx, "settings-editor-reload"))
                                                .disabled(busy)
                                                .on_click(move |_, window, cx| {
                                                    // Drops this draft for the file's current text.
                                                    let _ = reload.update(cx, |this, cx| {
                                                        this.editor = None;
                                                        window.close_dialog(cx);
                                                        this.request_open(
                                                            reopen.clone(),
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        ),
                                )
                            })
                            .child(
                                CodeEditor::new(&editor.input)
                                    .h(px(400.).min(window.viewport_size().height * 0.5))
                                    .readonly(!editor.editable)
                                    .disabled(busy),
                            ),
                    )
                    .footer(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("resource-editor-cancel")
                                    .label(t(cx, "action-cancel"))
                                    .disabled(busy)
                                    .debug_selector(|| "resource-editor-cancel".into())
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(Cancel), cx)
                                    }),
                            )
                            .child(
                                Button::new("resource-editor-save")
                                    .primary()
                                    .label(t(cx, "settings-resource-save"))
                                    .loading(saving)
                                    .disabled(
                                        busy || !editor.editable || (!editor.create && !dirty),
                                    )
                                    .debug_selector(|| "resource-editor-save".into())
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(
                                            Box::new(Confirm { secondary: false }),
                                            cx,
                                        )
                                    }),
                            ),
                    )
                    .on_ok(move |_, _, cx| {
                        let _ = save.update(cx, |this, cx| this.save_editor(cx));
                        // The successful write closes the dialog; an error leaves the draft visible.
                        false
                    })
                    .on_cancel(move |_, window, cx| {
                        cancel
                            .update(cx, |this, cx| this.cancel_editor(window, cx))
                            .unwrap_or(true)
                    })
                    .on_close(move |_, _, cx| {
                        let _ = close.update(cx, |this, cx| {
                            this.editor = None;
                            cx.notify();
                        });
                    })
            })
        });
    }

    fn save_editor(&mut self, cx: &mut Context<Self>) {
        if self.busy(cx) {
            return;
        }
        let Some(editor) = &self.editor else { return };
        if !editor.editable || (!editor.create && !editor.form.read(cx).is_dirty()) {
            return;
        }
        let path = editor.path.clone();
        let create = editor.create;
        let baseline = editor.baseline.clone();
        let owner = editor.owner.clone();
        if let Ok(prepared) = editor.form.update(cx, |form, cx| form.prepare(cx)) {
            self.change_in(
                &owner,
                Change::Save {
                    path,
                    text: prepared.value().text.clone(),
                    create,
                    baseline,
                },
                cx,
            );
        }
    }

    /// Saves the editor, then runs `proceed` once the write succeeds.
    pub(super) fn save_then(&mut self, proceed: Proceed, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else { return };
        let (path, owner) = (editor.path.clone(), editor.owner.clone());
        self.after_save = Some((path, proceed));
        self.save_editor(cx);
        // Nothing was written (busy or invalid): the user stays on the editor.
        if !owner.read(cx).mutation().is_running() {
            self.after_save = None;
        }
    }

    fn cancel_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.busy(cx) {
            return false;
        }
        if !self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.form.read(cx).is_dirty())
        {
            return true;
        }
        let owner = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let owner = owner.clone();
            dialog
                .title(t(cx, "settings-editor-discard"))
                .button_props(DialogButtonProps::default().show_cancel(true))
                .on_ok(move |_, window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        this.editor = None;
                        cx.notify();
                    });
                    // Pop the confirmation, then let its normal close remove the editor beneath it.
                    window.close_dialog(cx);
                    true
                })
        });
        false
    }

    pub(super) fn request_open(&mut self, next: Open, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy(cx) || self.open.is_running() || self.editor.is_some() {
            return;
        }
        self.open_generation += 1;
        let generation = self.open_generation;
        self.creating = None;
        self.error = None;
        if next.create {
            self.open = refresh::Operation::new();
            let text = if next.kind == Kind::Skill {
                format!(
                    "---\nname: {}\ndescription: \n---\n\n",
                    next.path
                        .parent()
                        .and_then(|p| p.file_name())
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            } else {
                String::new()
            };
            self.editor = Some(Self::editor(next, text, Baseline::Missing, window, cx));
            self.show_editor_dialog(window, cx);
            cx.notify();
            return;
        }
        let worker = cx.background_spawn({
            let path = next.path.clone();
            async move {
                match std::fs::read_to_string(&path) {
                    Ok(text) => Ok(Baseline::Text(text)),
                    Err(e)
                        if e.kind() == std::io::ErrorKind::NotFound
                            && path
                                .file_name()
                                .is_some_and(|n| n == "SYSTEM.md" || n == "APPEND_SYSTEM.md") =>
                    {
                        Ok(Baseline::Missing)
                    }
                    Err(e) => Err(io::Error::from(e)),
                }
            }
        });
        let task = cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.finish_open(generation, next, result, window, cx);
            });
        });
        match &self.open {
            refresh::Operation::Idle(_) => self.open.transition(Load(task)),
            refresh::Operation::Unavailable(_) => self.open.transition(Retry(task)),
            _ => self.open.transition(Refresh(task)),
        }
        cx.notify();
    }

    fn finish_open(
        &mut self,
        generation: u64,
        next: Open,
        result: Result<Baseline, io::Error>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_generation != generation {
            return;
        }
        match result {
            Ok(baseline) => {
                let text = match &baseline {
                    Baseline::Text(text) => text.clone(),
                    Baseline::Missing => String::new(),
                };
                self.editor = Some(Self::editor(next, text, baseline, window, cx));
                self.open.transition(Complete(Ok(())));
                self.show_editor_dialog(window, cx);
            }
            Err(error) => {
                self.open.transition(Complete(Err(error)));
            }
        }
        cx.notify();
    }

    fn editor(
        next: Open,
        text: String,
        baseline: Baseline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Editor {
        let form = cx.new(|_| Form::new(TextDraft { text: text.clone() }));
        let input = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .default_value(text)
        });
        let (binding, writer) = TextDraft::TEXT.bind_control_in(
            &form,
            &input,
            |input, projection, window, cx| {
                if let ControlProjection::Value(text) = projection {
                    input.set_value(text, window, cx);
                }
            },
            window,
            cx,
        );
        let input_sub = cx.subscribe_in(&input, window, move |_, input, event, window, cx| {
            if matches!(event, InputEvent::Change) {
                writer.defer_set(input.read(cx).value().to_string(), window, cx);
            }
        });
        let form_sub = cx.observe(&form, |_, _, cx| cx.notify());
        Editor {
            path: next.path,
            kind: next.kind,
            editable: next.editable,
            create: next.create,
            owner: next.owner,
            baseline,
            changed: false,
            form,
            input,
            _binding: binding,
            _subscriptions: vec![input_sub, form_sub],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Baseline, ConfigController, Form, Kind, Open, PiProbeController, Scope};
    use crate::features::settings::SettingsView;
    use gpui_kit::component::Root;
    use gpui_kit::{AppContext as _, TestAppContext};

    #[gpui_kit::test]
    fn scope_changes_cancel_pending_editors_and_reject_old_results(cx: &mut TestAppContext) {
        // Project catalog reads use Tokio blocking workers outside the test executor.
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_tokio::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let agent = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let path = project.path().join(".pi/prompts/example.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "Project text").unwrap();
        let mut settings = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(gupi_settings::config::AppConfig::default()));
            let controller = cx.new(|cx| ConfigController::new(&form, cx));
            let draft = cx.new(|_| PiProbeController::new());
            let applied = cx.new(|_| PiProbeController::new());
            let view = cx.new(|cx| {
                SettingsView::new(
                    form,
                    controller,
                    draft,
                    applied,
                    cx.focus_handle(),
                    window,
                    cx,
                )
            });
            settings = Some(view.clone());
            Root::new(view, window, cx)
        });
        let (resources, config) = settings.unwrap().read_with(cx, |view, _| {
            (view.resources.clone(), view.pi_config.clone())
        });
        cx.update(|window, cx| {
            config.update(cx, |config, _| {
                config.set_agent_for_test(agent.path().to_owned())
            });
            resources.update(cx, |view, cx| {
                let scope = Scope::Project(project.path().to_owned());
                config.update(cx, |config, cx| config.set_scope(scope.clone(), cx));
                view.sync_scope(window, cx);
                let owner = view.active().clone();
                view.controller
                    .update(cx, |controller, _| controller.stop());
                owner.update(cx, |controller, _| controller.stop());
                let request = || Open {
                    path: path.clone(),
                    kind: Kind::Prompt,
                    editable: true,
                    create: false,
                    owner: owner.clone(),
                };
                view.request_open(request(), window, cx);
                assert!(view.open.is_running());
                let generation = view.open_generation;
                config.update(cx, |config, cx| config.set_scope(Scope::Global, cx));
                view.sync_scope(window, cx);
                assert!(!view.open.is_running());
                config.update(cx, |config, cx| config.set_scope(scope, cx));
                view.sync_scope(window, cx);
                view.finish_open(
                    generation,
                    request(),
                    Ok(Baseline::Text("Late text".into())),
                    window,
                    cx,
                );
                assert!(view.editor.is_none());
                assert!(!view.open.is_running());
            });
        });
        cx.run_until_parked();
        resources.read_with(cx, |view, _| assert!(view.editor.is_none()));
    }
    #[gpui_kit::test]
    async fn leaving_during_resource_save_waits_for_success_or_keeps_the_editor(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_tokio::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        for fail in [false, true] {
            let agent = tempfile::tempdir().unwrap();
            let path = agent.path().join("prompts/new.md");
            if fail {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, "external").unwrap();
            }
            let mut settings = None;
            let (_, cx) = cx.add_window_view(|window, cx| {
                let form = cx.new(|_| Form::new(gupi_settings::config::AppConfig::default()));
                let controller = cx.new(|cx| ConfigController::new(&form, cx));
                let draft = cx.new(|_| PiProbeController::new());
                let applied = cx.new(|_| PiProbeController::new());
                let view = cx.new(|cx| {
                    SettingsView::new(
                        form,
                        controller,
                        draft,
                        applied,
                        cx.focus_handle(),
                        window,
                        cx,
                    )
                });
                settings = Some(view.clone());
                Root::new(view, window, cx)
            });
            let resources = settings
                .unwrap()
                .read_with(cx, |view, _| view.resources.clone());
            let left = std::rc::Rc::new(std::cell::Cell::new(false));
            cx.update(|window, cx| {
                resources.update(cx, |view, cx| {
                    view.controller.update(cx, |controller, _| {
                        let mut catalog = gupi_resources::pi_resources::Catalog::default();
                        catalog.root = agent.path().to_owned();
                        controller.set_catalog_for_test(catalog);
                    });
                    let next = Open {
                        path: path.clone(),
                        kind: Kind::Prompt,
                        editable: true,
                        create: true,
                        owner: view.controller.clone(),
                    };
                    view.editor = Some(super::ResourcesView::editor(
                        next,
                        "saved text".into(),
                        Baseline::Missing,
                        window,
                        cx,
                    ));
                    view.save_editor(cx);
                    assert!(view.controller.read(cx).mutation().is_running());
                    assert!(view.needs_leave_guard(cx));
                });
                let left = left.clone();
                super::ResourcesView::confirm_leave(
                    &resources,
                    move |_, _| left.set(true),
                    window,
                    cx,
                );
                assert!(resources.read(cx).after_save.is_some());
            });
            assert!(!left.get());
            cx.condition(&resources, |view, cx| {
                !view.controller.read(cx).mutation().is_running() && view.after_save.is_none()
            })
            .await;
            cx.run_until_parked();
            assert_eq!(left.get(), !fail);
            resources.read_with(cx, |view, _| assert_eq!(view.editor.is_some(), fail));
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                if fail { "external" } else { "saved text" }
            );
        }
    }
}
