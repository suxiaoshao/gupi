use gupi_conversation::tool_presentation::ToolKind;
use gupi_settings::assets::IconName;
pub(crate) trait ToolIcon {
    fn icon(self) -> IconName;
}
impl ToolIcon for ToolKind {
    fn icon(self) -> IconName {
        match self {
            Self::Read => IconName::FileText,
            Self::Skill => IconName::BookOpen,
            Self::Write => IconName::FilePlus,
            Self::Edit => IconName::FilePenLine,
            Self::Shell => IconName::Terminal,
            Self::Search => IconName::Search,
            Self::List => IconName::Folder,
            Self::Other => IconName::Wrench,
        }
    }
}
