pub(crate) mod instance;
mod logging;
pub(crate) mod menus;
pub(crate) mod notifications;
#[cfg(feature = "performance")]
pub(crate) mod performance;
pub(crate) mod shortcuts;
pub(crate) mod temporary;
mod tray;
pub(crate) mod updater;
use crate::{
    features::startup::StartupView,
    foundation::{assets::Assets, i18n, paths},
    state::layout,
};
use gpui_kit::component::{Root, TitleBar};
use gpui_kit::*;
#[cfg(feature = "performance")]
use tracing_subscriber::prelude::*;
use window_ext::WindowExt;
struct MainWindow {
    window: WindowHandle<Root>,
    view: Entity<StartupView>,
}
impl Global for MainWindow {}
pub(crate) fn run() {
    let instance =
        match paths::config_dir().and_then(|directory| instance::Instance::acquire(&directory)) {
            Ok(Some(instance)) => Ok(instance),
            Ok(None) => return,
            Err(error) => Err(error),
        };
    let system_locale = sys_locale::get_locale();
    // AppKit resolves system dialog languages before the async configuration
    // controller loads. Read only the startup locale here; the controller still
    // owns configuration, recovery, editing and persistence.
    let initial_language = paths::config_dir()
        .ok()
        .and_then(|dir| crate::state::config::read_config(dir.join("config.toml"), false).ok())
        .and_then(|data| data.configured().map(|config| config.language))
        .unwrap_or_default();
    #[cfg(target_os = "macos")]
    platform_ext::app::set_application_language_override(i18n::native_locale(initial_language));
    let app = gpui_kit::application().with_assets(Assets::default());
    app.on_reopen(|cx| cx.defer(|cx| show(None, cx)));
    app.run(move |cx| {
        #[cfg(feature = "performance")]
        profiling::scope!("app.initialize");
        #[cfg(feature = "performance")]
        performance::install_quit_hook(cx);
        cx.set_app_identity("top.sushao.gupi", "Gupi");
        let instance = instance.and_then(|instance| instance.listen(cx));
        if let Err(error) = &instance {
            eprintln!("Gupi instance startup failed: {error}");
        }
        gpui_kit::init(cx);
        gpui_tokio::init(cx);
        crate::state::environment::init(cx);
        crate::state::pi::init(cx);
        temporary::init(cx);
        shortcuts::init(cx);
        app_theme::init(cx);
        crate::state::theme::init(cx);
        cx.set_global(i18n::SystemLocale(system_locale));
        i18n::apply(initial_language, cx);
        menus::init(cx);
        menus::refresh(cx);
        crate::state::keybindings::apply(&Default::default(), cx);
        cx.on_action(|_: &menus::ShowTemporaryWindow, cx| {
            cx.defer(|cx| {
                if instance::is_owner(cx) {
                    temporary::toggle(cx);
                } else {
                    show(None, cx);
                }
            })
        });
        cx.on_action(|_: &menus::ShowSettings, cx| cx.defer(|cx| show(Some(true), cx)));
        cx.on_action(|_: &menus::ShowMainWindow, cx| cx.defer(|cx| show(Some(false), cx)));
        cx.on_action(|_: &menus::Quit, cx| cx.defer(quit));
        let log_warning = instance.is_ok() && instance_ready(cx);
        let layout = paths::config_dir()
            .map(|directory| {
                let path = directory.join("state.toml");
                if instance.is_ok() {
                    layout::load(&path)
                } else {
                    // Recovery may reuse saved geometry, but must not discard
                    // invalid state belonging to another instance.
                    layout::read(&path).unwrap_or_default()
                }
            })
            .unwrap_or_default();
        cx.set_global(layout);
        let bounds = cx
            .global::<layout::LayoutState>()
            .main_window
            .map(|p| p.restored(cx))
            .unwrap_or_else(|| {
                WindowBounds::Windowed(Bounds::centered(None, layout::default_window_size(), cx))
            });
        let options = WindowOptions {
            window_bounds: Some(bounds),
            window_min_size: Some(layout::minimum_window_size()),
            titlebar: Some(TitlebarOptions {
                title: Some("Gupi".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.), px(16.))),
            }),
            ..TitleBar::window_options()
        };
        match gpui_kit::open_window(options, cx, |window, cx| {
            window.on_window_should_close(cx, |window, cx| {
                if cfg!(any(target_os = "macos", target_os = "windows")) {
                    match window.native_window_handle() {
                        Ok(handle) => cx.defer(move |_| {
                            if let Err(error) = handle.hide() {
                                tracing::error!(%error, "hide window failed");
                            } else {
                                tracing::info!("main window hidden");
                            }
                        }),
                        Err(error) => tracing::error!(%error, "get native window failed"),
                    }
                } else {
                    cx.defer(quit);
                }
                false
            });
            cx.new(|cx| StartupView::new(instance, log_warning, window, cx))
        }) {
            Ok((window, view)) => {
                cx.set_global(MainWindow {
                    window: window.downcast::<Root>().expect("root window"),
                    view,
                });
                cx.activate(true);
            }
            Err(error) => {
                tracing::error!(%error, "open window failed");
                cx.quit();
            }
        }
    });
}
/// These services may touch shared state and start conversations. Start them
/// only after owning the configuration directory, including after a retry.
pub(crate) fn instance_ready(cx: &mut App) -> bool {
    let log = paths::log_dir().and_then(logging::LogWriter::open);
    let log_warning = log.is_err();
    match log {
        Ok(file) => {
            #[cfg(feature = "performance")]
            if performance::is_recording() {
                performance::set_log_writer(file);
            } else {
                let _ = tracing_subscriber::registry()
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(std::sync::Mutex::new(file))
                            .with_filter(tracing_subscriber::filter::filter_fn(
                                performance::diagnostic_metadata,
                            )),
                    )
                    .try_init();
            }
            #[cfg(not(feature = "performance"))]
            let _ = tracing_subscriber::fmt()
                .with_writer(std::sync::Mutex::new(file))
                .try_init();
        }
        Err(error) => {
            eprintln!("Gupi log initialization: {error}");
            let _ = tracing_subscriber::fmt().try_init();
        }
    }
    notifications::init(cx);
    tray::init(cx);
    updater::init(cx);
    log_warning
}
fn quit(cx: &mut App) {
    quit_then(cx, |cx| cx.quit());
}
fn quit_then(cx: &mut App, finish: impl FnOnce(&mut App) + 'static) {
    let main = cx
        .try_global::<MainWindow>()
        .map(|m| (m.window, m.view.clone()));
    if let Some((window, view)) = main {
        if let Err(error) = window.update(cx, |_, window, cx| {
            view.update(cx, |view, cx| view.quit_then(window, cx, finish))
        }) {
            tracing::error!(%error, "start managed quit failed");
        }
    } else {
        cx.quit();
    }
}
fn check_for_updates(cx: &mut App) {
    show(None, cx);
    if let Some(main) = cx.try_global::<MainWindow>() {
        // Dialog helpers access Root themselves; do not lease it through a typed update.
        let window: AnyWindowHandle = main.window.into();
        if main.view.read(cx).is_quitting() {
            return;
        }
        let config = main.view.read(cx).config.clone();
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                crate::features::updates::open(true, config, window, cx);
            });
        });
    }
}
fn show(settings: Option<bool>, cx: &mut App) {
    let main = cx
        .try_global::<MainWindow>()
        .map(|m| (m.window, m.view.clone()));
    if let Some((window, view)) = main {
        let mut native = None;
        if let Err(error) = window.update(cx, |_, window, cx| {
            if !window.is_window_active() {
                native = window.native_window_handle().ok();
            }
            view.update(cx, |view, cx| {
                if !view.is_quitting()
                    && let Some(settings) = settings
                {
                    view.set_settings_visible(settings, window, cx);
                }
                cx.notify();
            });
        }) {
            tracing::error!(%error, "update main window failed");
        }
        if let Some(native) = native {
            // Native focus notifications can synchronously re-enter GPUI.
            cx.defer(move |_| {
                if let Err(error) = native.show() {
                    tracing::error!(%error, "show window failed");
                } else {
                    tracing::info!("main window shown");
                }
            });
        }
    }
}

#[cfg(test)]
mod update_tests;
