use super::Field;
use super::Load;
use super::Page;
use super::PiConfig;
use super::Support;
use super::Trust;
use super::fields;
use super::forms::Slot;
use super::model_scope;
use super::supports;
use gpui_kit::AppContext as _;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::Render;
use gpui_kit::TestAppContext;
use gpui_kit::Window;
use gpui_kit::div;
use gpui_operation::Settle;
use gpui_operation::Transition as _;
use gupi_pi_runtime::PiProbeController;
use gupi_resources::pi_settings::Scope;
use gupi_resources::pi_settings::trust;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::Path;

impl crate::features::settings::SettingsView {
    pub(crate) fn seed_quit_draft_for_test(&self, saving: bool, cx: &mut gpui_kit::App) {
        self.pi_config.update(cx, |config, cx| {
            super::forms::ConversationDraft::STEERING.set(
                &config.conversation,
                Slot::Set("all".into()),
                cx,
            );
            if saving {
                config.saving = Some(gpui_kit::Task::ready(()));
            }
        });
    }

    pub(crate) fn quit_waits_for_save_for_test(&self, cx: &gpui_kit::App) -> bool {
        self.pi_config.read(cx).leave_after_save.is_some()
    }
}

#[test]
fn version_gate_accepts_1_1_and_later() {
    for version in ["1.1.0", "v1.1.3", "1.2.0-beta.1", "2.0.0"] {
        assert!(supports(version), "{version}");
    }
    for version in ["1.0.9", "0.9.0", "", "unknown", "1"] {
        assert!(!supports(version), "{version}");
    }
}

/// Keeps the config entity alive for the window's lifetime.
struct Host {
    _config: Entity<PiConfig>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A config entity over a temporary agent directory and a Pi 1.1.0 probe.
fn setup<'a>(
    cx: &'a mut TestAppContext,
    agent: &Path,
) -> (Entity<PiConfig>, &'a mut gpui_kit::VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let agent = agent.to_owned();
    let mut config = None;
    let (_, window) = cx.add_window_view(|window, cx| {
        let probe = cx.new(|_| {
            let mut probe = PiProbeController::new();
            probe
                .operation_mut_for_test()
                .transition(Settle(Ok(pi_rpc::probe::PiProbeData {
                    command: "/missing/pi".into(),
                    version: "1.1.0".into(),
                })));
            probe
        });
        let entity = cx.new(|cx| {
            let mut this = PiConfig::new(probe, window, cx);
            this.set_agent_for_test(agent, cx);
            this.activate(cx);
            this
        });
        config = Some(entity.clone());
        Host { _config: entity }
    });
    window.run_until_parked();
    (config.unwrap(), window)
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn save(config: &Entity<PiConfig>, page: Page, window: &mut gpui_kit::VisualTestContext) {
    window
        .update(|window, cx| config.update(cx, |this, cx| this.save(Some(page), None, window, cx)));
    window.run_until_parked();
}

fn input_text(
    config: &Entity<PiConfig>,
    field: Field,
    text: &str,
    visual: &mut gpui_kit::VisualTestContext,
) {
    visual.update(|window, cx| {
        let input = config.update(cx, |this, cx| this.input(field, window, cx));
        input.update(cx, |input, cx| {
            input.set_value(text.to_owned(), window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        });
    });
    visual.run_until_parked();
}

fn append(config: &Entity<PiConfig>, text: &str, visual: &mut gpui_kit::VisualTestContext) {
    visual.update(|window, cx| {
        config.update(cx, |this, cx| {
            this.append_pattern(text.to_owned(), window, cx)
        })
    });
    visual.run_until_parked();
}

fn remove_all(config: &Entity<PiConfig>, visual: &mut gpui_kit::VisualTestContext) {
    visual.update(|window, cx| {
        config.update(cx, |this, cx| {
            while let Some(key) = this.row_keys(cx).into_iter().next() {
                this.remove_pattern(&key, window, cx);
            }
        })
    });
    visual.run_until_parked();
}

#[gpui_kit::test]
fn page_save_writes_only_that_page_and_keeps_other_drafts(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let global = agent.path().join("settings.json");
    fs::write(
        &global,
        r#"{"theme":"dark","compaction":{"enabled":true,"custom":1}}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        assert_eq!(this.support(cx), Support::Ready);
        this.set_values(Field::Compaction, vec![Slot::Set(json!(false))], cx);
        this.set_values(Field::MaxRetries, vec![Slot::Set(json!(5))], cx);
    });
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&global),
        json!({"theme":"dark","compaction":{"enabled":false,"custom":1}})
    );
    config.update(window, |this, cx| {
        assert_eq!(
            Field::ALL
                .into_iter()
                .filter(|f| this.pending(*f, cx))
                .collect::<Vec<_>>(),
            [Field::MaxRetries]
        );
        assert!(this.error.is_none());
    });
}

