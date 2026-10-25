//! Operation feedback translated by the presenting application.
#[derive(Clone, Debug)]
pub enum Feedback {
    InputHandled,
    Message(String),
    DeleteFailed(String),
    Exported(String),
    TemplateUnreadable,
    TemplateUnusedFiles,
    TemplateUnclosedQuote,
    QuotedCommandUnsupported,
}
impl From<String> for Feedback {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
