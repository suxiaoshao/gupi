use super::Field;
use super::Load;
use super::Page;
use super::PiConfig;
use super::Support;
use super::Trust;
use super::fields;
use super::fields::Edit;
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
        self.pi_config.update(cx, |config, _| {
            config
                .drafts
                .insert(Field::Steering, Edit::Set(vec![json!("all")]));
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
            this.set_agent_for_test(agent);
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
        this.set_draft(Field::Compaction, Edit::Set(vec![json!(false)]), cx);
        this.set_draft(Field::MaxRetries, Edit::Set(vec![json!(5)]), cx);
    });
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&global),
        json!({"theme":"dark","compaction":{"enabled":false,"custom":1}})
    );
    config.update(window, |this, _| {
        assert_eq!(
            this.drafts.keys().copied().collect::<Vec<_>>(),
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
    config.update(window, |this, cx| {
        this.on_text(Field::MaxRetries, "-1".into(), cx);
        assert!(matches!(
            this.drafts.get(&Field::MaxRetries),
            Some(Edit::Invalid(_))
        ));
    });
    save(&config, Page::Network, window);
    config.update(window, |this, cx| {
        assert!(
            this.error.is_some(),
            "invalid text is reported, not written"
        );
        this.on_text(Field::MaxRetries, "2".into(), cx);
        assert!(
            this.drafts.is_empty(),
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
        this.set_draft(Field::Steering, Edit::Set(vec![json!("one-at-a-time")]), cx);
    });
    fs::write(&global, r#"{"steeringMode":"all","followUpMode":"all"}"#).unwrap();
    // A different leaf changed: the save still succeeds and keeps it.
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&global),
        json!({"steeringMode":"one-at-a-time","followUpMode":"all"})
    );

    config.update(window, |this, cx| {
        this.set_draft(Field::Steering, Edit::Set(vec![json!("all")]), cx);
    });
    fs::write(&global, r#"{"followUpMode":"all"}"#).unwrap();
    save(&config, Page::Conversation, window);
    assert_eq!(
        read(&global),
        json!({"followUpMode":"all"}),
        "nothing overwritten"
    );
    config.update(window, |this, _| {
        assert_eq!(this.conflicts, [Field::Steering]);
        assert!(
            this.drafts.contains_key(&Field::Steering),
            "the draft survives"
        );
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
        this.set_draft(Field::Proxy, Edit::Set(vec![json!("http://x")]), cx);
        this.set_draft(Field::Compaction, Edit::Set(vec![json!(false)]), cx);
        this.discard(Some(Page::Network), cx);
        assert_eq!(
            this.drafts.keys().copied().collect::<Vec<_>>(),
            [Field::Compaction]
        );
        this.discard(None, cx);
        assert!(!this.has_unsaved());
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
fn both_pages_render_fields_in_each_scope(cx: &mut TestAppContext) {
    let agent = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        agent.path().join("settings.json"),
        r#"{"defaultProvider":"p","defaultModel":"m","defaultThinkingLevel":"xhigh","httpProxy":"http://p"}"#,
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
        for page in [Page::Conversation, Page::Network] {
            host.update(window, |host, cx| {
                host.1 = page;
                cx.notify();
            });
            for width in [480., 1200.] {
                window.simulate_resize(gpui_kit::size(gpui_kit::px(width), gpui_kit::px(900.)));
                window.run_until_parked();
            }
        }
    }
    config.update(cx, |this, _| {
        assert!(matches!(this.files, Load::Ready(_)));
        assert!(this.drafts.is_empty(), "rendering never creates drafts");
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
        this.set_draft(Field::Steering, Edit::Set(vec![json!("all")]), cx);
        for path in [
            agent.path().join("missing"),
            std::path::PathBuf::from("relative"),
        ] {
            this.set_scope(Scope::Project(path), cx);
            assert_eq!(this.scope, Scope::Global);
            assert!(this.drafts.contains_key(&Field::Steering));
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
        this.set_draft(Field::Steering, Edit::Set(vec![json!("all")]), cx)
    });
    fs::remove_dir(&project).unwrap();
    save(&config, Page::Conversation, cx);
    assert!(!project.exists());
    config.read_with(cx, |this, _| {
        assert!(this.drafts.contains_key(&Field::Steering));
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
                this.set_draft(Field::Steering, Edit::Set(vec![json!("all")]), cx);
                if fail {
                    fs::write(agent.path().join("settings.json"), "invalid JSON").unwrap();
                }
                this.save(Some(Page::Conversation), None, window, cx);
                assert!(this.is_saving());
                let generation = this.generation;
                this.reload(cx);
                this.discard(None, cx);
                assert_eq!(this.generation, generation);
                assert!(this.has_unsaved());
            });
            let left = left.clone();
            PiConfig::confirm_leave(&config, move |_, _| left.set(true), window, cx);
            assert!(config.read(cx).leave_after_save.is_some());
        });
        assert!(!left.get());
        cx.run_until_parked();
        config.read_with(cx, |this, _| {
            assert!(!this.is_saving());
            assert!(this.leave_after_save.is_none());
            assert_eq!(this.has_unsaved(), fail);
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
