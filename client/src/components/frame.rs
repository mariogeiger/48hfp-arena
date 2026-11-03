//! The frame every sheet slides up in: a bottom sheet on phones, a panel on
//! wider screens, closed by its cross, a tap beside it or the Escape key
//! (see `lib.rs`).

use crate::components::icons::{Icon, icon};
use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub fn SheetFrame(
    #[prop(into)] on_close: Callback<()>,
    /// A class of its own, for what the sheet holds.
    #[prop(optional, into)]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class="sheet-backdrop"
            on:click=move |ev| {
                if ev.target() == ev.current_target() {
                    on_close.run(());
                }
            }
        >
            <div class=format!("sheet {class}") role="dialog" aria-modal="true">
                <button class="sheet-close" title=t().sheet_close on:click=move |_| on_close.run(())>
                    {icon(Icon::X)}
                </button>
                {children()}
            </div>
        </div>
    }
}