#[gpui_kit::test]
fn invalid_drafts_block_saving_and_writing_back_the_stored_value_clears_a_draft(
    cx: &mut TestAppContext,
) {
    let agent = tempfile::tempdir().unwrap();
    let global = agent.path().join("settings.json");
    fs::write(&global, r#"{"retry":{"maxRetries":2}}"#).unwrap();
    let (config, window) = setup(cx, agent.path());
    input_text(&config, Field::MaxRetries, "-1", window);
    config.update(window, |this, cx| {
        assert!(matches!(
            this.snapshot(cx).value(Field::MaxRetries).first(),
            Some(Slot::Invalid(_))
        ));
    });
    save(&config, Page::Network, window);
    config.update(window, |this, _| {
        assert!(
            this.error.is_some(),
            "invalid text is reported, not written"
        );
    });
    input_text(&config, Field::MaxRetries, "2", window);
    config.update(window, |this, cx| {
        assert!(
            !this.has_unsaved(cx),
            "typing the stored value is no change"
        );
    });
    assert_eq!(read(&global), json!({"retry":{"maxRetries":2}}));
}

#[gpui_kit::test]
fn same_field_external_change_is_a_conflict_that_keeps_the_draft(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let global = agent.path().join("settings.json");
    fs::write(&global, r#"{"steeringMode":"all"}"#).unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_values(Field::Steering, vec![Slot::Set(json!("one-at-a-time"))], cx);
    });
    fs::write(&global, r#"{"steeringMode":"all","followUpMode":"all"}"#).unwrap();
    // A different leaf changed: the save still succeeds and keeps it.
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&global),
        json!({"steeringMode":"one-at-a-time","followUpMode":"all"})
    );

    config.update(window, |this, cx| {
        this.set_values(Field::Steering, vec![Slot::Set(json!("all"))], cx);
    });
    fs::write(&global, r#"{"followUpMode":"all"}"#).unwrap();
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&global),
        json!({"followUpMode":"all"}),
        "nothing overwritten"
    );
    config.update(window, |this, cx| {
        assert_eq!(this.conflicts, [Field::Steering]);
        assert!(this.pending(Field::Steering, cx), "the draft survives");
    });
}

#[gpui_kit::test]
fn project_scope_writes_the_project_file_and_reports_trust(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let global = agent.path().join("settings.json");
    fs::write(
        &global,
        r#"{"compaction":{"enabled":false},"httpProxy":"http://p"}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_scope(Scope::Project(project.path().to_owned()), cx)
    });
    window.run_until_parked();
    config.update(window, |this, cx| {
        assert_eq!(this.trust, Some(Trust::Untrusted));
        let Load::Ready(files) = &this.files else {
            panic!("loaded")
        };
        let inherited = files.resolve(Field::Compaction, None);
        assert_eq!(inherited.source, fields::Source::Global);
        // Pinning an inherited value writes it explicitly for the project.
        this.pin(Field::Compaction, cx);
    });
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&project.path().join(".pi/settings.json")),
        json!({"compaction":{"enabled":false}})
    );
    assert_eq!(
        read(&global),
        json!({"compaction":{"enabled":false},"httpProxy":"http://p"}),
        "the global file is untouched"
    );
    assert!(
        !trust::file(agent.path()).exists(),
        "saving grants no trust"
    );

    window.update(|window, cx| config.update(cx, |this, cx| this.trust(window, cx)));
    window.run_until_parked();
    config.update(window, |this, _| {
        assert_eq!(this.trust, Some(Trust::Trusted))
    });
    let records = read(&trust::file(agent.path()));
    let key = trust::canonical(project.path())
        .to_string_lossy()
        .into_owned();
    assert_eq!(records, json!({ key: true }));
}

#[gpui_kit::test]
fn discarding_restores_the_files_as_read(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_values(Field::Proxy, vec![Slot::Set(json!("http://x"))], cx);
        this.set_values(Field::Compaction, vec![Slot::Set(json!(false))], cx);
        this.discard(Some(Page::Network), cx);
        assert_eq!(
            Field::ALL
                .into_iter()
                .filter(|f| this.pending(*f, cx))
                .collect::<Vec<_>>(),
            [Field::Compaction]
        );
        this.discard(None, cx);
        assert!(!this.has_unsaved(cx));
    });
    assert!(!agent.path().join("settings.json").exists());
}

struct PageHost(Entity<PiConfig>, Page);
struct Outer(Entity<PageHost>);
impl Render for Outer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.0.clone()
    }
}
impl Render for PageHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::component::setting::Settings::new("pi-test")
            .page(PiConfig::page(&self.0, self.1, cx))
    }
}

#[gpui_kit::test]
fn all_three_pages_render_fields_in_each_scope(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"defaultProvider":"p","defaultModel":"m","defaultThinkingLevel":"xhigh","httpProxy":"http://p","enabledModels":["p/m","*sonnet*:high"]}"#,
    )
    .unwrap();
    let (config, _) = setup(cx, agent.path());
    // Rendering would otherwise start a model query with a real Pi process.
    config.update(cx, |this, _| {
        this.models = super::Models::Failed("offline".into())
    });
    // One window, like the settings screen: the model picker is shared state.
    let mut host = None;
    let (_, window) = cx.add_window_view(|_, cx| {
        let view = cx.new(|_| PageHost(config.clone(), Page::Conversation));
        host = Some(view.clone());
        Outer(view)
    });
    let host = host.unwrap();
    for scope in [Scope::Global, Scope::Project(project.path().to_owned())] {
        config.update(window, |this, cx| {
            this.set_scope(scope.clone(), cx);
            this.models = super::Models::Failed("offline".into());
        });
        window.run_until_parked();
        for page in [Page::Conversation, Page::Network, Page::Tools] {
            host.update(window, |host, cx| {
                host.1 = page;
                cx.notify();
            });
            window.simulate_resize(gpui_kit::size(gpui_kit::px(480.), gpui_kit::px(900.)));
            window.run_until_parked();
        }
        host.update(window, |host, cx| {
            host.1 = Page::Conversation;
            cx.notify();
        });
        window.update(|window, cx| {
            config.update(cx, |this, cx| {
                let key = this.row_keys(cx)[1].clone();
                this.move_pattern(&key, false, window, cx);
            })
        });
        window.run_until_parked();
        config.update(window, |this, cx| {
            assert_eq!(
                this.snapshot(cx).value(Field::ModelScope),
                vec![Slot::Set(json!(["*sonnet*:high", "p/m"]))]
            );
            this.discard(None, cx);
        });
        window.run_until_parked();
        config.update(window, |this, cx| {
            assert!(
                !this.has_unsaved(cx),
                "input synchronization must not pin inherited values"
            );
        });
    }
    config.update(cx, |this, cx| {
        assert!(matches!(this.files, Load::Ready(_)));
        assert!(!this.has_unsaved(cx), "rendering never creates drafts");
    });
}

