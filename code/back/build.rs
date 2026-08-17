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

    let router_path = Path::new("src/interface/router.rs");
    println!("cargo:rerun-if-changed={}", router_path.display());
    let router_source = fs::read_to_string(router_path)?;
    let route_consts = route_literals(&router_source)
        .map(route_const)
        .collect::<String>();

    let out_dir = env::var("OUT_DIR")?;
    fs::write(Path::new(&out_dir).join("permissions.rs"), permissions)?;
    fs::write(Path::new(&out_dir).join("routes.rs"), route_consts)?;
    Ok(())
}

fn action_name(line: &str) -> Option<&str> {
    line.trim().strip_prefix("action \"")?.split('"').next()
}

fn permission_const(action: &str) -> String {
    let name = action.replace("::", "_").to_ascii_uppercase();
    let test_only = matches!(action, "User::Delete::Transfer");
    let cfg = if test_only { "#[cfg(test)]\n" } else { "" };
    format!("{cfg}pub const PERMISSION_{name}: &str = \"{action}\";\n")
}

fn route_literals(source: &str) -> impl Iterator<Item = &str> {
    let mut rest = source;
    std::iter::from_fn(move || {
        rest = &rest[rest
            .find(".route(")
            .map_or(rest.len(), |i| i + ".route(".len())..];
        let open = rest.find('"')?;
        let after_open = &rest[open + 1..];
        let close = after_open.find('"')?;
        let literal = &after_open[..close];
        rest = &after_open[close + 1..];
        Some(literal)
    })
}

fn route_const(route: &str) -> String {
    let name = route
        .trim_start_matches('/')
        .split('/')
        .map(|segment| {
            segment
                .trim_matches(['{', '}'])
                .replace('-', "_")
                .to_ascii_uppercase()
        })
        .collect::<Vec<_>>()
        .join("_");
    format!("pub const ROUTE_{name}: &str = \"{route}\";\n")
}
