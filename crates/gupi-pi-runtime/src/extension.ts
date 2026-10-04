import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

// Loaded only for Gupi-owned connections; never installed into Pi's user config.
export default function (pi: ExtensionAPI) {
  pi.registerCommand("gupi-continue", {
    description: "Continue from a history node in the current session",
    handler: async (args, ctx) => {
      const targetId = args.trim();
      const entry = ctx.sessionManager.getEntry(targetId);
      if (!entry) throw new Error(`Entry ${targetId} not found`);
      const previousLeaf = ctx.sessionManager.getLeafId();
      const result = await ctx.navigateTree(targetId, { summarize: false });
      if (result.cancelled || targetId === previousLeaf) return;
      // RPC's command context does not return navigateTree's editorText.
      // Reproduce Pi's public tree navigation editor semantics through its UI API.
      const content = entry.type === "message" && entry.message.role === "user"
        ? entry.message.content
        : entry.type === "custom_message" ? entry.content : undefined;
      const text = typeof content === "string" ? content : (content ?? [])
        .filter((part) => part.type === "text")
        .map((part) => part.text).join("");
      ctx.ui.setEditorText(text);
    },
  });
}
