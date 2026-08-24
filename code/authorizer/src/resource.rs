#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Article {
        id: String,
        owner: String,
    },
    Version {
        id: String,
        article_id: String,
        owner: String,
    },
    Comment {
        id: String,
        version_id: String,
        article_id: String,
        article_owner: String,
        owner: String,
    },
    Role {
        name: String,
    },
    User(String),
    Tag(String),
    Virtual(String),
}
