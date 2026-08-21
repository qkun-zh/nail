use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let schema_path = Path::new("cedar/schema.cedar");
    println!("cargo:rerun-if-changed={}", schema_path.display());
    let schema_text = fs::read_to_string(schema_path)?;
    let schema: cedar_policy::Schema = schema_text
        .parse()
        .map_err(|error| format!("invalid authorization schema: {error}"))?;

    let mut actions: Vec<String> = schema
        .actions()
        .map(|action| action.id().unescaped().to_string())
        .collect();
    actions.sort();
    actions.dedup();
    let permissions = actions
        .iter()
        .map(|action| permission_const(action))
        .collect::<String>();
    let all_permissions = format!(
        "pub const ALL_PERMISSIONS: &[&str] = &[{}];\n",
        actions
            .iter()
            .map(|action| format!("\"{action}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut entities: Vec<String> = schema.entity_types().map(ToString::to_string).collect();
    entities.sort();
    entities.dedup();
    let entity_consts = entities
        .iter()
        .map(|entity| entity_const(entity))
        .collect::<String>();

    let out_dir = env::var("OUT_DIR")?;
    fs::write(Path::new(&out_dir).join("permissions.rs"), permissions)?;
    fs::write(
        Path::new(&out_dir).join("all_permissions.rs"),
        all_permissions,
    )?;
    fs::write(Path::new(&out_dir).join("cedar_entities.rs"), entity_consts)?;
    Ok(())
}

fn permission_const(action: &str) -> String {
    let name = action.replace("::", "_").to_ascii_uppercase();
    format!("pub const PERMISSION_{name}: &str = \"{action}\";\n")
}

fn entity_const(entity: &str) -> String {
    let name = entity.to_ascii_uppercase();
    format!("pub const CEDAR_ENTITY_{name}: &str = \"{entity}\";\n")
}
