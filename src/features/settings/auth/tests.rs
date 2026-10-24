use super::AuthPage;
use gpui_kit::component::{Root, WindowExt as _};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, TestAppContext, Window};
use gupi_resources::pi_auth::File;
use gupi_settings::{
    config::{AppConfig, ConfigController},
    i18n,
};
struct Host(Entity<AuthPage>);
impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |this, cx| this.render_page(window, cx))
    }
}
#[gpui_kit::test]
async fn editor_retains_masked_keys_on_busy_and_saves_without_exposing_existing_values(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // Coordinate clicks must not race the dialog's wall-clock entrance motion;
        // this test exercises masked credentials and persistence, not animation.
        cx.set_reduce_motion(true);
        crate::app::init_capability_hosts(cx);
        i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    std::fs::write(
        &path,
        r#"{"p":{"type":"api_key","key":"fixture-original","env":{"ID":"fixture-env"}},"q":{"type":"future","private":"fixture-private"}}"#,
    )
    .unwrap();
    let mut owner = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| gpui_form::Form::new(AppConfig::default()));
        let controller = cx.new(|cx| ConfigController::new(&form, cx));
        let auth = cx.new(|_| AuthPage::new(controller));
        auth.update(cx, |this, _| {
            this.set_agent_for_test(dir.path().to_owned());
            this.file = Some(File::load(&path).unwrap());
            this.names = vec!["p".into(), "q".into()];
        });
        owner = Some(auth.clone());
        let host = cx.new(|_| Host(auth));
        Root::new(host, window, cx)
    });
    let owner = owner.unwrap();
    // Reload preserves the already loaded list, disabling actions until it finishes.
    owner.update(cx, |this, cx| this.reload(cx));
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("auth-row-p").visible());
        window.click("auth-row-p", cx);
        window.click("auth-add", cx);
        assert!(!window.has_active_dialog(cx));
    });
    cx.condition(&owner, |this, _| this.loading.is_none()).await;
    // If the row disappears while editing, cancel returns to Add. A modal
    // opened during closure must keep its own focus instead of being robbed.
    for opens_new_modal in [false, true] {
        owner.update(cx, |this, cx| {
            this.names = vec!["p".into(), "q".into()];
            cx.notify();
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("auth-row-p", cx);
        });
        owner.update(cx, |this, cx| {
            this.names.retain(|name| name != "p");
            cx.notify();
        });
        let mut new_modal_focus = None;
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.render_frame(cx);
            window.press("escape", cx);
            if opens_new_modal {
                window.open_dialog(cx, |dialog, _, _| dialog.title("New modal"));
                new_modal_focus = window.focused(cx);
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            if let Some(focus) = &new_modal_focus {
                assert!(window.has_active_dialog(cx));
                assert!(focus.contains_focused(window, cx));
                window.close_dialog(cx);
            } else {
                assert!(!window.has_active_dialog(cx));
                assert_eq!(window.find("auth-add").focused(), Some(true));
            }
        });
    }
    owner.update(cx, |this, cx| {
        this.names = vec!["p".into(), "q".into()];
        cx.notify();
    });
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("auth-add", cx);
        window.render_frame(cx);
        assert!(window.try_find("auth-candidates").is_some());
        window.click("auth-provider", cx);
        window.input("free text provider", cx);
        window.render_frame(cx);
        assert_eq!(
            window.find("auth-provider").value(),
            Some("free text provider")
        );
        window.press("escape", cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(!window.has_active_dialog(cx));
        assert_eq!(window.find("auth-add").focused(), Some(true));
        window.press("tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find("auth-reload").focused(), Some(true));
        window.press("shift-tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find("auth-add").focused(), Some(true));
        window.click("auth-row-p", cx);
        window.render_frame(cx);
        window.render_frame(cx);
        assert_eq!(window.find("auth-key").value(), None);
        assert!(window.try_find("auth-candidates").is_none());
        window.click("auth-save", cx);
        assert!(window.has_active_dialog(cx));
        window.click("auth-key", cx);
        window.input("fixture-new", cx);
        window.render_frame(cx);
        assert_eq!(window.find("auth-key").value(), None);
        std::fs::create_dir(dir.path().join("auth.json.lock")).unwrap();
        window.click("auth-save", cx);
        window.render_frame(cx);
        assert!(window.find("auth-row-p").visible());
    });
    cx.condition(&owner, |this, _| this.saving.is_none()).await;
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        assert_eq!(window.find("auth-key").value(), None);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("fixture-original")
        );
        std::fs::remove_dir(dir.path().join("auth.json.lock")).unwrap();
        // Retry the same input without retyping the key.
        window.click("auth-save", cx);
    });
    cx.condition(&owner, |this, _| this.saving.is_none()).await;
    cx.run_until_parked();
    cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    let root: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(root["p"]["key"], "fixture-new");
    assert_eq!(root["p"]["env"]["ID"], "fixture-env");
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("auth-row-q", cx);
        window.click("auth-key", cx);
        window.input("fixture-q-new", cx);
        window.click("auth-save", cx);
        assert_eq!(window.find("auth-key").value(), None);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("fixture-private")
        );
        window.click("auth-cancel", cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("auth-key").value(), None);
        // Cancelling confirmation also preserves the typed key.
        window.click("auth-save", cx);
        window.click("auth-save", cx);
    });
    cx.condition(&owner, |this, _| this.saving.is_none()).await;
    cx.run_until_parked();
    cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    let root: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        root["q"],
        serde_json::json!({"type":"api_key", "key":"fixture-q-new"})
    );
    assert!(owner.read_with(cx, |this, _| this.error.is_none()));
}