#[gpui_kit::test]
async fn reload_keeps_the_model_query_and_its_cleanup(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_tokio::init);
    cx.update(gupi_pi_runtime::init);
    let agent = tempfile::tempdir().unwrap();
    let (config, cx) = setup(cx, agent.path());
    let id = cx.update(|window, cx| {
        config.update(cx, |this, cx| {
            this.load_models(window, cx);
            let id = this.model_instance.unwrap();
            this.reload(cx);
            assert_eq!(this.model_instance, Some(id));
            assert!(this.model_task.is_some());
            id
        })
    });
    // The missing executable fails the real query, which must still settle
    // and remove its temporary runtime instance after the settings reload.
    cx.condition(&config, |this, _| {
        matches!(this.models, super::Models::Failed(_))
    })
    .await;
    cx.update(|_, cx| {
        assert!(config.read(cx).model_instance.is_none());
        assert!(matches!(
            gupi_pi_runtime::global(cx).read(cx).client(id),
            Err(pi_rpc::Error::Closed)
        ));
    });
}

#[gpui_kit::test]
fn changing_scope_closes_the_previous_model_query(cx: &mut TestAppContext) {
    cx.update(gpui_tokio::init);
    cx.update(gupi_pi_runtime::init);
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let (config, cx) = setup(cx, agent.path());
    cx.update(|window, cx| {
        config.update(cx, |this, cx| {
            this.load_models(window, cx);
            let id = this.model_instance.unwrap();
            this.set_scope(Scope::Project(project.path().to_owned()), cx);
            assert!(this.model_instance.is_none());
            assert!(this.model_task.is_none());
            assert!(matches!(this.models, super::Models::Idle));
            assert!(matches!(
                gupi_pi_runtime::global(cx).read(cx).client(id),
                Err(pi_rpc::Error::Closed)
            ));
        });
    });
    cx.run_until_parked();
    config.read_with(cx, |this, _| {
        assert!(matches!(this.models, super::Models::Idle))
    });
}

#[gpui_kit::test]
fn invalid_project_scope_preserves_the_current_scope_and_drafts(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let (config, cx) = setup(cx, agent.path());
    config.update(cx, |this, cx| {
        this.set_values(Field::Steering, vec![Slot::Set(json!("all"))], cx);
        for path in [
            agent.path().join("missing"),
            std::path::PathBuf::from("relative"),
        ] {
            this.set_scope(Scope::Project(path), cx);
            assert_eq!(this.scope, Scope::Global);
            assert!(this.pending(Field::Steering, cx));
            assert!(this.error.is_some());
        }
    });
}

#[gpui_kit::test]
fn saving_a_deleted_project_keeps_the_draft_without_recreating_the_folder(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = agent.path().join("project");
    fs::create_dir(&project).unwrap();
    let (config, cx) = setup(cx, agent.path());
    config.update(cx, |this, cx| {
        this.set_scope(Scope::Project(project.clone()), cx)
    });
    cx.run_until_parked();
    config.update(cx, |this, cx| {
        this.set_values(Field::Steering, vec![Slot::Set(json!("all"))], cx)
    });
    fs::remove_dir(&project).unwrap();
    save(&config, Page::Conversation, cx);
    assert!(!project.exists());
    config.read_with(cx, |this, cx| {
        assert!(this.pending(Field::Steering, cx));
        assert!(this.error.is_some());
    });
}

#[gpui_kit::test]
fn leaving_and_reloading_during_save_wait_for_the_result(cx: &mut TestAppContext) {
    for fail in [false, true] {
        let agent = tempfile::tempdir().unwrap();
        let (config, cx) = setup(cx, agent.path());
        let left = std::rc::Rc::new(std::cell::Cell::new(false));
        cx.update(|window, cx| {
            config.update(cx, |this, cx| {
                this.set_values(Field::Steering, vec![Slot::Set(json!("all"))], cx);
                if fail {
                    fs::write(agent.path().join("settings.json"), "invalid JSON").unwrap();
                }
                this.save(Some(Page::Conversation), None, window, cx);
                assert!(this.is_saving());
                let generation = this.generation;
                this.reload(cx);
                this.discard(None, cx);
                assert_eq!(this.generation, generation);
                assert!(this.has_unsaved(cx));
            });
            let left = left.clone();
            PiConfig::confirm_leave(&config, move |_, _| left.set(true), window, cx);
            assert!(config.read(cx).leave_after_save.is_some());
        });
        assert!(!left.get());
        cx.run_until_parked();
        config.read_with(cx, |this, cx| {
            assert!(!this.is_saving());
            assert!(this.leave_after_save.is_none());
            assert_eq!(this.has_unsaved(cx), fail);
            assert_eq!(this.error.is_some(), fail);
        });
        assert_eq!(left.get(), !fail);
        if !fail {
            assert_eq!(
                read(&agent.path().join("settings.json"))["steeringMode"],
                "all"
            );
        }
    }
}

