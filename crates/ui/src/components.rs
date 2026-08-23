use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Props)]
pub struct MetaProps {
    pub title: String,
    pub description: String,
    pub og_type: Option<String>,
    pub canonical: Option<String>,
}

#[component]
pub fn Meta(props: MetaProps) -> Element {
    let og_type = props.og_type.as_deref().unwrap_or("website");
    rsx! {
        document::Title { "{props.title}" }
        document::Meta { name: "description", content: "{props.description}" }
        document::Meta { property: "og:title", content: "{props.title}" }
        document::Meta { property: "og:description", content: "{props.description}" }
        document::Meta { property: "og:type", content: "{og_type}" }
        if let Some(canon) = &props.canonical {
            document::Link { rel: "canonical", href: "{canon}" }
        }
    }
}

#[component]
pub fn Tag(label: String) -> Element {
    rsx! {
        span { class: "tag", "{label}" }
    }
}

#[derive(Clone, Debug, PartialEq, Props)]
pub struct ExtLinkProps {
    pub href: String,
    pub label: String,
    #[props(default)]
    pub class: String,
}

#[component]
pub fn ExtLink(props: ExtLinkProps) -> Element {
    rsx! {
        a {
            class: "ext-link {props.class}",
            href: "{props.href}",
            target: "_blank",
            rel: "noopener noreferrer",
            "{props.label}"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Props)]
pub struct ShortcutOverlayProps {
    pub visible: bool,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn ShortcutOverlay(props: ShortcutOverlayProps) -> Element {
    if !props.visible {
        return rsx! { Fragment {} };
    }

    let shortcuts: &[(&str, &str)] = &[
        ("j / ↓", "Next item"),
        ("k / ↑", "Previous item"),
        ("Enter", "Open selected"),
        ("Esc", "Go back"),
        ("/", "Search"),
        ("?", "Toggle this overlay"),
        ("a", "Toggle art mode"),
    ];

    rsx! {
        div {
            class: "overlay-backdrop",
            role: "dialog",
            aria_label: "Keyboard shortcuts",
            aria_modal: "true",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "overlay-panel",
                onclick: move |e| e.stop_propagation(),

                header { class: "overlay-header",
                    span { class: "overlay-label", "keyboard shortcuts" }
                    button {
                        class: "overlay-close",
                        aria_label: "Close shortcuts overlay",
                        onclick: move |_| props.on_close.call(()),
                        "×"
                    }
                }

                dl { class: "shortcut-list",
                    for (key, desc) in shortcuts {
                        div { class: "shortcut-row",
                            dt { code { class: "key", "{key}" } }
                            dd { "{desc}" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Props)]
pub struct SearchBarProps {
    pub visible: bool,
    pub query: String,
    pub on_change: EventHandler<String>,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn SearchBar(props: SearchBarProps) -> Element {
    if !props.visible {
        return rsx! { Fragment {} };
    }

    rsx! {
        div { class: "search-bar",
            span { class: "search-prefix", "/" }
            input {
                class: "search-input",
                r#type: "text",
                value: "{props.query}",
                aria_label: "Search",
                autofocus: true,
                oninput: move |e| props.on_change.call(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Escape {
                        props.on_close.call(());
                    }
                },
            }
        }
    }
}

#[component]
pub fn Prose(children: Element) -> Element {
    rsx! {
        article { class: "prose",
            {children}
        }
    }
}
