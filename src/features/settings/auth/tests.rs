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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |this, cx| this.render_page(cx))
    }
}
#[gpui_kit::test]
fn editor_masks_new_keys_and_saves_without_exposing_existing_values(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        i18n::apply(gupi_settings::config::AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    std::fs::write(
        &path,
        r#"{"p":{"type":"api_key","key":"fixture-original","env":{"ID":"fixture-env"}}}"#,
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
            this.names = vec!["p".into()];
        });
        owner = Some(auth.clone());
        let host = cx.new(|_| Host(auth));
        Root::new(host, window, cx)
    });
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("auth-row-p", cx);
        window.render_frame(cx);
        window.click("auth-edit", cx);
        window.render_frame(cx);
        assert_eq!(window.find("auth-key").value(), None);
        window.click("auth-save", cx);
        assert!(window.has_active_dialog(cx));
        window.click("auth-key", cx);
        window.input("fixture-new", cx);
        window.render_frame(cx);
        assert_eq!(window.find("auth-key").value(), None);
        window.click("auth-save", cx);
    });
    cx.run_until_parked();
    let root: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(root["p"]["key"], "fixture-new");
    assert_eq!(root["p"]["env"]["ID"], "fixture-env");
    let owner = owner.unwrap();
    assert!(owner.read_with(cx, |this, _| this.error.is_none()));
}