#[gpui_kit::test]
fn model_scope_round_trips_and_preserves_scope_and_conflict_rules(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(
        &path,
        r#"{"enabledModels":["p/a"],"unknown":{"keep":true}}"#,
    )
    .unwrap();
    let (config, cx) = setup(cx, agent.path());
    append(&config, " p/*:high ", cx);
    save(&config, Page::Conversation, cx);
    assert_eq!(read(&path)["enabledModels"], json!(["p/a", " p/*:high "]));
    assert_eq!(read(&path)["unknown"], json!({"keep":true}));
    append(&config, " ", cx);
    save(&config, Page::Conversation, cx);
    assert_eq!(read(&path)["enabledModels"], json!(["p/a", " p/*:high "]));
    config.update(cx, |this, cx| this.discard(None, cx));
    config.update(cx, |this, cx| {
        this.set_scope(Scope::Project(project.path().to_owned()), cx)
    });
    cx.run_until_parked();
    remove_all(&config, cx);
    save(&config, Page::Conversation, cx);
    assert_eq!(
        read(&project.path().join(".pi/settings.json"))["enabledModels"],
        json!([])
    );
    config.update(cx, |this, cx| {
        this.set_values(Field::ModelScope, vec![Slot::Unset], cx)
    });
    save(&config, Page::Conversation, cx);
    assert!(
        read(&project.path().join(".pi/settings.json"))
            .get("enabledModels")
            .is_none()
    );
    config.update(cx, |this, cx| this.set_scope(Scope::Global, cx));
    cx.run_until_parked();
    append(&config, "p/b", cx);
    fs::write(&path, r#"{"enabledModels":["external"]}"#).unwrap();
    save(&config, Page::Conversation, cx);
    assert_eq!(read(&path)["enabledModels"], json!(["external"]));
    assert!(config.read_with(cx, |this, _| this.conflicts.contains(&Field::ModelScope)));
    config.update(cx, |this, cx| {
        this.discard(None, cx);
        this.reload(cx);
    });
    cx.run_until_parked();
    remove_all(&config, cx);
    save(&config, Page::Conversation, cx);
    assert!(read(&path).get("enabledModels").is_none());
}

#[gpui_kit::test]
fn malformed_model_scope_is_not_overwritten(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(&path, r#"{"enabledModels":[4]}"#).unwrap();
    let (config, cx) = setup(cx, agent.path());
    append(&config, "p/a", cx);
    assert!(!config.read_with(cx, |this, cx| this.has_unsaved(cx)));
    assert_eq!(read(&path)["enabledModels"], json!([4]));
}

#[gpui_kit::test]
fn model_scope_row_actions_follow_the_row(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(&path, r#"{"enabledModels":["p/a","p/b","p/c"]}"#).unwrap();
    let (config, cx) = setup(cx, agent.path());
    let a = config.read_with(cx, |this, cx| this.row_keys(cx)[0].clone());
    for _ in 0..2 {
        cx.update(|window, cx| {
            config.update(cx, |this, cx| this.move_pattern(&a, true, window, cx))
        });
    }
    let input =
        cx.update(|window, cx| config.update(cx, |this, cx| this.row_input(&a, "p/a", window, cx)));
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("p/x", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        })
    });
    cx.run_until_parked();
    save(&config, Page::Conversation, cx);
    assert_eq!(read(&path)["enabledModels"], json!(["p/b", "p/c", "p/x"]));
}

#[gpui_kit::test]
fn discarded_row_paths_stay_retired(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"enabledModels":["p/a","p/b"]}"#,
    )
    .unwrap();
    let (config, cx) = setup(cx, agent.path());
    let before = config.read_with(cx, |this, cx| this.row_keys(cx));
    cx.update(|window, cx| {
        config.update(cx, |this, cx| {
            this.move_pattern(&before[0], true, window, cx);
            this.discard(None, cx);
        })
    });
    cx.update(|window, cx| {
        config.update(cx, |this, cx| this.remove_pattern(&before[0], window, cx))
    });
    assert_eq!(
        config.read_with(cx, |this, cx| this.model_scope_rows(cx)),
        Some(vec!["p/a".into(), "p/b".into()])
    );
}

#[gpui_kit::test]
fn page_save_reconciles_both_forms_using_the_newly_read_file(cx: &mut TestAppContext) {
    for dirty in [false, true] {
        let agent = tempfile::tempdir().unwrap();
        let path = agent.path().join("settings.json");
        fs::write(&path, r#"{"retry":{"maxRetries":1}}"#).unwrap();
        let (config, window) = setup(cx, agent.path());
        config.update(window, |this, cx| {
            this.set_values(Field::Compaction, vec![Slot::Set(json!(false))], cx);
            if dirty {
                this.set_values(Field::MaxRetries, vec![Slot::Set(json!(3))], cx);
            }
        });
        fs::write(&path, r#"{"retry":{"maxRetries":4},"unknown":true}"#).unwrap();
        save(&config, Page::Conversation, window);
        config.read_with(window, |this, cx| {
            assert_eq!(
                this.snapshot(cx).value(Field::MaxRetries),
                vec![Slot::Set(json!(if dirty { 3 } else { 4 }))]
            );
            assert_eq!(this.has_page_drafts(Page::Network, cx), dirty);
        });
        save(&config, Page::Network, window);
        assert_eq!(
            read(&path)["retry"]["maxRetries"],
            if dirty { 3 } else { 4 }
        );
        assert_eq!(read(&path)["unknown"], true);
    }
}

#[gpui_kit::test]
fn bound_invalid_input_survives_reload_and_other_page_save_and_reverts(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(&path, r#"{"retry":{"maxRetries":1}}"#).unwrap();
    let (config, window) = setup(cx, agent.path());
    let input = window.update(|window, cx| {
        config.update(cx, |this, cx| this.input(Field::MaxRetries, window, cx))
    });
    window.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("-", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
            input.set_value("1", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        });
    });
    window.run_until_parked();
    assert!(
        !config.read_with(window, |this, cx| this.has_unsaved(cx)),
        "a newer native edit back to the loaded value supersedes queued invalid text"
    );
    window.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("-", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        })
    });
    window.run_until_parked();
    config.read_with(window, |this, cx| {
        assert!(this.needs_leave_guard(cx));
        assert!(this.invalid(Field::MaxRetries, cx));
    });
    config.update(window, |this, cx| {
        this.set_values(Field::Compaction, vec![Slot::Set(json!(false))], cx)
    });
    save(&config, Page::Conversation, window);
    assert_eq!(read(&path)["compaction"]["enabled"], false);
    assert_eq!(input.read_with(window, |input, _| input.value()), "-");
    fs::write(
        &path,
        r#"{"retry":{"maxRetries":4},"compaction":{"enabled":false}}"#,
    )
    .unwrap();
    config.update(window, |this, cx| this.reload(cx));
    window.run_until_parked();
    assert_eq!(input.read_with(window, |input, _| input.value()), "-");
    save(&config, Page::Network, window);
    assert_eq!(read(&path)["retry"]["maxRetries"], 4);
    config.update(window, |this, cx| this.clear_draft(Field::MaxRetries, cx));
    window.run_until_parked();
    assert_eq!(input.read_with(window, |input, _| input.value()), "4");
    assert!(!config.read_with(window, |this, cx| this.needs_leave_guard(cx)));
}

