use content::post::{POSTS, PostMeta, find_post};
use dioxus::prelude::*;
use gloo_events::EventListener;
use ui::{Footer, KeyNav, Meta, PageShell, Prose, Tag, Topbar};
use wasm_bindgen::JsCast;

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
    BlogHome {},
    #[route("/post/:slug")]
    Article { slug: String },
}

#[component]
fn BlogHome() -> Element {
    let mut nav = use_signal(|| KeyNav::new(POSTS.len()));
    let mut overlay_open = use_signal(|| false);
    let mut search_open = use_signal(|| false);
    let mut search_query = use_signal(String::new);
    let navigator = use_navigator();

    let selected_index = nav.read().index;

    {
        let mut nav = nav;
        let mut overlay_open = overlay_open;
        let mut search_open = search_open;
        let nav_ref = navigator;
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
                        "Enter" => {
                            ke.prevent_default();
                            let slug = POSTS[nav.read().index].meta.slug;
                            nav_ref.push(Route::Article {
                                slug: slug.to_string(),
                            });
                        }
                        "?" => {
                            ke.prevent_default();
                            *overlay_open.write() = !overlay_open();
                        }
                        "/" => {
                            ke.prevent_default();
                            *search_open.write() = true;
                        }
                        "Escape" => {
                            *overlay_open.write() = false;
                        }
                        _ => {}
                    }
                });
            std::mem::forget(listener);
        });
    }

    let filtered_posts: Vec<(usize, &'static PostMeta)> = {
        let q = search_query.read().to_lowercase();
        POSTS
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                q.is_empty()
                    || p.meta.title.to_lowercase().contains(&q)
                    || p.meta.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .map(|(i, p)| (i, &p.meta))
            .collect()
    };

    let preview_post = &POSTS[selected_index].meta;

    rsx! {
        Meta {
            title: "life2harsh — writing".to_string(),
            description: "Essays on systems programming, design, and toolcraft.".to_string(),
        }

        Topbar {
            site_label: "blog",
            domain: "blog.life2harsh.xyz",
            other_site_label: "portfolio →",
            other_site_url: "https://life2harsh.xyz",
        }

        main { class: "two-col", role: "main",

            section {
                class: "two-col__left",
                aria_label: "Post preview",

                div { class: "identity",
                    p { class: "identity__handle", "life2harsh · writing" }
                    h1 { class: "identity__statement",
                        "words\nabout\ncraft."
                    }
                }

                PostPreview {
                    meta: preview_post,
                    key: "{preview_post.slug}",
                    on_open: move || {
                        let slug = preview_post.slug;
                        navigator.push(Route::Article { slug: slug.to_string() });
                    },
                }
            }

            section {
                class: "two-col__right",
                aria_label: "Posts",

                div { class: "post-list",
                    p { class: "post-list__heading", "essays & notes" }

                    for (display_idx, (orig_idx, post)) in filtered_posts.iter().enumerate() {
                        PostListItem {
                            key: "{post.slug}",
                            number: display_idx + 1,
                            meta: post,
                            is_selected: *orig_idx == selected_index,
                            on_select: {
                                let oi = *orig_idx;
                                move |_| nav.write().index = oi
                            },
                            on_open: {
                                let slug = post.slug;
                                let nav_ref = navigator;
                                move || { nav_ref.push(Route::Article { slug: slug.to_string() }); }
                            },
                        }
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
struct PostPreviewProps {
    meta: &'static PostMeta,
    on_open: EventHandler<()>,
}

#[component]
fn PostPreview(props: PostPreviewProps) -> Element {
    let m = props.meta;

    rsx! {
        div { class: "post-preview",

            div { class: "post-preview__tag-row",
                for tag in m.tags {
                    Tag { label: tag.to_string() }
                }
            }

            h2 { class: "post-preview__title", "{m.title}" }

            p { class: "post-preview__excerpt", "{m.excerpt}" }

            div { class: "post-preview__meta",
                span { "{m.date}" }
                span { "{m.read_time_min} min read" }
            }

            button {
                class: "post-preview__open",
                onclick: move |_| props.on_open.call(()),
                "read → (enter)"
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct PostListItemProps {
    number: usize,
    meta: &'static PostMeta,
    is_selected: bool,
    on_select: EventHandler<()>,
    on_open: EventHandler<()>,
}

#[component]
fn PostListItem(props: PostListItemProps) -> Element {
    let m = props.meta;
    let class = if props.is_selected {
        "post-item is-selected"
    } else {
        "post-item"
    };

    rsx! {
        div {
            class: "{class}",
            role: "option",
            aria_selected: "{props.is_selected}",
            tabindex: 0,
            onclick: move |_| props.on_select.call(()),
            ondoubleclick: move |_| props.on_open.call(()),
            onkeydown: move |e| {
                if e.key() == Key::Enter {
                    props.on_open.call(());
                }
            },

            span { class: "post-item__num", "{props.number:02}" }

            div { class: "post-item__body",
                span { class: "post-item__title", "{m.title}" }
                div { class: "post-item__meta",
                    span { "{m.date}" }
                    span { "{m.read_time_min} min" }
                }
            }
        }
    }
}

#[component]
fn Article(slug: String) -> Element {
    let navigator = use_navigator();

    {
        let nav_ref = navigator;
        use_effect(move || {
            let listener =
                EventListener::new(&gloo_utils::document().into(), "keydown", move |event| {
                    let ke = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
                    if ke.key() == "Escape" {
                        nav_ref.go_back();
                    }
                });
            std::mem::forget(listener);
        });
    }

    let post = find_post(&slug);

    if post.is_none() {
        return rsx! {
            p { style: "padding: 4rem 2rem; color: var(--color-ink-3);", "Post not found." }
        };
    }

    let post = post.unwrap();
    let m = &post.meta;
    let html = content::parse_markdown(post.body);

    let post_index = POSTS.iter().position(|p| p.meta.slug == m.slug);
    let prev_post = post_index.filter(|&i| i > 0).and_then(|i| POSTS.get(i - 1));
    let next_post = post_index.and_then(|i| POSTS.get(i + 1));

    rsx! {
        Meta {
            title: format!("{} — life2harsh", m.title),
            description: m.excerpt.to_string(),
            og_type: Some("article".to_string()),
        }

        Topbar {
            site_label: "blog",
            domain: "blog.life2harsh.xyz",
            other_site_label: "portfolio →",
            other_site_url: "https://life2harsh.xyz",
        }

        main {
            class: "single-col",
            role: "main",

            article {
                header { class: "article-header",
                    div { class: "article-header__tags",
                        for tag in m.tags {
                            Tag { label: tag.to_string() }
                        }
                    }
                    h1 { class: "article-header__title", "{m.title}" }
                    div { class: "article-header__meta",
                        span { "{m.date}" }
                        span { "{m.read_time_min} min read" }
                    }
                }

                Prose {
                    div { dangerous_inner_html: "{html}" }
                }

                nav { class: "article-nav", aria_label: "Post navigation",
                    Link {
                        class: "article-nav__back",
                        to: Route::BlogHome {},
                        "← all posts"
                    }

                    div { style: "display: flex; gap: 2rem;",
                        if let Some(prev) = prev_post {
                            Link {
                                class: "article-nav__link",
                                to: Route::Article { slug: prev.meta.slug.to_string() },
                                span { class: "article-nav__dir", "← prev" }
                                span { class: "article-nav__title", "{prev.meta.title}" }
                            }
                        }

                        if let Some(next) = next_post {
                            Link {
                                class: "article-nav__link",
                                to: Route::Article { slug: next.meta.slug.to_string() },
                                span { class: "article-nav__dir", "next →" }
                                span { class: "article-nav__title", "{next.meta.title}" }
                            }
                        }
                    }
                }
            }
        }

        Footer {}
    }
}
