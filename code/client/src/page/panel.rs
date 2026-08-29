use leptos::prelude::*;

const PAGE: &str = "h-screen grid place-items-center px-6 py-12";

const FRAME: &str = "relative w-[40%] min-w-[320px] border border-line bg-[rgba(255,255,255,0.85)] \
    before:pointer-events-none before:absolute before:inset-1.5 before:content-[''] \
    before:border before:border-[rgba(0,0,0,0.04)]";

const FRAME_WIDE: &str = "w-[62%] max-w-[820px] max-narrow:w-[60%] max-compact:w-[84%]";

const INNER: &str = "relative px-10 pb-11 pt-12 max-tight:px-6 max-tight:pb-8 max-tight:pt-9";

const TITLE: &str = "m-0 text-center font-mono text-[37px] font-medium uppercase leading-none \
    tracking-[0.14em] text-ink max-tight:text-[29px]";

const FORM: &str = "flex items-center gap-2.5";
const FORM_FIRST: &str = "mt-9";
const FORM_NEXT: &str = "mt-4";

const FIELD: &str = "min-w-0 flex-1";

const INPUT: &str = "w-full rounded-md border border-line-strong bg-card px-3.5 py-3 font-mono \
    text-[23px] leading-normal text-ink outline-none transition-[border-color,background-color] \
    duration-200 placeholder:text-muted focus:border-ink focus:bg-bg-soft";

const SUBMIT: &str = "shrink-0 cursor-pointer whitespace-nowrap rounded-lg border border-brick-deep \
    bg-linear-to-b from-brick-soft to-brick px-5 py-2.5 \
    font-mono text-[23px] leading-normal text-white \
    shadow-[inset_0_1px_0_rgba(255,255,255,0.28),0_3px_0_#6f3526] \
    transition-[transform,box-shadow] duration-[120ms] ease-out \
    enabled:hover:-translate-y-px \
    enabled:hover:shadow-[inset_0_1px_0_rgba(255,255,255,0.32),0_5px_0_#6f3526,0_8px_14px_rgba(111,53,38,0.35)] \
    enabled:active:-translate-y-[2px] enabled:active:scale-[0.96] enabled:active:font-bold \
    enabled:active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.3),0_1px_0_#6f3526] \
    focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-secondary \
    disabled:opacity-40 disabled:cursor-not-allowed disabled:shadow-none disabled:from-brick";

#[component]
pub fn PanelPage(children: Children) -> impl IntoView {
    view! { <div class=PAGE>{children()}</div> }
}

#[component]
pub fn PanelFrame(#[prop(optional)] wide: bool, children: Children) -> impl IntoView {
    let class = if wide {
        format!("{FRAME} {FRAME_WIDE}")
    } else {
        FRAME.to_string()
    };
    view! { <div class=class>{children()}</div> }
}

#[component]
pub fn PanelInner(children: Children) -> impl IntoView {
    view! { <div class=INNER>{children()}</div> }
}

#[component]
pub fn PanelTitle(children: Children) -> impl IntoView {
    view! { <h1 class=TITLE>{children()}</h1> }
}

#[component]
pub fn PanelForm(
    /// Pairs several fields per row; stacks them on tight screens.
    #[prop(optional)]
    pair: bool,
    /// Centers the row's content, e.g. a lone action button.
    #[prop(optional)]
    center: bool,
    /// True when this form directly follows another panel form.
    #[prop(optional)]
    next: bool,
    children: Children,
) -> impl IntoView {
    let mut class = String::new();
    class.push_str(if next { FORM_NEXT } else { FORM_FIRST });
    class.push(' ');
    class.push_str(FORM);
    if pair {
        class.push_str(" flex-wrap max-tight:flex-col");
    }
    if center {
        class.push_str(" justify-center");
    }
    view! { <div class=class>{children()}</div> }
}

#[component]
pub fn PanelField(children: Children) -> impl IntoView {
    view! { <div class=FIELD>{children()}</div> }
}

#[component]
pub fn PanelInput(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_input: Callback<String>,
    #[prop(optional, into)] placeholder: Option<&'static str>,
    #[prop(optional, into)] autocomplete: Option<&'static str>,
    #[prop(optional)] spellcheck: bool,
) -> impl IntoView {
    view! {
        <input
            class=INPUT
            type="text"
            prop:value=value
            on:input=move |ev| on_input.run(event_target_value(&ev))
            placeholder=placeholder
            autocomplete=autocomplete
            spellcheck=spellcheck
        />
    }
}

#[component]
pub fn PanelSubmit(
    children: Children,
    #[prop(optional, into)] on_click: Option<Callback<()>>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <button
            class=SUBMIT
            type="submit"
            on:click=move |_| { if let Some(cb) = on_click { cb.run(()); } }
            disabled=disabled
        >
            {children()}
        </button>
    }
}
