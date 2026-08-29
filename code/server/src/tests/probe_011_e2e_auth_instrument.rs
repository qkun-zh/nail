use std::collections::HashMap;

use database::{EdgeKind, NodeId, NodeKind};

use super::context::TestCtx;
use crate::repository::schema::{
    ArticleRow, CommentRow, KEY_EMAIL_ADDRESS_HASH, KEY_USER_NAME, PermissionRow, RoleRow, TagRow,
    UserRow, VersionRow,
};

fn cache_key_of(token: &str) -> String {
    crate::logic::session::cache_key(token).expect("cache key")
}

fn email_hash(email: &str) -> String {
    common::hash::hash(email.as_bytes()).expect("hash")
}

fn dump_cache(context: &TestCtx, label: &str) {
    eprintln!("\n===== CACHE {label} =====");
    context.state.cache.challenge.dump("challenge");
    context.state.cache.user_creation.dump("user_creation");
    context.state.cache.session.dump("session");
    context.state.cache.email_update.dump("email_update");
    context.state.cache.user_deletion.dump("user_deletion");
    context.state.cache.download.dump("download");
}

fn dump_graph(context: &TestCtx, label: &str) {
    eprintln!("\n===== GRAPH {label} =====");
    let db = &context.state.database;
    let _ = db.read(|scope| {
        let users = scope.all_nodes(NodeKind::User)?;
        eprintln!("[DB] User count={}", users.len());
        let mut user_map: HashMap<NodeId, String> = HashMap::new();
        for node in &users {
            if let Some(row) = scope.read_node::<UserRow>(*node)? {
                user_map.insert(*node, row.id.clone());
                eprintln!(
                    "[DB]   User node={node} id={} name={} email_hash={}",
                    row.id, row.name, row.email_address_hash
                );
            }
        }

        let roles = scope.all_nodes(NodeKind::Role)?;
        eprintln!("[DB] Role count={}", roles.len());
        let mut role_map: HashMap<NodeId, String> = HashMap::new();
        for node in &roles {
            if let Some(row) = scope.read_node::<RoleRow>(*node)? {
                role_map.insert(*node, row.role_name.clone());
                eprintln!("[DB]   Role node={node} role_name={}", row.role_name);
            }
        }

        let perms = scope.all_nodes(NodeKind::Permission)?;
        eprintln!("[DB] Permission count={}", perms.len());
        let mut perm_map: HashMap<NodeId, String> = HashMap::new();
        for node in &perms {
            if let Some(row) = scope.read_node::<PermissionRow>(*node)? {
                perm_map.insert(*node, row.permission_name.clone());
            }
        }

        eprintln!("[DB] UserHoldRole edges:");
        for (node, uid) in &user_map {
            for role_node in scope.outgoing(*node, EdgeKind::UserHoldRole)? {
                if let Some(name) = role_map.get(&role_node) {
                    eprintln!("[DB]   {uid} -UserHoldRole-> {name}");
                }
            }
        }
        eprintln!("[DB] RoleGrantPermission edges:");
        for (node, role_name) in &role_map {
            for perm_node in scope.outgoing(*node, EdgeKind::RoleGrantPermission)? {
                if let Some(perm) = perm_map.get(&perm_node) {
                    eprintln!("[DB]   {role_name} -RoleGrantPermission-> {perm}");
                }
            }
        }

        let article_nodes = scope.all_nodes(NodeKind::Article)?;
        eprintln!("[DB] Article count={}", article_nodes.len());
        for node in &article_nodes {
            if let Some(row) = scope.read_node::<ArticleRow>(*node)? {
                eprintln!(
                    "[DB]   Article node={node} id={} title={} summary={} latest_version_id={:?}",
                    row.id, row.title, row.summary, row.latest_version_id
                );
            }
        }
        let version_nodes = scope.all_nodes(NodeKind::Version)?;
        eprintln!("[DB] Version count={}", version_nodes.len());
        for node in &version_nodes {
            if let Some(row) = scope.read_node::<VersionRow>(*node)? {
                eprintln!(
                    "[DB]   Version node={node} id={} number={} content_hash={}",
                    row.id, row.version_number, row.content_hash
                );
            }
        }
        let tag_nodes = scope.all_nodes(NodeKind::Tag)?;
        eprintln!("[DB] Tag count={}", tag_nodes.len());
        for node in &tag_nodes {
            if let Some(row) = scope.read_node::<TagRow>(*node)? {
                eprintln!(
                    "[DB]   Tag node={node} id={} tag_name={}",
                    row.id, row.tag_name
                );
            }
        }
        let comment_nodes = scope.all_nodes(NodeKind::Comment)?;
        eprintln!("[DB] Comment count={}", comment_nodes.len());
        for node in &comment_nodes {
            if let Some(row) = scope.read_node::<CommentRow>(*node)? {
                eprintln!(
                    "[DB]   Comment node={node} id={} content={}",
                    row.id, row.content
                );
            }
        }
        Ok(())
    });
}

