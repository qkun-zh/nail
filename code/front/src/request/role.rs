use nail_common::request::{ChangeList, CreateRoleRequest, DeleteMode, RoleUpdateRequest};
use nail_common::response::ListPage;
use nail_common::response::NamedRef;
use nail_common::response::role::{RoleListItem, RoleView};

use crate::request::error::RequestResult;
use crate::request::pow::prove_pow;
use crate::request::validate::validate_id;
use crate::request::{http, url};

pub async fn read_roles(page: u64, limit: u64) -> RequestResult<ListPage<RoleListItem>> {
    let pow = prove_pow().await?;
    let path = url::build_path_with_query(
        &["roles"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn read_role(role_id: &str) -> RequestResult<RoleView> {
    let pow = prove_pow().await?;
    let role_id = validate_id(role_id, "role_id")?;
    let path = url::build_path_with_query(&["roles", &role_id], &[]);
    http::get_json(&path, true, Some(&pow)).await
}

pub async fn create_role(name: &str) -> RequestResult<NamedRef> {
    let pow = prove_pow().await?;
    let path = url::build_path_with_query(&["roles"], &[]);
    let body = CreateRoleRequest {
        name: name.to_string(),
    };
    http::post_json(&path, &body, true, Some(&pow)).await
}

pub async fn update_role(
    role_id: &str,
    permissions_add: &[String],
    permissions_remove: &[String],
    users_add: &[String],
    users_remove: &[String],
) -> RequestResult<RoleView> {
    let pow = prove_pow().await?;
    let role_id = validate_id(role_id, "role_id")?;
    let path = url::build_path_with_query(&["roles", &role_id], &[]);
    let body = RoleUpdateRequest {
        permissions: Some(ChangeList {
            add: permissions_add.to_vec(),
            remove: permissions_remove.to_vec(),
        }),
        users: Some(ChangeList {
            add: users_add.to_vec(),
            remove: users_remove.to_vec(),
        }),
    };
    http::patch_json(&path, &body, true, Some(&pow)).await
}

pub async fn delete_role(role_id: &str) -> RequestResult<NamedRef> {
    let pow = prove_pow().await?;
    let role_id = validate_id(role_id, "role_id")?;
    let path = url::build_path_with_query(
        &["roles", &role_id],
        &[(
            "mode",
            &serde_json::to_string(&DeleteMode::Hard).unwrap_or_default(),
        )],
    );
    http::delete_json(&path, true, Some(&pow)).await
}
