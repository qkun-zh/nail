use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <main class="nf-page" aria-label="not found">
            <div class="nf-inner">
                <h1 class="nf-title">
                    <span class="nf-char" style="animation-delay: 0.0s">N</span>
                    <span class="nf-char" style="animation-delay: 0.25s">O</span>
                    <span class="nf-char" style="animation-delay: 0.12s">T</span>
                    <span class="nf-char nf-space" style="animation-delay: 0.35s"> </span>
                    <span class="nf-char" style="animation-delay: 0.4s">F</span>
                    <span class="nf-char" style="animation-delay: 0.18s">O</span>
                    <span class="nf-char" style="animation-delay: 0.3s">U</span>
                    <span class="nf-char" style="animation-delay: 0.08s">N</span>
                    <span class="nf-char" style="animation-delay: 0.22s">D</span>
                </h1>
            </div>
        </main>
    }
}
