use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let schema_path = Path::new("src/infrastructure/cedar/schema.cedar");
    println!("cargo:rerun-if-changed={}", schema_path.display());
    let schema = fs::read_to_string(schema_path)?;
    let permissions = schema
        .lines()
        .filter_map(action_name)
        .map(permission_const)
        .collect::<String>();
    let out_dir = env::var("OUT_DIR")?;
    fs::write(Path::new(&out_dir).join("permissions.rs"), permissions)?;
    Ok(())
}

fn action_name(line: &str) -> Option<&str> {
    line.trim().strip_prefix("action \"")?.split('"').next()
}

fn permission_const(action: &str) -> String {
    let name = action.replace("::", "_").to_ascii_uppercase();
    let test_only = matches!(
        action,
        "User::Delete::Transfer" | "Version::Delete::Transfer"
    );
    let cfg = if test_only { "#[cfg(test)]\n" } else { "" };
    format!("{cfg}pub const PERMISSION_{name}: &str = \"{action}\";\n")
}
