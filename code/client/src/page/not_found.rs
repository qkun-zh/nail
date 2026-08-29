use leptos::prelude::*;

const NFPAGE: &str = "h-screen grid place-items-center overflow-hidden px-6 py-12";
const NFINNER: &str = "flex flex-col items-center animate-nf-fade";
const NFTITLE: &str = "m-0 flex items-baseline justify-center gap-[0.06em] border-2 border-secondary \
    px-16 py-12 font-mono font-bold leading-none tracking-[0.06em] text-ink \
    text-[clamp(80px,16vw,160px)] max-tight:text-[38px] max-tight:tracking-[0.18em]";
const NFCHAR: &str = "inline-block opacity-0 animate-char-in";
const NFSPACE: &str = "w-[0.3em]";

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <main class=NFPAGE aria-label="not found">
            <div class=NFINNER>
                <h1 class=NFTITLE>
                    <span class=NFCHAR style="animation-delay: 0.0s">N</span>
                    <span class=NFCHAR style="animation-delay: 0.25s">O</span>
                    <span class=NFCHAR style="animation-delay: 0.12s">T</span>
                    <span class=format!("{NFCHAR} {NFSPACE}") style="animation-delay: 0.35s"> </span>
                    <span class=NFCHAR style="animation-delay: 0.4s">F</span>
                    <span class=NFCHAR style="animation-delay: 0.18s">O</span>
                    <span class=NFCHAR style="animation-delay: 0.3s">U</span>
                    <span class=NFCHAR style="animation-delay: 0.08s">N</span>
                    <span class=NFCHAR style="animation-delay: 0.22s">D</span>
                </h1>
            </div>
        </main>
    }
}