#[gpui_kit::test]
fn inherited_rows_transfer_inputs_and_reject_old_callbacks_after_reload(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"enabledModels":["p/a","p/b"]}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_scope(Scope::Project(project.path().to_owned()), cx)
    });
    window.run_until_parked();
    let key = config.read_with(window, |this, cx| this.row_keys(cx)[1].clone());
    let input = window
        .update(|window, cx| config.update(cx, |this, cx| this.row_input(&key, "p/b", window, cx)));
    // Discard can reconstruct inherited rows without advancing the file-read generation.
    // A queued callback from the old native entity must not target its replacement.
    window.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("abandoned", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        });
        config.update(cx, |this, cx| {
            this.discard(Some(Page::Conversation), cx);
            this.row_input(&key, "p/b", window, cx);
        });
    });
    window.run_until_parked();
    let input = window
        .update(|window, cx| config.update(cx, |this, cx| this.row_input(&key, "p/b", window, cx)));
    assert!(!config.read_with(window, |this, cx| this.has_unsaved(cx)));
    window.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("p/b'", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
            // Consecutive native input before the deferred handoff must keep the newest text.
            input.set_value("p/b''", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        })
    });
    window.run_until_parked();
    config.read_with(window, |this, cx| {
        let new_key = &this.row_keys(cx)[1];
        assert_eq!(this.rows[new_key].input, input);
        assert_eq!(
            this.model_scope_rows(cx),
            Some(vec!["p/a".into(), "p/b''".into()])
        );
    });
    window.update(|window, cx| config.update(cx, |this, cx| this.remove_pattern(&key, window, cx)));
    assert_eq!(
        config.read_with(window, |this, cx| this.model_scope_rows(cx)),
        Some(vec!["p/a".into(), "p/b''".into()]),
        "a retired inherited button target cannot address the materialized list by index"
    );
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&project.path().join(".pi/settings.json"))["enabledModels"],
        json!(["p/a", "p/b''"])
    );
    config.update(window, |this, cx| {
        this.set_values(Field::ModelScope, vec![Slot::Unset], cx)
    });
    window.run_until_parked();
    let key = config.read_with(window, |this, cx| this.row_keys(cx)[1].clone());
    let input = window
        .update(|window, cx| config.update(cx, |this, cx| this.row_input(&key, "p/b", window, cx)));
    window.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("stale", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        });
        config.update(cx, |this, cx| this.reload(cx));
    });
    window.run_until_parked();
    assert_eq!(
        config.read_with(window, |this, cx| this
            .snapshot(cx)
            .value(Field::ModelScope)),
        vec![Slot::Unset]
    );
}

#[gpui_kit::test]
fn correcting_a_bound_number_restores_its_baseline_slot(cx: &mut TestAppContext) {
    for (project_scope, malformed) in [(false, false), (true, false), (false, true)] {
        let agent = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let path = agent.path().join("settings.json");
        let text = if malformed { "bad" } else { "16384" };
        let original = if malformed {
            json!({"compaction":{"reserveTokens":"bad"},"unknown":true})
        } else if project_scope {
            json!({"compaction":{"reserveTokens":16384},"unknown":true})
        } else {
            json!({"unknown":true})
        };
        fs::write(&path, original.to_string()).unwrap();
        let (config, visual) = setup(cx, agent.path());
        if project_scope {
            config.update(visual, |this, cx| {
                this.set_scope(Scope::Project(project.path().to_owned()), cx)
            });
            visual.run_until_parked();
        }
        let baseline =
            config.read_with(visual, |this, _| this.baseline.value(Field::ReserveTokens));
        input_text(&config, Field::ReserveTokens, &format!("{text}x"), visual);
        assert!(config.read_with(visual, |this, cx| this.invalid(Field::ReserveTokens, cx)));
        // This correction occurs AFTER the invalid edit has drained.
        input_text(&config, Field::ReserveTokens, text, visual);
        config.read_with(visual, |this, cx| {
            assert_eq!(this.snapshot(cx).value(Field::ReserveTokens), baseline);
            assert!(!this.has_unsaved(cx));
        });
        save(&config, Page::Conversation, visual);
        assert_eq!(read(&path), original);
        if project_scope {
            assert!(
                !project.path().join(".pi/settings.json").exists(),
                "correcting inherited text must not create a settings file"
            );
        }
    }
}

#[gpui_kit::test]
fn reloading_an_unchanged_conversation_keeps_local_input_entities(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"enabledModels":["p/a","p/b"]}"#,
    )
    .unwrap();
    let (config, visual) = setup(cx, agent.path());
    let key = config.read_with(visual, |this, cx| this.row_keys(cx)[0].clone());
    let input = visual
        .update(|window, cx| config.update(cx, |this, cx| this.row_input(&key, "p/a", window, cx)));
    config.update(visual, |this, cx| this.reload(cx));
    visual.run_until_parked();
    config.read_with(visual, |this, cx| {
        assert_eq!(this.row_keys(cx)[0], key);
        assert_eq!(this.rows[&key].input, input);
        assert!(!this.has_unsaved(cx));
    });
}

