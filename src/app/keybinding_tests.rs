use gpui_kit::TestAppContext;
use gupi_settings::keybindings::Overrides;
use gupi_settings::keybindings::apply;
#[gpui_kit::test]
fn binding_changes_rebuild_native_menus_without_a_locale_change(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        crate::app::menus::refresh(cx);

        // The test platform does not materialize native accelerators. Clear
        // its menu snapshot to observe whether applying bindings rebuilds it.
        for overrides in [
            Overrides::new(),
            Overrides::from([("quit".into(), "cmd-alt-q".into())]),
            Overrides::from([("quit".into(), String::new())]),
            Overrides::new(),
        ] {
            cx.set_menus(Vec::new());
            apply(&overrides, cx);
            assert_eq!(cx.get_menus().unwrap().len(), 6);

            cx.set_menus(Vec::new());
            apply(&overrides, cx);
            crate::app::menus::refresh(cx);
            assert!(cx.get_menus().unwrap().is_empty());
        }
    });
}
