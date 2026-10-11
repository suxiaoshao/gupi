use super::StartupScreen;
use super::StartupView;
use crate::app::temporary;
use gpui_kit::AppContext;
use gpui_kit::Task;
use gpui_kit::TestAppContext;
use gpui_kit::component::Root;
use gpui_operation::Retry;
use gpui_operation::Settle;
use gpui_operation::Transition;
use gupi_settings::config::AppConfig;
use gupi_settings::config::ConfigContents;
use gupi_settings::config::ConfigData;

#[gpui_kit::test]
fn configured_windows_and_sessions_do_not_wait_for_a_probe(cx: &mut TestAppContext) {
    init(cx);
    let mut startup = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| StartupView::new(Ok(()), false, window, cx));
        // Replace the pending filesystem read before it runs; this test never
        // reads or writes the user's configuration.
        view.read(cx).config.clone().update(cx, |owner, cx| {
            owner.settle_for_test(
                {
                    let mut record = ConfigData::new(
                        "unused-config.toml".into(),
                        ConfigContents::Configured({
                            let mut record = AppConfig::default();
                            record.pi_command = Some("missing-pi".into());
                            record
                        }),
                    );
                    record.backup = None;
                    record
                },
                cx,
            )
        });
        startup = Some(view.clone());
        // This tests screen selection and probe ownership, not Home rendering.
        // Rendering Home would start real workspace I/O outside the deterministic
        // scheduler and read the user's history during a unit test.
        let host = cx.new(|_| ProbeTestHost);
        Root::new(host, window, cx)
    });
    let startup = startup.unwrap();
    startup.update(visual, |view, cx| {
        assert!(matches!(view.screen(cx), StartupScreen::Home(command) if command == std::path::Path::new("missing-pi")));
        assert!(!view.applied_pi.read(cx).is_running());
        assert!(view.applied_pi.read(cx).data().is_none());
        assert!(temporary::state(cx).is_some(), "template actions can create their session immediately");
        view.applied_pi.update(cx, |pi, _| {
            pi.operation_mut_for_test().transition(Settle(Err(pi_rpc::probe::ProbeFailure::InvalidVersion)));
        });
        assert!(matches!(view.screen(cx), StartupScreen::Home(_)));
        view.applied_pi.update(cx, |pi, _| {
            pi.operation_mut_for_test().transition(Retry(Task::ready(())));
        });
        assert!(matches!(view.screen(cx), StartupScreen::Home(_)));
        view.applied_pi.update(cx, |pi, _| pi.stop());
    });
}

#[gpui_kit::test]
fn instance_failure_stays_in_recovery_without_loading_configuration(cx: &mut TestAppContext) {
    init(cx);
    cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            StartupView::new(
                Err(std::io::ErrorKind::PermissionDenied.into()),
                false,
                window,
                cx,
            )
        });
        view.update(cx, |view, cx| {
            assert!(matches!(view.screen(cx), StartupScreen::InstanceFailure(_)));
            view.config.read(cx).configuration().read(cx, |op| {
                assert!(!op.is_running());
                assert!(op.data().is_none());
            });
            assert!(cx.global::<temporary::Temporary>().config.is_none());
            assert!(temporary::state(cx).is_none());
            // The always-available Settings menu must not bypass recovery.
            view.set_settings_visible(true, window, cx);
            assert!(matches!(view.screen(cx), StartupScreen::InstanceFailure(_)));
            assert!(view.home.is_none());
        });
        Root::new(view, window, cx)
    });
}

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        gpui_tokio::init(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gupi_pi_runtime::init(cx);
        temporary::init(cx);
        cx.set_global(gupi_settings::layout::LayoutState::default());
    });
}

#[gpui_kit::test]
fn quit_checks_pi_drafts_before_teardown_even_inside_a_root_update(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt as _;
    for saving in [false, true] {
        init(cx);
        let mut startup = None;
        let (root, visual) = cx.add_window_view(|window, cx| {
            // Recovery construction avoids reading any personal config. Clear
            // the error afterwards to exercise the normal managed quit route.
            let view = cx.new(|cx| {
                StartupView::new(
                    Err(std::io::ErrorKind::PermissionDenied.into()),
                    false,
                    window,
                    cx,
                )
            });
            view.update(cx, |view, cx| {
                view.instance_error = None;
                view.settings.update(cx, |settings, cx| {
                    settings.seed_quit_draft_for_test(saving, cx);
                });
            });
            startup = Some(view);
            Root::new(cx.new(|_| ProbeTestHost), window, cx)
        });
        let startup = startup.unwrap();
        root.update_in(visual, |_, window, cx| {
            startup.update(cx, |view, cx| {
                view.quit_then(window, cx, |_| panic!("quit must remain blocked"));
                assert!(!view.is_quitting());
            });
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(window.has_active_dialog(cx), !saving);
            assert!(!startup.read(cx).is_quitting());
            let settings = startup.read(cx).settings.clone();
            assert!(settings.read(cx).has_unsaved_pi(cx));
            assert_eq!(settings.read(cx).quit_waits_for_save_for_test(cx), saving);
            if !saving {
                // Cancel closes the prompt without starting teardown.
                window.close_dialog(cx);
                assert!(!startup.read(cx).is_quitting());
                assert!(settings.read(cx).has_unsaved_pi(cx));
            }
        });
    }
}

struct ProbeTestHost;
impl gpui_kit::Render for ProbeTestHost {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        gpui_kit::div()
    }
}
