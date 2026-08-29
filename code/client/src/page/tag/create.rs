use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::panel::{
    PanelField, PanelForm, PanelFrame, PanelInner, PanelInput, PanelPage, PanelSubmit, PanelTitle,
};
use crate::request::tag;

#[component]
pub fn CreateTag() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let navigate = use_navigate();
    let notifications = use_notifications();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let name = name.get();
        let navigate = navigate.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match tag::create_tag(&name).await {
                Ok(tag) => {
                    navigate(&format!("/tag/{}", tag.id), NavigateOptions::default());
                }
                Err(err) => {
                    notify_error(&notifications, err.to_string());
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <PanelPage>
            <PanelFrame>
                <PanelInner>
                    <PanelTitle>"TAG CREATE"</PanelTitle>
                    <PanelForm>
                        <form class="contents" on:submit=on_submit>
                            <PanelField>
                                <PanelInput value=name on_input=move |v| name.set(v) placeholder="name" />
                            </PanelField>
                            <PanelSubmit disabled=submitting>
                                {move || if submitting.get() { "Creating..." } else { "Create" }}
                            </PanelSubmit>
                        </form>
                    </PanelForm>
                </PanelInner>
            </PanelFrame>
        </PanelPage>
    }
}
