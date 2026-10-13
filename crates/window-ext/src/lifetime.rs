//! App-scoped witnesses; native calls read no GPUI context.
use crate::WindowExtError;
use gpui::{App, Global, Subscription, WindowId};
use std::{cell::Cell, collections::HashMap, rc::Rc};

#[derive(Clone, Debug)]
pub(super) struct Lifetime(Rc<Cell<bool>>);
impl Lifetime {
    pub(super) fn for_window(id: WindowId, cx: &mut App) -> Result<Self, WindowExtError> {
        if !cx.has_global::<Registry>() {
            let closed = cx.on_window_closed(|cx, id| {
                if cx.has_global::<Registry>()
                    && let Some(lifetime) = cx.global_mut::<Registry>().windows.remove(&id)
                {
                    lifetime.0.set(false);
                }
            });
            let quit = cx.on_app_quit(|cx| {
                // Synchronous: shutdown clears windows BEFORE it polls this future.
                if cx.has_global::<Registry>() {
                    cx.global_mut::<Registry>().invalidate();
                }
                std::future::ready(())
            });
            cx.set_global(Registry {
                windows: HashMap::new(),
                shutting_down: false,
                _subscriptions: vec![closed, quit],
            });
        }
        let registry = cx.global_mut::<Registry>();
        if registry.shutting_down {
            return Err(WindowExtError::WindowClosed);
        }
        Ok(registry
            .windows
            .entry(id)
            .or_insert_with(|| Self(Rc::new(Cell::new(true))))
            .clone())
    }
    pub(super) fn check(&self) -> Result<(), WindowExtError> {
        if self.0.get() {
            Ok(())
        } else {
            Err(WindowExtError::WindowClosed)
        }
    }
}
struct Registry {
    windows: HashMap<WindowId, Lifetime>,
    shutting_down: bool,
    _subscriptions: Vec<Subscription>,
}
impl Global for Registry {}
impl Registry {
    fn invalidate(&mut self) {
        self.shutting_down = true;
        for (_, lifetime) in self.windows.drain() {
            lifetime.0.set(false);
        }
    }
}
impl Drop for Registry {
    fn drop(&mut self) {
        self.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::{Lifetime, Registry};
    use crate::WindowExtError;
    use gpui::{Context, IntoElement, Render, TestAppContext, Window, div};

    struct Empty;
    impl Render for Empty {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    #[gpui::test]
    fn removal_invalidates_shared_gate_and_replacement_has_its_own_gate(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_quit_mode(gpui::QuitMode::Explicit));
        let window = cx.add_window(|_, _| Empty);
        let first = window
            .update(cx, |_, window, cx| {
                Lifetime::for_window(window.window_handle().window_id(), cx).unwrap()
            })
            .unwrap();
        let second = window
            .update(cx, |_, window, cx| {
                Lifetime::for_window(window.window_handle().window_id(), cx).unwrap()
            })
            .unwrap();
        assert!(std::rc::Rc::ptr_eq(&first.0, &second.0));
        // Visibility does not affect the GPUI lifetime gate.
        assert!(first.check().is_ok());
        window
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
        assert!(matches!(first.check(), Err(WindowExtError::WindowClosed)));
        assert!(matches!(second.check(), Err(WindowExtError::WindowClosed)));
        let new_window = cx.add_window(|_, _| Empty);
        let next = new_window
            .update(cx, |_, window, cx| {
                Lifetime::for_window(window.window_handle().window_id(), cx).unwrap()
            })
            .unwrap();
        assert!(!std::rc::Rc::ptr_eq(&first.0, &next.0));
        assert!(next.check().is_ok());
    }
    #[gpui::test]
    fn app_shutdown_and_registry_teardown_invalidate_outstanding_gates(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_quit_mode(gpui::QuitMode::Explicit));
        let window = cx.add_window(|_, _| Empty);
        let gate = window
            .update(cx, |_, window, cx| {
                Lifetime::for_window(window.window_handle().window_id(), cx).unwrap()
            })
            .unwrap();
        cx.update(|cx| cx.shutdown());
        assert!(matches!(gate.check(), Err(WindowExtError::WindowClosed)));
        // Teardown independently rejects outstanding gates without close events.
        cx.update(|cx| drop(cx.remove_global::<Registry>()));
        let next = cx.add_window(|_, _| Empty);
        let gate = next
            .update(cx, |_, window, cx| {
                Lifetime::for_window(window.window_handle().window_id(), cx).unwrap()
            })
            .unwrap();
        cx.update(|cx| drop(cx.remove_global::<Registry>()));
        assert!(matches!(gate.check(), Err(WindowExtError::WindowClosed)));
    }
}
