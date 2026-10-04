//! Operation feedback translated by the presenting application.
#[derive(Clone, Debug)]
pub enum Feedback {
    Message(String),
    DeleteFailed(String),
    Exported(String),
    TemplateUnreadable,
    TemplateUnusedFiles,
    TemplateUnclosedQuote,
}
impl From<String> for Feedback {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
