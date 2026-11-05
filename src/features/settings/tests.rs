use super::AppConfig;
use super::AppLanguage;
use super::ConfigController;
use super::PiProbeController;
use super::SettingsView;
use gpui_form::Form;
use gpui_kit::AppContext;
use gpui_kit::Focusable as _;
use gpui_kit::TestAppContext;
use gpui_kit::component::Root;

#[gpui_kit::test]
fn theme_grid_contributes_height_and_wraps_in_onboarding(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
    });
    let mut fixture = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| Form::new(AppConfig::default()));
        let controller = cx.new(|cx| ConfigController::new(&form, cx));
        let view = cx.new(|_| ThemeGridFixture {
            controller,
            width: 640.,
        });
        fixture = Some(view.clone());
        Root::new(view, window, cx)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let wide = cx.debug_bounds("theme-grid-measure").unwrap().size.height;
    let first = cx.debug_bounds("light-themes-tile-0").unwrap();
    let second = cx.debug_bounds("light-themes-tile-1").unwrap();
    let last = cx.debug_bounds("light-themes-tile-3").unwrap();
    assert_eq!(
        first.top(),
        second.top(),
        "first frame must already have multiple columns"
    );
    assert!(
        last.top() > first.top(),
        "fixture must include multiple rows"
    );
    assert!(
        (first.size.width - last.size.width).abs() < gpui_kit::px(1.),
        "last row must use the same column width: {first:?}, {last:?}"
    );
    let fixture = fixture.unwrap();
    fixture.update(cx, |view, cx| {
        view.width = 320.;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let narrow = cx.debug_bounds("theme-grid-measure").unwrap().size.height;
    assert!(
        wide > gpui_kit::px(64.),
        "theme rows must contribute to the parent height: {wide:?}"
    );
    assert!(
        narrow > wide,
        "narrow onboarding must wrap and grow vertically: {wide:?}, {narrow:?}"
    );
}
struct ThemeGridFixture {
    controller: gpui_kit::Entity<ConfigController>,
    width: f32,
}
impl gpui_kit::Render for ThemeGridFixture {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        use gpui_kit::InteractiveElement;
        use gpui_kit::ParentElement;
        use gpui_kit::Styled;
        let mut choices = app_theme::theme_choices(
            gpui_kit::component::ThemeRegistry::global(cx),
            gpui_kit::component::ThemeMode::Light,
            &[],
        );
        choices.truncate(4);
        gpui_kit::div()
            .w(gpui_kit::px(self.width))
            .debug_selector(|| "theme-grid-measure".into())
            .child(super::preferences::theme_grid(
                (
                    "light-themes",
                    gpui_kit::component::ThemeMode::Light,
                    choices,
                ),
                &AppConfig::default(),
                &self.controller,
                false,
            ))
    }
}

