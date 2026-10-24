//! Remote Markdown pictures stay explicit browser actions.
use super::*;
use gpui_kit::component::text::{MarkdownNode, MarkdownParseContext, MarkdownPlugin, markdown_ast};

#[derive(Clone)]
pub(super) struct RemoteImages;

impl MarkdownPlugin for RemoteImages {
    fn name(&self) -> &str {
        "gupi-preview-remote-image"
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        context: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Image(image) = node else {
            return None;
        };
        if !url::Url::parse(&image.url).is_ok_and(|url| matches!(url.scheme(), "http" | "https")) {
            return None;
        }
        Some(
            MarkdownNode::new(self.name(), image.url.clone())
                .text(image.alt.clone())
                .markdown(context.node_source(node)?.to_owned()),
        )
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = if node.as_text().is_empty() {
            t(cx, "files-open-remote-image").to_string()
        } else {
            node.as_text().to_owned()
        };
        let focus = cx.theme().accent;
        let offset = node.source_range().map_or(0, |r| r.start);
        let link = gpui_kit::base::Link::new(("preview-image-link", offset))
            .accessibility_label(label.clone())
            .child(label)
            .href(node.data::<String>().expect("remote image URL").clone())
            .open_with(|url, _, _, cx| cx.open_url(url))
            .text_color(cx.theme().link)
            .text_decoration_1()
            .cursor_pointer()
            .focus_visible(move |style| style.bg(focus));
        div()
            .id(format!("preview-image-{offset}"))
            .test_support()
            .min_w_0()
            .max_w_full()
            .child(link)
    }
}
