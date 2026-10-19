use super::{File, FilePages, Kind};
use gpui_kit::component::{
    Root, WindowExt as _,
    setting::{SettingPage, Settings},
};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, TestAppContext, Window};
struct Host {
    files: Entity<FilePages>,
    config: Entity<super::super::pi_config::PiConfig>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Settings::new("auth-metadata-settings")
            .page(FilePages::page(&self.files, Kind::Models, &self.config, cx))
            .page(SettingPage::new("Other").group(
                gpui_kit::component::setting::SettingGroup::new().item(
                    gpui_kit::component::setting::SettingItem::render(|_, _, _| gpui_kit::div()),
                ),
            ))
    }
}
#[gpui_kit::test]
async fn entry_refreshes_metadata_without_resetting_drafts_or_looping(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let models = dir.path().join("models.json");
    let auth = dir.path().join("auth.json");
    std::fs::write(&models, r#"{"providers":{"p":{"baseUrl":"disk"}}}"#).unwrap();
    std::fs::write(&auth, "{}").unwrap();
    let mut files = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let probe = cx.new(|_| gupi_pi_runtime::PiProbeController::new());
        let config = cx.new(|cx| super::super::pi_config::PiConfig::new(probe, window, cx));
        config.update(cx, |config, cx| {
            config.set_agent_for_test(dir.path().to_owned(), cx)
        });
        let owner = config.read(cx).file_pages.clone();
        owner.update(cx, |this, _| {
            this.models.file = Some(File::load(Kind::Models, &models).unwrap());
            let mut entry = this.models.file.as_ref().unwrap().entry("p").unwrap();
            entry
                .set(&["baseUrl".into()], Some(serde_json::json!("draft")))
                .unwrap();
            this.models.edit("p".into(), Some(entry));
        });
        files = Some(owner.clone());
        let host = cx.new(|_| Host {
            files: owner,
            config,
        });
        Root::new(host, window, cx)
    });
    let files = files.unwrap();
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
    let first = files.read_with(cx, |this, _| this.auth_generation);
    assert_eq!(first, 2); // Agent injection invalidates generation 0 before the first entry.
    files.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
    assert_eq!(files.read_with(cx, |this, _| this.auth_generation), first);
    cx.update(|window, cx| window.within("settings-sidebar").click("0-1", cx));
    // Model a successful Authentication-page write while the Models page is absent.
    let snapshot = gupi_resources::pi_auth::File::load(&auth).unwrap();
    snapshot
        .commit("p", &snapshot.entry("p").revision(), Some("fixture-secret"))
        .unwrap();
    cx.update(|window, cx| window.within("settings-sidebar").click("0-0", cx));
    cx.run_until_parked();
    files.read_with(cx, |this, _| {
        assert!(this.has_stored_credentials("p"));
        assert_eq!(this.auth_generation, first + 1);
        assert!(this.models.dirty());
        assert_eq!(
            this.models.entry("p").unwrap().visible(&["baseUrl".into()]),
            Some(serde_json::json!("draft"))
        );
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert!(cx.debug_bounds("file-auth-p").is_some());
    cx.update(|_, cx| gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::German, cx));
    cx.simulate_resize(gpui_kit::size(gpui_kit::px(800.), gpui_kit::px(600.)));
    cx.update(|window, cx| window.render_frame(cx));
    let identity = cx.debug_bounds("file-name-p").unwrap();
    let marker = cx.debug_bounds("file-auth-p").unwrap();
    assert!(identity.size.width > gpui_kit::px(100.));
    assert!(marker.top() >= identity.bottom());
    cx.update(|window, _| assert!(window.find("file-add").bounds().right() <= gpui_kit::px(800.)));
    cx.update(|window, cx| window.within("settings-sidebar").click("0-1", cx));
    std::fs::write(&auth, "invalid").unwrap();
    cx.update(|window, cx| window.within("settings-sidebar").click("0-0", cx));
    cx.run_until_parked();
    files.read_with(cx, |this, _| {
        assert!(this.auth_providers.is_none());
        assert!(this.models.dirty());
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert!(cx.debug_bounds("file-auth-p").is_none());
    cx.update(|window, cx| {
        window.click("file-row-p", cx);
        assert!(window.has_active_dialog(cx));
        window.close_dialog(cx);
        window.click("file-reset", cx);
    });
    std::fs::write(&auth, r#"{"p":{"type":"api_key","key":"fixture-secret"}}"#).unwrap();
    cx.update(|window, cx| window.click("file-reload", cx));
    cx.condition(&files, |this, _| {
        this.auth_loading.is_none() && this.loading.is_none()
    })
    .await;
    files.read_with(cx, |this, _| {
        assert!(this.has_stored_credentials("p"));
        assert!(!this.models.dirty());
    });
}
struct NavigationHost {
    settings: Entity<super::super::SettingsView>,
    _observation: gpui_kit::Subscription,
}
impl Render for NavigationHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.settings.update(cx, |this, cx| this.settings_panel(cx))
    }
}
#[gpui_kit::test]
async fn credential_links_use_leave_guard_and_only_change_pages(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let models = dir.path().join("models.json");
    std::fs::write(&models, r#"{"providers":{"":{}}}"#).unwrap();
    std::fs::write(dir.path().join("auth.json"), r#"{"":{"type":"api_key","key":"fixture-only"},"orphan":{"type":"api_key","key":"fixture-only"}}"#).unwrap();
    let mut settings = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| gpui_form::Form::new(gupi_settings::config::AppConfig::default()));
        let controller = cx.new(|cx| gupi_settings::config::ConfigController::new(&form, cx));
        let draft = cx.new(|_| gupi_pi_runtime::PiProbeController::new());
        let applied = cx.new(|_| {
            let mut probe = gupi_pi_runtime::PiProbeController::new();
            probe.stop();
            probe
        });
        let view = cx.new(|cx| {
            super::super::SettingsView::new(
                form,
                controller,
                draft,
                applied,
                cx.focus_handle(),
                window,
                cx,
            )
        });
        view.update(cx, |this, cx| {
            this.domain = super::super::layout::Domain::Pi;
            this.navigation.1 = super::super::layout::MODELS_PAGE;
            this.pi_config.update(cx, |config, cx| {
                config.set_agent_for_test(dir.path().to_owned(), cx)
            });
            this.auth
                .update(cx, |auth, _| auth.set_agent_for_test(dir.path().to_owned()));
        });
        settings = Some(view.clone());
        let host = cx.new(|cx| {
            let observation = cx.observe(&view, |_, _, cx| cx.notify());
            NavigationHost {
                settings: view,
                _observation: observation,
            }
        });
        Root::new(host, window, cx)
    });
    let settings = settings.unwrap();
    let files = settings.read_with(cx, |this, cx| this.pi_config.read(cx).file_pages.clone());
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
    assert!(files.read_with(cx, |this, _| this.has_stored_credentials("")));
    assert!(cx.debug_bounds("file-auth-").is_some());
    files.update(cx, |this, cx| {
        let mut entry = this.models.file.as_ref().unwrap().entry("").unwrap();
        entry
            .set(&["baseUrl".into()], Some(serde_json::json!("unsaved")))
            .unwrap();
        this.models.edit("".into(), Some(entry));
        cx.notify();
    });
    cx.update(|window, cx| window.click("file-menu-", cx));
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert!(window.has_active_dialog(cx));
        window.click("file-cancel", cx);
    });
    cx.run_until_parked();
    settings.read_with(cx, |this, _| {
        assert_eq!(this.navigation.1, super::super::layout::MODELS_PAGE)
    });
    assert!(files.read_with(cx, |this, _| this.models.dirty()));
    cx.update(|window, cx| window.click("file-menu-", cx));
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    cx.update(|window, cx| window.click("file-discard", cx));
    cx.run_until_parked();
    settings.read_with(cx, |this, _| {
        assert_eq!(this.navigation.1, super::super::layout::AUTH_PAGE)
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert!(!window.has_active_dialog(cx));
        window.click("auth-menu-", cx);
    });
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    settings.read_with(cx, |this, _| {
        assert_eq!(this.navigation.1, super::super::layout::MODELS_PAGE)
    });
    cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    assert!(!files.read_with(cx, |this, _| this.models.dirty()));
    assert_eq!(
        std::fs::read_to_string(&models).unwrap(),
        r#"{"providers":{"":{}}}"#
    );
}
