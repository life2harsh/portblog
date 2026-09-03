use dioxus::prelude::*;

#[component]
pub fn PageShell(children: Element) -> Element {
    rsx! {
        document::Link {
            rel: "preconnect",
            href: "https://fonts.googleapis.com"
        }
        document::Link {
            rel: "preconnect",
            href: "https://fonts.gstatic.com",
            crossorigin: "anonymous"
        }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:ital,wght@0,300;0,400;0,500;1,300;1,400&family=IBM+Plex+Serif:ital,wght@0,300;0,400;1,300&display=swap"
        }
        {children}
    }
}

#[component]
pub fn TwoColumn(left: Element, right: Element) -> Element {
    rsx! {
        div { class: "two-col",
            div { class: "two-col__left", {left} }
            div { class: "two-col__right", {right} }
        }
    }
}

#[component]
pub fn SingleColumn(children: Element) -> Element {
    rsx! {
        div { class: "single-col",
            {children}
        }
    }
}

#[derive(Clone, Debug, PartialEq, Props)]
pub struct TopbarProps {
    pub site_label: &'static str,
    pub domain: &'static str,
    pub other_site_label: &'static str,
    pub other_site_url: &'static str,
}

#[component]
pub fn Topbar(props: TopbarProps) -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "topbar__left",
                span { class: "topbar__domain", "{props.domain}" }
                span { class: "topbar__section", "/ {props.site_label}" }
            }
            nav { class: "topbar__right", aria_label: "Site navigation",
                a {
                    class: "topbar__link",
                    href: "{props.other_site_url}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "{props.other_site_label}"
                }
            }
        }
    }
}

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "site-footer",
            span { class: "footer__copy", "↑↓ navigate" }
            span { class: "footer__sep", "·" }
            span { class: "footer__hint", "enter open" }
            span { class: "footer__sep", "·" }
            span { class: "footer__hint", "a art" }
            span { class: "footer__sep", "·" }
            span { class: "footer__hint", "? shortcuts" }
        }
    }
}