#[gpui_kit::test]
fn tools_save_preserves_invalid_conversation_and_network_drafts(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(
        &path,
        r#"{"httpIdleTimeoutMs":"DISABLED","unknown":{"keep":true}}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_values(Field::ReserveTokens, vec![Slot::Invalid("-".into())], cx);
        this.set_values(Field::MaxRetries, vec![Slot::Invalid("bad".into())], cx);
        this.set_values(
            Field::ShellPrefix,
            vec![Slot::Set(json!(" export TEST=1; "))],
            cx,
        );
    });
    save(&config, Page::Tools, window);
    let disk = read(&path);
    assert_eq!(disk["shellCommandPrefix"], " export TEST=1; ");
    assert_eq!(disk["httpIdleTimeoutMs"], "DISABLED");
    assert_eq!(disk["unknown"]["keep"], true);
    config.read_with(window, |this, cx| {
        assert!(this.invalid(Field::ReserveTokens, cx));
        assert!(this.invalid(Field::MaxRetries, cx));
        assert!(!this.has_page_drafts(Page::Tools, cx));
        assert!(this.needs_leave_guard(cx));
    });
}

#[gpui_kit::test]
fn tool_last_row_and_argv_use_distinct_empty_rules(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(
        &path,
        r#"{"defaultTools":["read"],"npmCommand":["npm",""," run "]}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    window.update(|window, cx| {
        config.update(cx, |this, cx| {
            let path = this
                .list_paths(Field::DefaultTools, cx)
                .unwrap()
                .try_items(&this.tools, cx)
                .unwrap()
                .remove(0);
            this.change_list(
                Field::DefaultTools,
                &model_scope::RowKey::Local(path.key()),
                0,
                window,
                cx,
            );
        })
    });
    save(&config, Page::Tools, window);
    assert_eq!(read(&path)["defaultTools"], json!([]));
    assert_eq!(read(&path)["npmCommand"], json!(["npm", "", " run "]));
    config.update(window, |this, cx| {
        this.set_values(Field::DefaultTools, vec![Slot::Unset], cx)
    });
    save(&config, Page::Tools, window);
    assert!(read(&path).get("defaultTools").is_none());
    window.update(|window, cx| {
        config.update(cx, |this, cx| {
            this.set_values(Field::NpmCommand, vec![Slot::Set(json!(["npm"]))], cx);
            let input = this.input(Field::NpmCommand, window, cx);
            input.update(cx, |input, cx| {
                input.set_value("", window, cx);
                cx.emit(gpui_kit::component::input::InputEvent::Change);
            });
        })
    });
    window.run_until_parked();
    save(&config, Page::Tools, window);
    assert!(read(&path).get("npmCommand").is_none());
}

#[gpui_kit::test]
fn model_tokens_inherit_per_leaf_and_report_exact_conflicts(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"compaction":{"modelOverrides":{"p/m":{"reserveTokens":100,"keepRecentTokens":200}}}}"#,
    )
    .unwrap();
    fs::create_dir(project.path().join(".pi")).unwrap();
    let path = project.path().join(".pi/settings.json");
    fs::write(&path,r#"{"compaction":{"modelOverrides":{"p/m":{"reserveTokens":300,"custom":true}}},"unknown":7}"#).unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_scope(Scope::Project(project.path().to_owned()), cx)
    });
    window.run_until_parked();
    config.read_with(window, |this, cx| {
        assert_eq!(
            this.token_value("p/m", false, cx),
            ("300".into(), fields::Source::Project)
        );
        assert_eq!(
            this.token_value("p/m", true, cx),
            ("200".into(), fields::Source::Global)
        );
    });
    config.update(window, |this, cx| {
        this.set_token("p/m", false, Slot::Set(400), cx)
    });
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&path)["compaction"]["modelOverrides"]["p/m"],
        json!({"reserveTokens":400,"custom":true})
    );
    config.update(window, |this, cx| {
        this.set_token("p/m", false, Slot::Set(500), cx)
    });
    fs::write(&path,r#"{"compaction":{"modelOverrides":{"p/m":{"reserveTokens":450,"custom":true}}},"unknown":7}"#).unwrap();
    save(&config, Page::Conversation, window);
    config.read_with(window, |this, cx| {
        assert!(this.conflict_leaves.contains(&vec![
            "compaction".into(),
            "modelOverrides".into(),
            "p/m".into(),
            "reserveTokens".into()
        ]));
        assert!(this.conflicts.contains(&Field::ModelOverrides));
        assert_eq!(this.token_value("p/m", false, cx).0, "500");
    });
    config.update(window, |this, cx| this.reload(cx));
    window.run_until_parked();
    config.update(window, |this, cx| {
        this.remove_map_row(Field::ModelOverrides, "p/m", cx)
    });
    save(&config, Page::Conversation, window);
    config.read_with(window, |this, cx| {
        assert_eq!(
            this.token_value("p/m", false, cx),
            ("100".into(), fields::Source::Global)
        )
    });
    assert_eq!(
        read(&path)["compaction"]["modelOverrides"]["p/m"],
        json!({"custom":true})
    );
    assert_eq!(read(&path)["unknown"], 7);
}

#[gpui_kit::test]
fn inherited_model_input_keeps_invalid_text_across_other_page_save(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"compaction":{"modelOverrides":{"p/m":{"reserveTokens":100,"keepRecentTokens":200}}}}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_scope(Scope::Project(project.path().to_owned()), cx)
    });
    window.run_until_parked();
    let input = window.update(|window, cx| {
        config.update(cx, |this, cx| this.token_input("p/m", false, window, cx))
    });
    window.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("-", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        })
    });
    window.run_until_parked();
    config.read_with(window, |this, cx| {
        assert!(this.invalid(Field::ModelOverrides, cx))
    });
    config.update(window, |this, cx| {
        this.set_values(Field::Telemetry, vec![Slot::Set(json!(false))], cx)
    });
    save(&config, Page::Tools, window);
    config.read_with(window, |this, cx| {
        assert_eq!(this.token_slot("p/m", false, cx), Slot::Invalid("-".into()));
        assert_eq!(this.token_value("p/m", true, cx).0, "200");
    });
    assert_eq!(
        input.read_with(window, |input, _| input.value().to_string()),
        "-"
    );
}

