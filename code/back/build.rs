use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let schema_path = Path::new("src/infrastructure/cedar/schema.cedar");
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

    let mut entities: Vec<String> = schema.entity_types().map(ToString::to_string).collect();
    entities.sort();
    entities.dedup();
    let entity_consts = entities
        .iter()
        .map(|entity| entity_const(entity))
        .collect::<String>();

    let router_path = Path::new("src/interface/router.rs");
    println!("cargo:rerun-if-changed={}", router_path.display());
    let router_source = fs::read_to_string(router_path)?;
    let route_consts = route_literals(&router_source)
        .map(route_const)
        .collect::<String>();

    let out_dir = env::var("OUT_DIR")?;
    fs::write(Path::new(&out_dir).join("permissions.rs"), permissions)?;
    fs::write(Path::new(&out_dir).join("cedar_entities.rs"), entity_consts)?;
    fs::write(Path::new(&out_dir).join("routes.rs"), route_consts)?;
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
