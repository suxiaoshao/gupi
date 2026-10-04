use super::MainWindow;
use super::check_for_updates;
use crate::features::startup::StartupView;
use gpui_kit::AppContext;
use gpui_kit::Task;
use gpui_kit::TestAppContext;
use gpui_kit::component::Root;
use gpui_kit::component::WindowExt;
use gupi_updates::updates;

#[gpui_kit::test]
fn update_menu_opens_a_dialog_without_leasing_root_twice(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::init_capability_hosts(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gupi_pi_runtime::init(cx);
        super::temporary::init(cx);
        cx.set_global(gupi_settings::layout::LayoutState::default());
        // Reuse an in-flight check to keep this UI regression independent of network I/O.
        updates::get(cx).update(cx, |owner, _| {
            owner.set_status_for_test(updates::Status::Checking {
                _task: Task::ready(()),
                manual: false,
            });
        });
    });
    let mut startup = None;
    let window = cx.add_window(|window, cx| {
        let view = cx.new(|cx| {
            StartupView::new(
                Err(std::io::ErrorKind::PermissionDenied.into()),
                false,
                window,
                cx,
            )
        });
        startup = Some(view.clone());
        Root::new(view, window, cx)
    });
    cx.update(|cx| {
        cx.set_global(MainWindow {
            window,
            view: startup.unwrap(),
        });
        check_for_updates(cx);
    });
    cx.run_until_parked();
    let window: gpui_kit::AnyWindowHandle = window.into();
    window
        .update(cx, |_, window, cx| {
            assert!(window.has_active_dialog(cx));
            assert!(matches!(
                updates::get(cx).read(cx).status(),
                updates::Status::Checking { manual: true, .. }
            ));
        })
        .unwrap();
}