#[gpui_kit::test]
fn every_settings_page_renders_with_resources_at_narrow_width(cx: &mut TestAppContext) {
    use gupi_resources::pi_resources::Catalog;
    use gupi_resources::pi_resources::Kind;
    use gupi_resources::pi_resources::Package;
    use gupi_resources::pi_resources::Resource;

    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
    });
    let agent = tempfile::tempdir().unwrap();
    let pages = super::layout::DOMAIN_PAGES
        .into_iter()
        .flat_map(|(domain, count)| (0..count).map(move |page| (domain, page)));
    for (domain, page) in pages {
        let (_, window_cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| {
                let mut record = AppConfig::default();
                record.pi_command = Some("/custom/onboarding-pi".into());
                Form::new(record)
            });
            let controller = cx.new(|cx| ConfigController::new(&form, cx));
            controller
                .read(cx)
                .pi_form()
                .clone()
                .update(cx, |form, cx| {
                    let mut record = super::PiSettings::default();
                    record.command = Some("/custom/settings-pi".into());
                    form.rebase(record, gpui_form::DraftPolicy::Discard, cx)
                        .expect("initial test settings");
                });
            let draft = cx.new(|_| PiProbeController::new());
            let applied = cx.new(|_| {
                let mut probe = PiProbeController::new();
                probe.stop(); // Layout fixture: no external executable probing.
                probe
            });
            let settings = cx.new(|cx| {
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
            assert_eq!(
                settings.read(cx).input.read(cx).value().as_ref(),
                "/custom/onboarding-pi"
            );
            assert_eq!(
                settings.read(cx).pi_input.read(cx).value().as_ref(),
                "/custom/settings-pi"
            );
            settings.read(cx).auth.clone().update(cx, |auth, _| {
                auth.set_agent_for_test(agent.path().to_owned());
            });
            let config = settings.read(cx).pi_config.clone();
            config.update(cx, |config, cx| {
                config.set_agent_for_test(agent.path().to_owned(), cx)
            });
            let resources = settings.read(cx).resources.read(cx).controller.clone();
            resources.update(cx, |owner, _| {
                let root = std::path::PathBuf::from("/tmp/settings-layout-fixture");
                owner.set_catalog_for_test({
                    let mut record = Catalog::default();
                    record.root = root.clone();
                    record.packages = vec![{
                        let mut package = Package::new("local-review-package".into(), root.clone());
                        package.version = Some("1.0.0".into());
                        package
                    }];
                    record.resources = [Kind::Extension, Kind::Skill, Kind::Prompt]
                        .into_iter()
                        .map(|kind| {
                            let mut record = Resource::new(
                                kind,
                                root.join("example.md"),
                                root.clone(),
                                "测试资源".into(),
                            );
                            record.package = None;
                            record.description = "用于布局检查的说明".into();
                            record.enabled = true;
                            record.editable = kind != Kind::Extension;
                            record
                        })
                        .collect();
                    record.warnings = vec![];
                    record
                });
            });
            let view = cx.new(|_| SettingsPageFixture {
                settings,
                domain,
                page,
            });
            Root::new(view, window, cx)
        });
        window_cx.simulate_resize(gpui_kit::size(gpui_kit::px(760.), gpui_kit::px(640.)));
        window_cx.update(|window, cx| window.draw(cx).clear(cx));
        if (domain, page)
            == (
                super::layout::Domain::Gupi,
                super::layout::KEYBOARD_PAGE.index(),
            )
        {
            for width in [760., 1600.] {
                window_cx.simulate_resize(gpui_kit::size(gpui_kit::px(width), gpui_kit::px(1200.)));
                window_cx.update(|window, cx| window.draw(cx).clear(cx));
                let bound = window_cx.debug_bounds("key-binding-palette").unwrap();
                let unbound = window_cx.debug_bounds("key-binding-main").unwrap();
                assert!(
                    bound.left() >= gpui_kit::px(0.) && bound.right() <= gpui_kit::px(width),
                    "native setting field remains inside the window"
                );
                if width == 1600. {
                    assert_eq!(bound.left(), unbound.left());
                }
            }
        }
    }
}
struct SettingsPageFixture {
    settings: gpui_kit::Entity<SettingsView>,
    domain: super::layout::Domain,
    page: usize,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[gpui_kit::test]
fn native_update_disables_settings_controls_until_closed(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;
    use gupi_settings::config::ConfigContents;
    use gupi_settings::config::ConfigData;
    use gupi_updates::updates;
    use gupi_updates::updates::Status;

    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
    });
    let directory = tempfile::tempdir().unwrap();
    let mut fixture = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| {
            Form::new({
                let mut record = AppConfig::default();
                record.auto_check_updates = Some(false);
                record
            })
        });
        let controller = cx.new(|cx| {
            ConfigController::at_path(&form, Ok(directory.path().join("config.toml")), cx)
        });
        controller.update(cx, |owner, cx| {
            owner.settle_for_test(
                {
                    let mut record = ConfigData::new(
                        directory.path().join("config.toml"),
                        ConfigContents::Missing,
                    );
                    record.backup = None;
                    record
                },
                cx,
            )
        });
        let draft = cx.new(|_| PiProbeController::new());
        let applied = cx.new(|_| {
            let mut probe = PiProbeController::new();
            probe.stop();
            probe
        });
        let settings = cx.new(|cx| {
            SettingsView::new(
                form.clone(),
                controller.clone(),
                draft,
                applied,
                cx.focus_handle(),
                window,
                cx,
            )
        });
        fixture = Some((form, controller));
        let view = cx.new(|_| SettingsPageFixture {
            settings,
            domain: super::layout::Domain::Gupi,
            page: 4,
        });
        Root::new(view, window, cx)
    });
    visual.simulate_resize(gpui_kit::size(gpui_kit::px(1000.), gpui_kit::px(1200.)));
    let (form, controller) = fixture.unwrap();
    let updates = visual.update(|_, cx| updates::get(cx));
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("updates-automatic", cx);
        assert_eq!(AppConfig::AUTO_CHECK_UPDATES.get(&form, cx), Some(true));
    });
    updates.update(visual, |owner, cx| {
        owner.set_status_for_test(Status::Available(gupi_updates::releases::Release::new(
            semver::Version::new(2, 0, 0),
            "https://github.com/suxiaoshao/gupi/releases/tag/v2.0.0".into(),
        )));
        owner.start_install(cx).unwrap();
    });
    visual.update(|window, cx| {
        assert!(controller.read(cx).busy(cx));
        window.render_frame(cx);
        window.click("updates-automatic", cx);
        assert_eq!(AppConfig::AUTO_CHECK_UPDATES.get(&form, cx), Some(true));
        assert!(!controller.read(cx).is_running(cx));
    });
    updates.update(visual, |owner, cx| owner.finish_install(false, cx));
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("updates-automatic", cx);
        assert_eq!(AppConfig::AUTO_CHECK_UPDATES.get(&form, cx), Some(false));
    });
}
impl gpui_kit::Render for SettingsPageFixture {
    fn render(
        &mut self,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        self.settings.update(cx, |view, cx| {
            view.domain = self.domain;
            view.navigation = super::layout::PageId::pages(self.domain)[self.page];
            view.render_settings(window, cx)
        })
    }
}

