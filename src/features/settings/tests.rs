use super::AppConfig;
use super::AppLanguage;
use super::ConfigController;
use super::PiProbeController;
use super::SettingsView;
use gpui_form::Form;
use gpui_kit::AppContext;
use gpui_kit::TestAppContext;
use gpui_kit::component::Root;

#[gpui_kit::test]
fn shared_settings_layout_renders_without_reentrant_entity_access(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|_| {
            Form::new({
                let mut record = AppConfig::default();
                record.pi_command = Some("/custom/onboarding-pi".into());
                record
            })
        });
        let controller = cx.new(|cx| ConfigController::new(&form, cx));
        controller
            .read(cx)
            .pi_form()
            .clone()
            .update(cx, |form, cx| {
                form.rebase(
                    {
                        let mut record = super::PiSettings::default();
                        record.command = Some("/custom/settings-pi".into());
                        record
                    },
                    cx,
                )
            });
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
        assert_eq!(
            view.read(cx).input.read(cx).value().as_ref(),
            "/custom/onboarding-pi"
        );
        assert_eq!(
            view.read(cx).pi_input.read(cx).value().as_ref(),
            "/custom/settings-pi"
        );
        Root::new(view, window, cx)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui_kit::test]
fn theme_grid_contributes_height_and_wraps_in_settings(cx: &mut TestAppContext) {
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
        "narrow settings must wrap and grow vertically: {wide:?}, {narrow:?}"
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
    let pages = super::layout::DOMAIN_PAGES
        .into_iter()
        .flat_map(|(domain, count)| (0..count).map(move |page| (domain, page)));
    for (domain, page) in pages {
        let (_, window_cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let controller = cx.new(|cx| ConfigController::new(&form, cx));
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
        if (domain, page) == (super::layout::Domain::Gupi, 0) {
            for width in [640., 1600.] {
                window_cx.simulate_resize(gpui_kit::size(gpui_kit::px(width), gpui_kit::px(640.)));
                window_cx.update(|window, cx| window.draw(cx).clear(cx));
                let path = window_cx.debug_bounds("settings-config-path").unwrap();
                let reload = window_cx.debug_bounds("settings-config-reload").unwrap();
                let open = window_cx.debug_bounds("settings-config-open").unwrap();
                assert_eq!(reload.center().y, open.center().y);
                assert!(
                    reload.top() >= path.bottom(),
                    "labeled actions follow the path"
                );
                assert!(reload.right() <= open.left());
                assert!(open.right() <= gpui_kit::px(width));
                if width == 1600. {
                    assert!(
                        path.left() > gpui_kit::px(width / 2.),
                        "configuration controls belong to the right-hand setting field"
                    );
                }
            }
        }
        if (domain, page) == (super::layout::Domain::Gupi, super::layout::KEYBOARD_PAGE) {
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
                    assert_eq!(bound.right(), unbound.right());
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
        _: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        let domain = self.domain;
        self.settings.update(cx, |view, cx| {
            view.domain = domain;
            view.settings_panel(cx).default_selected_index(
                gpui_kit::component::setting::SelectIndex {
                    page_ix: self.page,
                    group_ix: None,
                },
            )
        })
    }
}
