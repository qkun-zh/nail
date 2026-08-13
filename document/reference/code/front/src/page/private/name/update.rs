use crate::page::auth_gate::who_are_you;
use crate::page::notify::{notify_error, notify_success, use_notify};
use crate::pow::prove;
use crate::req::{ProveInput, get_challenge, update_user_name};
use common::name::validate_name;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};

#[component]
pub fn NameUpdate() -> impl IntoView {
    let notification = use_notify();
    let location = use_location();
    let navigate = use_navigate();
    let verified =
        use_context::<RwSignal<Option<bool>>>().unwrap_or_else(|| RwSignal::new(Some(false)));

    let url_params = location.query.get_untracked();
    let new_name = RwSignal::new(url_params.get("name").unwrap_or_default());
    let pathname = location.pathname.get_untracked();

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            let name = new_name.get();
            if !name.is_empty() {
                pairs.push(format!("name={}", crate::req::url_encode(&name)));
            }
            let query_string = pairs.join("&");
            navigate(
                &format!("{pathname}?{query_string}"),
                leptos_router::NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |prev: Option<()>| {
        let _ = new_name.get();
        if prev.is_none() {
            return;
        }
        sync_url();
    });

    let updating = RwSignal::new(false);

    let on_update = move |ev: SubmitEvent| {
        ev.prevent_default();
        if updating.get() {
            return;
        }
        let name_val = match validate_name(&new_name.get()) {
            Ok(canonical) => canonical,
            Err(e) => {
                notify_error(&notification, &e.to_string());
                return;
            }
        };
        updating.set(true);
        spawn_local(async move {
            let challenge = match get_challenge().await {
                Ok(c) => c,
                Err(e) => {
                    notify_error(&notification, &format!("get challenge failed: {e}"));
                    updating.set(false);
                    return;
                }
            };
            let pow = match prove(ProveInput {
                challenge,
                payload: name_val.clone(),
            })
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    notify_error(&notification, &format!("proof failed: {e}"));
                    updating.set(false);
                    return;
                }
            };
            match update_user_name(&pow).await {
                Ok(_) => {
                    notify_success(&notification, &format!("name updated to {name_val}"));
                }
                Err(e) => {
                    notify_error(&notification, &format!("name update failed: {e}"));
                }
            }
            updating.set(false);
        });
    };

    view! {
        {move || match verified.get() {
            Some(true) => view! {
                <form on:submit=on_update>
                    <input type="text" placeholder="new name" bind:value=new_name/>
                    <button type="submit" disabled=move || updating.get()>
                        {move || if updating.get() { "updating..." } else { "update name" }}
                    </button>
                </form>
            }.into_any(),
            _ => who_are_you(),
        }}
    }
}