#[gpui_kit::test]
async fn empty_ids_keep_their_identity_through_actions_and_candidate_edits(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // Coordinate clicks must not race the dialog's wall-clock entrance motion;
        // this test exercises credential identity and persistence, not animation.
        cx.set_reduce_motion(true);
        crate::app::init_capability_hosts(cx);
        i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    std::fs::write(&path, r#"{"":{"type":"future","private":"fixture-secret"},"Empty ID":{"type":"api_key","key":"fixture-literal"}}"#).unwrap();
    std::fs::write(dir.path().join("models.json"), r#"{"providers":{"":{}}}"#).unwrap();
    let mut owner = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| gpui_form::Form::new(AppConfig::default()));
        let controller = cx.new(|cx| ConfigController::new(&form, cx));
        let auth = cx.new(|_| AuthPage::new(controller));
        auth.update(cx, |this, cx| {
            this.set_agent_for_test(dir.path().to_owned());
            this.reload(cx);
        });
        owner = Some(auth.clone());
        let host = cx.new(|_| Host(auth));
        Root::new(host, window, cx)
    });
    let owner = owner.unwrap();
    cx.condition(&owner, |this, _| this.loading.is_none()).await;
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(
            window
                .find("auth-row-")
                .label()
                .unwrap()
                .contains("⟨Empty ID⟩")
        );
        assert!(
            window
                .find("auth-row-Empty ID")
                .label()
                .unwrap()
                .contains("Empty ID")
        );
        window.click("auth-menu-", cx);
        assert_eq!(
            window.within("popup-menu").find(3usize).label(),
            Some("Pi cannot check this provider ID.")
        );
        window.within("popup-menu").click(2usize, cx);
        assert!(owner.read(cx).checking.is_empty());
        window.press("escape", cx);
        window.click("auth-row-", cx);
    });
    cx.update(|window, cx| {
        assert_eq!(window.find("auth-provider").label(), Some("⟨Empty ID⟩"));
        window.click("auth-key", cx);
        window.input("fixture-empty-replacement", cx);
        assert_eq!(window.find("auth-key").value(), None);
        window.click("auth-save", cx);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("fixture-secret")
        );
        window.click("auth-save", cx);
        assert!(owner.read(cx).saving.is_some());
    });
    cx.condition(&owner, |this, _| {
        this.saving.is_none() && this.loading.is_none()
    })
    .await;
    cx.run_until_parked();
    let root: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        root[""],
        serde_json::json!({"type":"api_key","key":"fixture-empty-replacement"})
    );
    assert_eq!(root["Empty ID"]["key"], "fixture-literal");
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("auth-menu-", cx);
        window.within("popup-menu").click(4usize, cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.click("confirm", cx));
    // Confirm dispatches a deferred action; run it before waiting for idle.
    cx.run_until_parked();
    cx.condition(&owner, |this, _| {
        this.saving.is_none() && this.loading.is_none()
    })
    .await;
    cx.run_until_parked();
    let root: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(root.get("").is_none());
    cx.update(|window, cx| {
        window.render_frame(cx);
        // The models-only empty candidate remains listed and can be selected.
        assert!(window.find("auth-row-").visible());
        window.click("auth-add", cx);
        window.click("auth-key", cx);
        window.input("fixture-candidate", cx);
        window.click("auth-save", cx);
        assert!(window.has_active_dialog(cx));
        window.click("auth-candidates", cx);
    });
    // Auth keys precede models-only candidates: literal name, then empty ID.
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("auth-provider").label(), Some("⟨Empty ID⟩"));
        window.click("auth-provider", cx);
        window.input("x", cx);
        window.press("backspace", cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_ne!(window.find("auth-provider").label(), Some("⟨Empty ID⟩"));
        window.click("auth-save", cx);
        assert!(window.has_active_dialog(cx));
        window.click("auth-candidates", cx);
    });
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("auth-provider").label(), Some("⟨Empty ID⟩"));
        window.click("auth-save", cx);
        assert!(owner.read(cx).saving.is_some());
    });
    cx.condition(&owner, |this, _| {
        this.saving.is_none() && this.loading.is_none()
    })
    .await;
    cx.run_until_parked();
    cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    let root: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(root[""]["key"], "fixture-candidate");
    assert!(root.get("x").is_none());
    // Empty OAuth identity is still manageable, with a plain TUI fallback.
    std::fs::write(&path, r#"{"":{"type":"oauth","access":"fixture-only"}}"#).unwrap();
    owner.update(cx, |this, cx| this.reload(cx));
    cx.condition(&owner, |this, _| this.loading.is_none()).await;
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("auth-menu-", cx);
        window.within("popup-menu").click(2usize, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("/login")
        );
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.click("auth-row-", cx);
        assert!(window.has_active_dialog(cx));
        assert!(window.find("confirm").label().unwrap().contains("Log out"));
        window.click("cancel", cx);
    });
}