#[gpui_kit::test]
fn empty_project_entry_keeps_the_current_settings_page(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
    });
    let mut settings = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| Form::new(AppConfig::default()));
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
        view.update(cx, |view, cx| {
            view.open_project_pi_settings(std::path::PathBuf::new(), window, cx);
        });
        settings = Some(view.clone());
        Root::new(view, window, cx)
    });
    cx.run_until_parked();
    let settings = settings.unwrap();
    cx.update(|_, cx| {
        let view = settings.read(cx);
        assert_eq!(view.domain, super::layout::Domain::Gupi);
        assert_eq!(view.navigation, super::layout::PageId::General);
        assert_eq!(
            view.pi_config.read(cx).scope(),
            &gupi_resources::pi_settings::Scope::Global
        );
    });
}

#[gpui_kit::test]
async fn auth_notifications_invalidate_settings(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let mut settings = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| Form::new(AppConfig::default()));
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
        let auth = view.read(cx).auth.clone();
        auth.update(cx, |auth, _| auth.set_agent_for_test(dir.path().to_owned()));
        settings = Some(view.clone());
        Root::new(view, window, cx)
    });
    let settings = settings.unwrap();
    let changes = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = changes.clone();
    let _observation =
        cx.update(|_, cx| cx.observe(&settings, move |_, _| count.set(count.get() + 1)));
    cx.run_until_parked();
    let before = changes.get();
    let auth = settings.read_with(cx, |settings, _| settings.auth.clone());
    // Model a deferred load/save/check completion without any user event or
    // explicit parent update/render, which would mask missing observation.
    cx.update(|_, cx| cx.defer(move |cx| auth.update(cx, |_, cx| cx.notify())));
    cx.run_until_parked();
    assert!(changes.get() > before);
}

#[gpui_kit::test]
fn language_selection_keeps_its_raw_value_and_accessible_name_across_locales(
    cx: &mut TestAppContext,
) {
    use gpui_kit::test::TestWindowExt as _;
    use gupi_settings::config::{ConfigContents, ConfigData};
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::English, cx);
    });
    let dir = tempfile::tempdir().unwrap();
    let mut owner = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let mut config = AppConfig::default();
        config.language = AppLanguage::English;
        let form = cx.new(|_| Form::new(config.clone()));
        let controller =
            cx.new(|cx| ConfigController::at_path(&form, Ok(dir.path().join("config.toml")), cx));
        controller.update(cx, |this, cx| {
            this.settle_for_test(
                ConfigData::new(
                    dir.path().join("config.toml"),
                    ConfigContents::Configured(config),
                ),
                cx,
            )
        });
        let draft = cx.new(|_| PiProbeController::new());
        let applied = cx.new(|_| {
            let mut probe = PiProbeController::new();
            probe.stop();
            probe
        });
        let settings = cx.new(|cx| {
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
        owner = Some(settings.clone());
        let host = cx.new(|_| SettingsPageFixture {
            settings,
            domain: super::layout::Domain::Gupi,
            page: 0,
        });
        Root::new(host, window, cx)
    });
    let owner = owner.unwrap();
    for locale in [
        AppLanguage::English,
        AppLanguage::German,
        AppLanguage::Chinese,
    ] {
        visual.update(|_, cx| gupi_settings::i18n::apply(locale, cx));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(
                window.find("settings-language").label(),
                Some(gupi_settings::i18n::t(cx, "settings-app-language").as_str())
            );
            assert_eq!(window.find("settings-language").value(), Some("English"));
            owner.read(cx).language.read_with(cx, |state, _| {
                assert_eq!(state.selected_value(), Some(&AppLanguage::English))
            });
        });
    }
}

