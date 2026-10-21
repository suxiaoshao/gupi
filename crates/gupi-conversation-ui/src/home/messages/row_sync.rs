//! Keep the native list's reading row alive across transcript projections.
use gpui_kit::Context;
use gpui_kit::component::message_scroller::MessageScrollerState;

pub(in crate::home) fn sync(
    state: &mut MessageScrollerState,
    old: &[&str],
    new: &[&str],
    cx: &mut Context<MessageScrollerState>,
) {
    let top = state.logical_scroll_top();
    let survivor = (!state.is_following_tail())
        .then(|| old.get(top.item_ix))
        .flatten()
        .and_then(|id| new.iter().position(|candidate| candidate == id));
    if let Some(index) = survivor {
        // Splice around, rather than through, the surviving row. The native
        // list then retains both its identity and offset inside that row.
        if old[top.item_ix + 1..] != new[index + 1..] {
            state.splice(top.item_ix + 1..old.len(), new.len() - index - 1, cx);
        }
        if old[..top.item_ix] != new[..index] {
            state.splice(0..top.item_ix, index, cx);
        }
    } else {
        // A deleted reading row uses the native splice's replacement anchor.
        // The tail sentinel (item_count) is not a row and follows this path too.
        let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        if prefix < old.len() || new.len() != old.len() {
            state.splice(prefix..old.len(), new.len() - prefix, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    use gpui_kit::component::message_scroller::MessageScroller;
    use gpui_kit::*;

    struct Harness(Entity<MessageScrollerState>);
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size(px(400.)).child(
                MessageScroller::new("anchor-messages", self.0.clone(), |_, _, _| {
                    div().h(px(100.))
                })
                .scrollbar(false)
                .jump_button(false),
            )
        }
    }

    #[gpui_kit::test]
    fn projection_preserves_surviving_row_and_nonzero_offset(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let state = cx.new(|cx| MessageScrollerState::new(20, cx));
        let (_, visual) = cx.add_window_view({
            let state = state.clone();
            move |_, _| Harness(state)
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.draw(cx).clear(cx);
            state.update(cx, |state, cx| state.scroll_to_item(5, cx));
            window.draw(cx).clear(cx);
        });
        visual.simulate_event(ScrollWheelEvent {
            position: point(px(200.), px(200.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-17.))),
            ..Default::default()
        });
        visual.update(|_, cx| {
            let before = state.read(cx).logical_scroll_top();
            assert_eq!(before.item_ix, 5);
            assert!(before.offset_in_item > px(0.));
            let old = (0..20).map(|i| i.to_string()).collect::<Vec<_>>();
            let mut new = old.clone();
            new.insert(0, "new".into());
            new[10] = "replaced".into();
            state.update(cx, |state, cx| {
                sync(
                    state,
                    &old.iter().map(String::as_str).collect::<Vec<_>>(),
                    &new.iter().map(String::as_str).collect::<Vec<_>>(),
                    cx,
                );
                let after = state.logical_scroll_top();
                assert_eq!(after.item_ix, 6);
                assert_eq!(after.offset_in_item, before.offset_in_item);
            });
        });
    }

    #[gpui_kit::test]
    fn deleted_reading_row_uses_native_replacement(cx: &mut TestAppContext) {
        let state = cx.new(|cx| MessageScrollerState::new(4, cx));
        state.update(cx, |state, cx| {
            state.scroll_to_item(1, cx);
            sync(state, &["a", "b", "c", "d"], &["a", "c", "d"], cx);
            assert_eq!(state.item_count(), 3);
            assert!(state.logical_scroll_top().item_ix < state.item_count());
            assert!(!state.is_following_tail());
        });
    }

    #[gpui_kit::test]
    fn tail_sentinel_is_not_treated_as_a_reading_row(cx: &mut TestAppContext) {
        let state = cx.new(|cx| MessageScrollerState::new(3, cx));
        state.update(cx, |state, cx| {
            state.scroll_to_end(cx);
            assert_eq!(state.logical_scroll_top().item_ix, state.item_count());
            sync(state, &["a", "b", "c"], &["x", "a", "b", "c", "d"], cx);
            assert_eq!(state.item_count(), 5);
            assert!(state.is_following_tail());
        });
    }
}