#[gpui_kit::test]
fn model_map_picker_rejects_duplicate_keys_and_keeps_unavailable_records(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(
        &path,
        r#"{"modelThinkingLevels":{"missing/model":false},"unknown":true}"#,
    )
    .unwrap();
    let (config, window) = setup(cx, agent.path());
    for field in [Field::ModelThinking, Field::ModelOverrides] {
        let picker = config.read_with(window, |this, _| this.map_pickers[&field].clone());
        for _ in 0..2 {
            picker.update(window, |_, cx| {
                cx.emit(gpui_kit::component::select::SelectEvent::<
                    gpui_kit::component::searchable_list::SearchableVec<super::ModelItem>,
                >::Confirm(Some(("p".into(), "new".into()))))
            });
            window.run_until_parked();
        }
        config.read_with(window, |this, cx| {
            let raw = this.snapshot(cx).value(field)[0].raw().unwrap();
            let map = raw.as_object().unwrap();
            assert_eq!(map.keys().filter(|key| key.as_str() == "p/new").count(), 1);
        });
        config.update(window, |this, cx| this.remove_map_row(field, "p/new", cx));
    }
    config.update(window, |this, cx| {
        this.set_values(Field::CacheWarming, vec![Slot::Set(json!("off"))], cx)
    });
    save(&config, Page::Conversation, window);
    assert_eq!(read(&path)["modelThinkingLevels"]["missing/model"], false);
    assert!(read(&path).get("compaction").is_none());
    assert_eq!(read(&path)["unknown"], true);
}

#[gpui_kit::test]
fn saving_model_maps_keeps_disk_order_and_clears_dirty(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(&path, r#"{"modelThinkingLevels":{"z/model":"high","a/model":"low"},"compaction":{"modelOverrides":{"z/model":{"reserveTokens":100},"a/model":{"keepRecentTokens":200}}}}"#).unwrap();
    let (config, window) = setup(cx, agent.path());
    config.update(window, |this, cx| {
        this.set_values(Field::ModelThinking, vec![Slot::Set(json!({"z/model":"medium","a/model":"low","b/model":"off"}))], cx);
        this.set_values(Field::ModelOverrides, vec![Slot::Set(json!({"z/model":{"reserveTokens":300},"a/model":{"keepRecentTokens":200},"b/model":{"reserveTokens":50}}))], cx);
    });
    save(&config, Page::Conversation, window);
    config.read_with(window, |this, cx| {
        assert!(!this.has_unsaved(cx));
        assert!(!this.needs_leave_guard(cx));
        assert!(this.snapshot(cx) == this.baseline);
    });
    assert_eq!(read(&path)["modelThinkingLevels"]["b/model"], "off");
}

#[gpui_kit::test]
fn compact_tools_append_remove_and_npm_invalid_input(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(
        &path,
        r#"{"defaultTools":["read","bash"],"npmCommand":["npm",""," run "]}"#,
    )
    .unwrap();
    let (config, visual) = setup(cx, agent.path());
    visual.update(|window, cx| {
        config.update(cx, |this, cx| {
            this.append_list(Field::DefaultTools, "+custom".into(), window, cx);
            let paths = this
                .list_paths(Field::DefaultTools, cx)
                .unwrap()
                .try_items(&this.tools, cx)
                .unwrap();
            this.change_list(
                Field::DefaultTools,
                &model_scope::RowKey::Local(paths[1].key()),
                0,
                window,
                cx,
            );
            let input = this.input(Field::NpmCommand, window, cx);
            input.update(cx, |input, cx| {
                input.set_value("npm 'unfinished", window, cx);
                cx.emit(gpui_kit::component::input::InputEvent::Change);
            });
        })
    });
    visual.run_until_parked();
    config.read_with(visual, |this, cx| {
        assert_eq!(
            this.snapshot(cx).value(Field::DefaultTools),
            vec![Slot::Set(json!(["read", "+custom"]))]
        );
        assert!(this.invalid(Field::NpmCommand, cx));
        assert!(this.has_page_drafts(Page::Tools, cx));
    });
    let input = visual.update(|window, cx| {
        config.update(cx, |this, cx| this.input(Field::NpmCommand, window, cx))
    });
    visual.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("npm '' ' run '", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        })
    });
    visual.run_until_parked();
    save(&config, Page::Tools, visual);
    assert_eq!(read(&path)["npmCommand"], json!(["npm", "", " run "]));
    assert_eq!(read(&path)["defaultTools"], json!(["read", "+custom"]));
}

#[gpui_kit::test]
fn grouped_budget_inputs_reconcile_independently(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let path = agent.path().join("settings.json");
    fs::write(
        &path,
        r#"{"thinkingBudgets":{"minimal":1,"low":2,"medium":3,"high":4}}"#,
    )
    .unwrap();
    let (config, visual) = setup(cx, agent.path());
    visual.update(|window, cx| {
        config.update(cx, |this, cx| {
            for (field, text) in [(Field::BudgetMinimal, "10"), (Field::BudgetHigh, "40")] {
                let input = this.input(field, window, cx);
                input.update(cx, |input, cx| {
                    input.set_value(text, window, cx);
                    cx.emit(gpui_kit::component::input::InputEvent::Change);
                });
            }
            this.set_values(Field::Transport, vec![Slot::Set(json!("sse"))], cx);
        })
    });
    visual.run_until_parked();
    fs::write(
        &path,
        r#"{"thinkingBudgets":{"minimal":1,"low":25,"medium":3,"high":4}}"#,
    )
    .unwrap();
    save(&config, Page::Network, visual);
    config.read_with(visual, |this, cx| {
        assert_eq!(
            this.snapshot(cx).value(Field::BudgetMinimal),
            vec![Slot::Set(json!(10))]
        );
        assert_eq!(
            this.snapshot(cx).value(Field::BudgetLow),
            vec![Slot::Set(json!(25))]
        );
        assert_eq!(
            this.snapshot(cx).value(Field::BudgetHigh),
            vec![Slot::Set(json!(40))]
        );
    });
    save(&config, Page::Conversation, visual);
    assert_eq!(
        read(&path)["thinkingBudgets"],
        json!({"minimal":10,"low":25,"medium":3,"high":40})
    );
}

