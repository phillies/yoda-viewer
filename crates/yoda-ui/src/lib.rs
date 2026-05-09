use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    rsx! {
        main {
            h1 { "YoDa Rust" }
            p { "Workspace scaffold complete." }
        }
    }
}