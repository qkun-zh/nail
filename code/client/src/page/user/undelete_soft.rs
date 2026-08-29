use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::use_confirm_action;
use crate::page::fetch::require_id;
use crate::page::notify::{notify_success, use_notifications};
use crate::page::panel::{PanelForm, PanelFrame, PanelInner, PanelPage, PanelSubmit, PanelTitle};

#[component]
pub fn UndeleteSoftUser() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let confirm = use_confirm_action(move || {
        let uid = params.get().get("uid").unwrap_or_default();
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        async move {
            require_id(&uid)?;
            crate::request::user::undelete_soft_user(&uid).await?;
            notify_success(&notifications, "user restored");
            navigate(&format!("/user/{uid}"), NavigateOptions::default());
            Ok(())
        }
    });

    view! {
        <PanelPage>
            <PanelFrame>
                <PanelInner>
                    <PanelTitle>"USER UNDELETE"</PanelTitle>
                    <PanelForm center=true>
                        <PanelSubmit
                            disabled=confirm.working
                            on_click=Callback::new(move |()| confirm.submit.run(()))
                        >
                            {move || if confirm.working.get() { "undeleting..." } else { "undelete" }}
                        </PanelSubmit>
                    </PanelForm>
                </PanelInner>
            </PanelFrame>
        </PanelPage>
    }
}
