use gpui_kit::*;
use serde::Deserialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Kind {
    TemporaryActions,
    PasteAnswer,
    CopyTemporaryAnswer,
    RevealWorkspace,
    HideTemporary,
    TrashTemporary,
    TemporarySession(u8),
    Palette,
    QuickOpen,
    New,
    Settings,
    Sidebar,
    Scan,
    FocusInput,
    ShowMain,
    Quit,
    Model,
    History,
    OpenHistory,
    ProjectFiles,
    CloseSource,
    FocusSource,
    ShowConversation,
    Export,
    Clone,
    CopyLastAnswer,
    Compact,
    Reconnect,
    Rename,
    Stop,
    Close,
    Reveal,
    CopyPath,
    Delete,
    SessionInfo,
    Find,
}
impl Kind {
    pub fn temporary_only(self) -> bool {
        matches!(
            self,
            Self::TemporaryActions
                | Self::PasteAnswer
                | Self::CopyTemporaryAnswer
                | Self::RevealWorkspace
                | Self::HideTemporary
                | Self::TrashTemporary
                | Self::TemporarySession(_)
        )
    }
    pub fn search_terms(self) -> &'static str {
        match self {
            Self::TemporaryActions => "actions temporary 操作",
            Self::PasteAnswer => "paste answer 粘贴 回填",
            Self::CopyTemporaryAnswer => "copy answer 复制",
            Self::RevealWorkspace => "reveal workspace 工作目录 定位",
            Self::TrashTemporary => "delete trash temporary 移到废纸篓 删除",
            Self::HideTemporary => "hide temporary 隐藏",
            Self::TemporarySession(_) => "switch temporary conversation 切换临时会话",
            Self::Palette => "command palette",
            Self::QuickOpen => "resume search sessions quick open",
            Self::New => "new conversation",
            Self::Settings => "settings preferences",
            Self::Sidebar => "sidebar",
            Self::Scan => "scan refresh session directory",
            Self::FocusInput => "focus input composer",
            Self::ShowMain => "show main window",
            Self::Quit => "quit exit",
            Self::Model => "model thinking",
            Self::CloseSource => "close source file 关闭 文件",
            Self::ShowConversation => "show conversation 会话",
            Self::FocusSource => "focus source file 聚焦 源码",
            Self::ProjectFiles => "project files directory 文件 项目 目录",
            Self::History | Self::OpenHistory => "tree fork history",
            Self::Reconnect => "reload reconnect",
            Self::Rename => "name rename",
            Self::Export => "export html",
            Self::Clone => "clone duplicate conversation",
            Self::CopyLastAnswer => "copy last assistant answer",
            Self::Compact => "compact context",
            Self::Stop => "stop abort hide 停止 隐藏",
            Self::Close => "close connection",
            Self::Reveal => "reveal locate file",
            Self::CopyPath => "copy path",
            Self::Delete => "delete trash",
            Self::Find => "find search text 查找 正文",
            Self::SessionInfo => "session information info details 会话信息",
        }
    }
}
#[derive(Clone, PartialEq, Deserialize, Action)]
#[action(namespace = gupi, no_json)]
#[non_exhaustive]
pub struct Run(pub Kind);
impl Run {
    pub fn new(value: Kind) -> Self {
        Self(value)
    }
}

actions!(
    gupi,
    [
        Quit,
        ShowSettings,
        ShowMainWindow,
        ShowCommandPalette,
        ShowTemporaryWindow,
        About,
        CheckForUpdates,
        Minimize,
        Zoom,
        Fullscreen,
        Hide,
        HideOthers,
        ShowAll,
        UserGuide,
        PiDocs,
        ReportIssue,
        ShowLogs,
        CopyDiagnostics
    ]
);
/// Signals only menu definition changes, not unrelated component global state.
pub struct MenusChanged;
impl Global for MenusChanged {}

pub const CONVERSATION_COMMANDS: [Kind; 6] = [
    Kind::New,
    Kind::QuickOpen,
    Kind::Rename,
    Kind::Export,
    Kind::Sidebar,
    Kind::History,
];
#[derive(Default, PartialEq)]
#[non_exhaustive]
pub struct ConversationCommands(pub [bool; 6]);
impl ConversationCommands {
    pub fn new(value: [bool; 6]) -> Self {
        Self(value)
    }
}

impl Global for ConversationCommands {}
/// Native menus capture enabled state. Only rebuild when the active window's
/// capabilities change, not for each streamed message or render.
pub fn conversation_commands(enabled: [bool; 6], window: &Window, cx: &mut App) {
    if !window.is_window_active() {
        return;
    }
    let value = ConversationCommands(enabled);
    if cx.try_global::<ConversationCommands>() == Some(&value) {
        return;
    }
    cx.set_global(value);
    crate::host::refresh_menus(cx);
}
