use nail_common::request::{
    ChangeList, CreateRoleRequest, DeleteBody, DeleteMode, RoleUpdateRequest,
};
use nail_common::response::role::{RoleListPage, RoleNameView, RoleView};

use crate::request::error::RequestResult;
use crate::request::{http, url};

pub async fn read_roles(page: u64, limit: u64) -> RequestResult<RoleListPage> {
    let path = url::build_path_with_query(
        &["role", "read"],
        &[("page", &page.to_string()), ("limit", &limit.to_string())],
    );
    http::get_json(&path, true).await
}

pub async fn read_role(role_id: &str) -> RequestResult<RoleView> {
    let path = url::build_path_with_query(&["role", role_id, "read"], &[]);
    http::get_json(&path, true).await
}

pub async fn create_role(name: &str) -> RequestResult<RoleNameView> {
    let path = url::build_path_with_query(&["role", "create"], &[]);
    let body = CreateRoleRequest {
        name: name.to_string(),
    };
    http::post_json(&path, &body, true).await
}

pub async fn update_role(
    role_id: &str,
    permissions_add: &[String],
    permissions_remove: &[String],
    users_add: &[String],
    users_remove: &[String],
) -> RequestResult<RoleView> {
    let path = url::build_path_with_query(&["role", role_id, "update"], &[]);
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
    http::post_json(&path, &body, true).await
}

pub async fn delete_role(role_id: &str) -> RequestResult<RoleNameView> {
    let path = url::build_path_with_query(&["role", role_id, "delete"], &[]);
    http::post_json(
        &path,
        &DeleteBody {
            mode: Some(DeleteMode::Hard),
        },
        true,
    )
    .await
}
