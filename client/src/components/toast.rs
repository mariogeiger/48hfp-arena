use crate::state::AppState;
use leptos::prelude::*;

#[component]
pub fn ToastContainer() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    view! {
        <div class="vote-toast-container" id="toast-container">
            <For
                each=move || state.toasts.get()
                key=|t| t.id.to_bits()
                let:toast
            >
                <div class="vote-toast" inner_html=toast.html />
            </For>
        </div>
    }
}