#[gpui_kit::test]
fn resource_arrays_are_raw_ordered_and_scope_local(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(agent.path().join("settings.json"), json!({"extensions":["global"],"skills":["global"],"prompts":["global"],"themes":["global"]}).to_string()).unwrap();
    let (config, visual) = setup(cx, agent.path());
    config.update(visual, |this, cx| {
        this.set_scope(Scope::Project(project.path().to_owned()), cx)
    });
    visual.run_until_parked();
    config.update(visual, |this, cx| {
        for field in Field::RESOURCE_PATHS {
            assert_eq!(this.snapshot(cx).value(field), vec![Slot::Unset]);
            assert_eq!(this.resolved(field, cx).values, vec![None]);
            for text in ["", "same", "same", "!*.ts", "+a", "-a", "-builtin:mcp"] {
                this.append_resource_path(field, text.into(), cx);
            }
        }
    });
    save(&config, Page::Tools, visual);
    let file = project.path().join(".pi/settings.json");
    let saved = read(&file);
    for key in ["extensions", "skills", "prompts", "themes"] {
        assert_eq!(
            saved[key],
            json!(["", "same", "same", "!*.ts", "+a", "-a", "-builtin:mcp"])
        );
    }
    config.update(visual, |this, cx| {
        for field in Field::RESOURCE_PATHS {
            this.set_values(field, vec![Slot::Set(json!([]))], cx);
        }
    });
    save(&config, Page::Tools, visual);
    for key in ["extensions", "skills", "prompts", "themes"] {
        assert_eq!(read(&file)[key], json!([]));
    }
    config.update(visual, |this, cx| {
        for field in Field::RESOURCE_PATHS {
            this.set_values(field, vec![Slot::Unset], cx);
        }
    });
    save(&config, Page::Tools, visual);
    assert_eq!(read(&file), json!({}));
    assert_eq!(
        read(&agent.path().join("settings.json"))["extensions"],
        json!(["global"])
    );
}

#[gpui_kit::test]
fn switch_reread_preserves_dirty_array_conflict_but_updates_clean_arrays(cx: &mut TestAppContext) {
    for project_scope in [false, true] {
        let agent = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let root = if project_scope {
            project.path().join(".pi")
        } else {
            agent.path().to_owned()
        };
        fs::create_dir_all(root.join("prompts")).unwrap();
        fs::write(root.join("prompts/a.md"), "a").unwrap();
        fs::create_dir_all(root.join("extensions")).unwrap();
        fs::write(root.join("extensions/a.ts"), "export default () => {};").unwrap();
        fs::write(
            root.join("settings.json"),
            json!({"extensions":["old"],"unknown":true}).to_string(),
        )
        .unwrap();
        let (config, visual) = setup(cx, agent.path());
        if project_scope {
            config.update(visual, |this, cx| {
                this.set_scope(Scope::Project(project.path().to_owned()), cx)
            });
            visual.run_until_parked();
        }
        config.update(visual, |this, cx| {
            this.append_resource_path(Field::Prompts, "draft".into(), cx)
        });
        // Real switch write, then the refresh used by ResourcesView.
        let resource = gupi_resources::pi_resources::Resource::new(
            gupi_resources::pi_resources::Kind::Prompt,
            root.join("prompts/a.md"),
            root.clone(),
            "a".into(),
        );
        let extension = gupi_resources::pi_resources::Resource::new(
            gupi_resources::pi_resources::Kind::Extension,
            root.join("extensions/a.ts"),
            root.clone(),
            "a".into(),
        );
        for resource in [resource, extension] {
            if project_scope {
                gupi_resources::pi_resources::set_project_enabled(project.path(), &resource, false)
                    .unwrap();
            } else {
                gupi_resources::pi_resources::set_enabled(&root, &resource, false).unwrap();
            }
        }
        config.update(visual, |this, cx| this.refresh_resource_paths(cx));
        visual.run_until_parked();
        config.read_with(visual, |this, cx| {
            assert_eq!(
                this.snapshot(cx).value(Field::Extensions),
                vec![Slot::Set(json!(["old", "-extensions/a.ts"]))]
            );
            assert_eq!(this.baseline.value(Field::Prompts), vec![Slot::Unset]);
        });
        // An unrelated page save must not adopt the new resource base either.
        config.update(visual, |this, cx| {
            this.set_values(Field::MaxRetries, vec![Slot::Set(json!(4))], cx)
        });
        save(&config, Page::Network, visual);
        save(&config, Page::Tools, visual);
        config.read_with(visual, |this, _| {
            assert!(this.conflicts.contains(&Field::Prompts))
        });
        assert_eq!(
            read(&root.join("settings.json"))["prompts"],
            json!(["-prompts/a.md"])
        );
        assert_eq!(read(&root.join("settings.json"))["unknown"], true);
    }
}

impl PiConfig {
    pub(in crate::features::settings) fn append_prompt_path_for_test(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.append_resource_path(Field::Prompts, text, cx);
    }
}
