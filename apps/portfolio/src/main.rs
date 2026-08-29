use content::project::{PROJECTS, Project};
use dioxus::prelude::*;
use gloo_events::EventListener;
use ui::{Footer, KeyNav, Meta, PageShell, Topbar};
use wasm_bindgen::JsCast;

mod canvas;

fn main() {
    console_log::init_with_level(log::Level::Info).ok();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        PageShell {
            Router::<Route> {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Home {},
}

#[component]
fn Home() -> Element {
    let mut nav = use_signal(|| KeyNav::new(PROJECTS.len()));
    let mut overlay_open = use_signal(|| false);
    let mut search_open = use_signal(|| false);
    let mut search_query = use_signal(String::new);
    let mut art_mode_open = use_signal(|| false);

    let selected_index = nav.read().index;
    let selected_project = &PROJECTS[selected_index];

    {
        let mut nav = nav;
        let mut overlay_open = overlay_open;
        let mut search_open = search_open;
        let mut art_mode_open = art_mode_open;
        use_effect(move || {
            let listener =
                EventListener::new(&gloo_utils::document().into(), "keydown", move |event| {
                    let ke = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
                    if search_open() {
                        return;
                    }
                    match ke.key().as_str() {
                        "j" | "ArrowDown" => {
                            ke.prevent_default();
                            let new_idx = nav.read().advance(ui::nav::NavDir::Down).index;
                            nav.write().index = new_idx;
                        }
                        "k" | "ArrowUp" => {
                            ke.prevent_default();
                            let new_idx = nav.read().advance(ui::nav::NavDir::Up).index;
                            nav.write().index = new_idx;
                        }
                        "?" => {
                            ke.prevent_default();
                            *overlay_open.write() = !overlay_open();
                        }
                        "/" => {
                            ke.prevent_default();
                            *search_open.write() = true;
                        }
                        "a" => {
                            ke.prevent_default();
                            *art_mode_open.write() = !art_mode_open();
                        }
                        "Escape" => {
                            if overlay_open() {
                                *overlay_open.write() = false;
                            }
                            if art_mode_open() {
                                *art_mode_open.write() = false;
                            }
                        }
                        _ => {}
                    }
                });
            std::mem::forget(listener);
        });
    }

    let filtered_projects: Vec<(usize, &'static Project)> = {
        let q = search_query.read().to_lowercase();
        PROJECTS
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
            })
            .collect()
    };

    rsx! {
        Meta {
            title: "life2harsh".to_string(),
            description: "Developer, systems programmer, generative artist.".to_string(),
        }

        Topbar {
            site_label: "portfolio",
            domain: "life2harsh.xyz",
            other_site_label: "writing →",
            other_site_url: "https://blog.life2harsh.xyz",
        }

        main { class: "two-col", role: "main",

            section {
                class: "two-col__left",
                aria_label: "Selected project details",

                div { class: "identity",
                    p { class: "identity__handle", "life2harsh" }
                    h1 { class: "identity__statement",
                        "systems.\ncraft.\nlines."
                    }
                }

                canvas::ArtCanvas {
                    config: selected_project.art.clone(),
                    art_mode_open: art_mode_open(),
                    on_art_mode_close: move |_| *art_mode_open.write() = false,
                }

                ProjectDetail {
                    project: selected_project,
                    key: "{selected_project.slug}",
                }

                nav {
                    class: "identity-links",
                    aria_label: "Quick links",
                    a { class: "identity-link", href: "https://github.com/life2harsh", target: "_blank", rel: "noopener noreferrer", "github" }
                    a { class: "identity-link", href: "mailto:life2harsh@proton.me", "email" }
                    a { class: "identity-link", href: "/resume.pdf", target: "_blank", "résumé" }
                }
            }

            section {
                class: "two-col__right",
                aria_label: "Projects",

                div { class: "project-list",
                    p { class: "project-list__heading", "selected projects" }

                    for (display_idx, (orig_idx, project)) in filtered_projects.iter().enumerate() {
                        ProjectListItem {
                            key: "{project.slug}",
                            number: display_idx + 1,
                            project,
                            is_selected: *orig_idx == selected_index,
                            on_select: {
                                let oi = *orig_idx;
                                move |_| nav.write().index = oi
                            },
                        }
                    }
                }

                nav { class: "right-links", aria_label: "About links",
                    a { class: "right-link", href: "https://blog.life2harsh.xyz", target: "_blank", rel: "noopener noreferrer",
                        span { "writing" }
                        span { class: "right-link__arrow", "→" }
                    }
                    a { class: "right-link", href: "/about",
                        span { "about" }
                        span { class: "right-link__arrow", "→" }
                    }
                    a { class: "right-link", href: "/resume.pdf", target: "_blank",
                        span { "résumé" }
                        span { class: "right-link__arrow", "↗" }
                    }
                }
            }
        }

        Footer {}

        ui::ShortcutOverlay {
            visible: overlay_open(),
            on_close: move |_| *overlay_open.write() = false,
        }

        ui::SearchBar {
            visible: search_open(),
            query: search_query.read().clone(),
            on_change: move |q| *search_query.write() = q,
            on_close: move |_| {
                *search_open.write() = false;
                *search_query.write() = String::new();
            },
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct ProjectDetailProps {
    project: &'static Project,
}

#[component]
fn ProjectDetail(props: ProjectDetailProps) -> Element {
    let p = props.project;

    rsx! {
        div { class: "project-detail",
            h2 { class: "project-detail__name", "{p.name}" }
            p { class: "project-detail__desc", "{p.long_description}" }

            div { class: "project-detail__meta",
                div { class: "meta-row",
                    span { class: "meta-row__label", "year" }
                    span { class: "meta-row__value", "{p.year}" }
                }
                div { class: "meta-row",
                    span { class: "meta-row__label", "status" }
                    span { class: "meta-row__value", "{p.status.label()}" }
                }
                div { class: "meta-row",
                    span { class: "meta-row__label", "stack" }
                    div { class: "stack-list",
                        for s in p.stack {
                            span { class: "stack-item", "{s}" }
                        }
                    }
                }
            }

            div { class: "project-detail__links",
                for link in p.links {
                    a {
                        class: "project-link",
                        href: "{link.url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{link.label}"
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct ProjectListItemProps {
    number: usize,
    project: &'static Project,
    is_selected: bool,
    on_select: EventHandler<()>,
}

#[component]
fn ProjectListItem(props: ProjectListItemProps) -> Element {
    let p = props.project;
    let class = if props.is_selected {
        "project-item is-selected"
    } else {
        "project-item"
    };

    rsx! {
        div {
            class: "{class}",
            role: "option",
            aria_selected: "{props.is_selected}",
            tabindex: 0,
            onclick: move |_| props.on_select.call(()),
            onkeydown: move |e| {
                if e.key() == Key::Enter || e.key() == Key::Character(" ".to_string()) {
                    props.on_select.call(());
                }
            },

            span { class: "project-item__num", "{props.number:02}" }

            div { class: "project-item__body",
                span { class: "project-item__name", "{p.name}" }
                span { class: "project-item__excerpt", "{p.description}" }
                span { class: "project-item__status", "— {p.year} · {p.status.label()}" }
            }
        }
    }
}