// A running dump that shows the change delta per flow step. We snapshot then
// print again after each HTTP call so the reader can diff node/edge/value deltas.
#[tokio::test]
async fn instrument_auth_email_name_flows() {
    let context = TestCtx::new().await.expect("test context");
    let email = "alice@example.com";

    eprintln!("\n################ FLOW 1: AUTHENTICATE (create user) ################");
    eprintln!("\n--- STEP 1a: send authenticate (create-user) email ---");
    eprintln!("[ACTION] POST /tokens purpose=create_user email={email}");
    let (status, body) = context
        .post(
            "/tokens",
            serde_json::json!({ "purpose": "create_user", "email": email }),
            None,
        )
        .await;
    eprintln!("[RESP ] status={status} body={body}");
    let messages = context.emails();
    eprintln!("[MAIL ] {} message(s)", messages.len());
    for m in &messages {
        eprintln!("  to={} subject={} token={}", m.0, m.1, m.2);
    }
    let creation_token = &messages[0].2;
    let creation_key = cache_key_of(creation_token);
    eprintln!(
        "[OBSERVE] user_creation cache key(hash of create token)={creation_key} value={:?}",
        context
            .state
            .cache
            .user_creation
            .read(creation_key.as_str()),
    );
    dump_cache(&context, "after create-user email send");
    dump_graph(&context, "after create-user email send");

    eprintln!("\n--- STEP 1b: redeem token (create user + session) ---");
    eprintln!("[ACTION] POST /users token={creation_token}");
    let (status, body) = context
        .post(
            "/users",
            serde_json::json!({ "token": creation_token }),
            None,
        )
        .await;
    eprintln!("[RESP ] status={status} body={body}");
    let session_token = body["data"]["session_token"]
        .as_str()
        .expect("session token")
        .to_string();
    let session_key = cache_key_of(&session_token);
    eprintln!("[OBSERVE] after redeem: session key={session_key}");
    eprintln!(
        "[OBSERVE] user_creation entry now (should be gone): {:?}",
        context
            .state
            .cache
            .user_creation
            .read(creation_key.as_str()),
    );
    eprintln!("[OBSERVE] session table now holds the new session");
    dump_cache(&context, "after redeem (authenticate)");
    dump_graph(&context, "after redeem (authenticate)");

    let user_id = crate::repository::user::read_user_by_email_address_hash(
        &context.state.database,
        &email_hash(email),
    )
    .expect("lookup")
    .expect("user id");
    eprintln!(
        "[OBSERVE] resolved user_id for {email} = {user_id} (name defaults to {})",
        user_id.replace('-', "")
    );
    eprintln!("[OBSERVE] authorizer: member User::Read on self (should allow)");
    let _ = context.state.authorizer.authorize(
        &user_id,
        "User::Read",
        &crate::repository::authorization::Resource::User(user_id.clone()),
    );

    eprintln!("\n################ FLOW 2: EMAIL UPDATE ################");
    eprintln!("\n--- STEP 2a: send change-email confirmation emails ---");
    let new_email = "alice-new@example.com";
    eprintln!("[ACTION] POST /tokens purpose=update_user_email old={email} new={new_email}");
    let (status, body) = context
        .post(
            "/tokens",
            serde_json::json!({
                "purpose": "update_user_email",
                "old_email": email,
                "new_email": new_email,
            }),
            Some(&session_token),
        )
        .await;
    eprintln!("[RESP ] status={status} body={body}");
    let messages = context.emails();
    let old_token = &messages[1].2;
    let new_token = &messages[2].2;
    eprintln!("[MAIL ] messages after send (index 1 = old, 2 = new):");
    eprintln!("  old token={old_token}, new token={new_token}");
    eprintln!(
        "[OBSERVE] email_update cache for user {user_id}: {:?}",
        context.state.cache.email_update.read(&user_id),
    );
    dump_cache(&context, "after email-update send");
    dump_graph(&context, "after email-update send (unchanged)");

    eprintln!("\n--- STEP 2b: confirm email change (patch) ---");
    eprintln!("[ACTION] PATCH /users/{user_id} with old/new email tokens");
    let (status, body) = context
        .patch(
            &format!("/users/{user_id}"),
            serde_json::json!({
                "old_email_token": old_token,
                "new_email_token": new_token,
            }),
            Some(&session_token),
        )
        .await;
    eprintln!("[RESP ] status={status} body={body}");
    let new_session_token = body["data"]["session_token"]
        .as_str()
        .expect("new session token")
        .to_string();
    eprintln!(
        "[OBSERVE] after confirm: email_update entry should be gone: {:?}",
        context.state.cache.email_update.read(&user_id),
    );
    eprintln!(
        "[OBSERVE] old session key should be deleted: {:?}",
        context.state.cache.session.read(session_key.as_str()),
    );
    eprintln!(
        "[OBSERVE] new session key present: {:?}",
        context
            .state
            .cache
            .session
            .read(cache_key_of(&new_session_token).as_str()),
    );
    dump_cache(&context, "after email-update confirm");
    dump_graph(&context, "after email-update confirm");
    let user_row = crate::repository::user::read_user(&context.state.database, &user_id)
        .expect("read")
        .expect("entry");
    eprintln!(
        "[OBSERVE] user email_hash now: {} (expected {})",
        user_row.email_address_hash,
        email_hash(new_email),
    );

    eprintln!("\n################ FLOW 3: NAME UPDATE ################");
    eprintln!("\n--- STEP 3: rename self ---");
    eprintln!("[ACTION] PATCH /users/{user_id} name=alice-renamed");
    let (status, body) = context
        .patch(
            &format!("/users/{user_id}"),
            serde_json::json!({ "name": "alice-renamed" }),
            Some(&new_session_token),
        )
        .await;
    eprintln!("[RESP ] status={status} body={body}");
    dump_cache(&context, "after name update");
    dump_graph(&context, "after name update");
    let user_entry = crate::repository::user::read_user(&context.state.database, &user_id)
        .expect("read")
        .expect("entry");
    eprintln!(
        "[OBSERVE] name now: {} (expected alice-renamed), email unchanged: {}",
        user_entry.name, user_entry.email_address_hash
    );
    eprintln!(
        "[OBSERVE] name index lookup by new name -> node exists: {:?}",
        context
            .state
            .database
            .read(|scope| scope.find_by_key(KEY_USER_NAME, "alice-renamed")),
    );
    eprintln!(
        "[OBSERVE] email index lookup by OLD hash now gone: {:?}",
        context
            .state
            .database
            .read(|scope| scope.find_by_key(KEY_EMAIL_ADDRESS_HASH, &email_hash(email))),
    );
    eprintln!("\n################ FLOWS COMPLETE ################");
}
