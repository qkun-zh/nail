#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Principal {
    pub id: String,
    pub roles: Vec<String>,
}