/// Real settings shell with isolated paths; no configuration is loaded from the user.
pub(super) fn settings_shell<'a>(
    cx: &'a mut TestAppContext,
    agent: &std::path::Path,
) -> (
    gpui_kit::Entity<SettingsView>,
    &'a mut gpui_kit::VisualTestContext,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::English, cx);
    });
    let mut settings = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| Form::new(AppConfig::default()));
        let controller =
            cx.new(|cx| ConfigController::at_path(&form, Ok(agent.join("gupi.toml")), cx));
        let draft = cx.new(|_| PiProbeController::new());
        let applied = cx.new(|_| {
            use gpui_operation::Transition as _;
            let mut probe = PiProbeController::new();
            probe
                .operation_mut_for_test()
                .transition(gpui_operation::Settle(Ok(pi_rpc::probe::PiProbeData {
                    command: "/missing/pi".into(),
                    version: "1.5.0".into(),
                })));
            probe
        });
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
        view.update(cx, |view, cx| {
            view.pi_config.update(cx, |config, cx| {
                config.set_agent_for_test(agent.to_owned(), cx)
            })
        });
        settings = Some(view.clone());
        Root::new(view, window, cx)
    });
    visual.run_until_parked();
    (settings.unwrap(), visual)
}

pub(super) fn search_settings(
    owner: &gpui_kit::Entity<SettingsView>,
    query: &str,
    visual: &mut gpui_kit::VisualTestContext,
) {
    visual.update(|window, cx| {
        let input = owner.read(cx).search.clone();
        input.update(cx, |input, cx| {
            input.set_value(query, window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        });
    });
    visual.run_until_parked();
}

// Search must expose a recoverable empty state, and sidebar navigation must
// clear that state without saving, reloading or abandoning the current session.
#[gpui_kit::test]
fn settings_search_empty_clear_and_page_navigation_restore_content(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let agent = tempfile::tempdir().unwrap();
    let (owner, visual) = settings_shell(cx, agent.path());
    search_settings(&owner, "PREWARM", visual);
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("temporary-prewarm").is_some());
        assert!(window.try_find("reload-config").is_none());
    });
    search_settings(&owner, "no-setting-for-this-query", visual);
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("settings-clear-search").is_some());
        window.click("settings-clear-search", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let view = owner.read(cx);
        assert!(view.query.is_empty());
        assert!(view.search.focus_handle(cx).is_focused(window));
        assert!(window.try_find("reload-config").is_some());
    });
    search_settings(&owner, "no-setting-for-this-query", visual);
    visual.update(|window, cx| {
        window
            .within("settings-sidebar")
            .click("settings-nav-settings-appearance", cx)
    });
    visual.run_until_parked();
    owner.read_with(visual, |view, _| {
        assert!(view.query.is_empty());
        assert_eq!(view.navigation, super::layout::PageId::Appearance);
    });
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("theme-light").is_some());
        assert!(window.try_find("settings-clear-search").is_none());
    });
    visual.update(|window, cx| {
        window
            .within("settings-sidebar")
            .click("settings-nav-settings-keyboard", cx)
    });
    visual.run_until_parked();
    search_settings(&owner, "palette", visual);
    visual.update(|window, cx| window.render_frame(cx));
    assert!(visual.debug_bounds("key-binding-palette").is_some());
    assert!(visual.debug_bounds("key-binding-quick_open").is_none());
    assert!(!agent.path().join("gupi.toml").exists());
}
