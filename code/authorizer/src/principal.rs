#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Principal {
    pub id: String,
    pub roles: Vec<Role>,
}
