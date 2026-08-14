use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSubjectView {
    pub email_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSubjectsView {
    pub old_email_subject: String,
    pub new_email_subject: String,
}
