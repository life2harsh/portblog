use std::rc::Rc;

use content::{Entry, EntryDraft, EntryKind, format_date, parse_markdown};
use dioxus::prelude::*;
use gloo_events::EventListener;
use ui::Meta;
use wasm_bindgen::JsCast;

mod api;
mod art;

const PORTFOLIO: &str = "https://life2harsh.xyz";
const LIGHT_INK: &str = "#141413";
const DARK_INK: &str = "#e6e3d9";
const MODE_KEY: &str = "life2harsh.mode";

fn main() {
    console_log::init_with_level(log::Level::Info).ok();
    dioxus::launch(App);
}

#[derive(Debug, Clone, PartialEq, Routable)]
#[rustfmt::skip]
enum Route {
    #[layout(Frame)]
        #[route("/")]
        Index {},
        #[route("/posts/:slug")]
        Post { slug: String },
        #[route("/write")]
        Write {},
        #[route("/write/:slug")]
        Edit { slug: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    Acid,
    Ice,
    Cobalt,
    Burnt,
    Warm,
}

impl Theme {
    fn next(self) -> Self {
        match self {
            Self::Acid => Self::Ice,
            Self::Ice => Self::Cobalt,
            Self::Cobalt => Self::Burnt,
            Self::Burnt => Self::Warm,
            Self::Warm => Self::Acid,
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::Acid => "theme--acid",
            Self::Ice => "theme--ice",
            Self::Cobalt => "theme--cobalt",
            Self::Burnt => "theme--burnt",
            Self::Warm => "theme--warm",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Acid => "ACID",
            Self::Ice => "ICE",
            Self::Cobalt => "COBALT",
            Self::Burnt => "BURNT",
            Self::Warm => "WARM",
        }
    }

    fn art_tint(self) -> &'static str {
        match self {
            Self::Acid => "#b4e01c",
            Self::Ice => "#2cc4e0",
            Self::Cobalt => "#3d63f0",
            Self::Burnt => "#f07422",
            Self::Warm => "#ec4c42",
        }
    }
}

#[derive(Clone, Copy)]
struct Journal {
    entries: Signal<Vec<Entry>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    token: Signal<Option<String>>,
    cursor: Signal<usize>,
    theme: Signal<Theme>,
    dark: Signal<bool>,
}

impl Journal {
    fn refresh(mut self) {
        spawn(async move {
            *self.loading.write() = true;
            let token = self.token.peek().clone();
            match api::list(token.as_deref()).await {
                Ok(entries) => {
                    *self.entries.write() = entries;
                    *self.error.write() = None;
                }
                Err(api::ApiError::Unauthorized) if token.is_some() => {
                    api::store_token(None);
                    *self.token.write() = None;
                    self.refresh();
                }
                Err(failure) => *self.error.write() = Some(failure.to_string()),
            }
            *self.loading.write() = false;
        });
    }

    fn owner(&self) -> bool {
        self.token.read().is_some()
    }

    fn ink(&self) -> &'static str {
        if (self.dark)() { DARK_INK } else { LIGHT_INK }
    }
}

#[component]
fn App() -> Element {
    let journal = use_context_provider(|| Journal {
        entries: Signal::new(Vec::new()),
        loading: Signal::new(true),
        error: Signal::new(None),
        token: Signal::new(api::stored_token()),
        cursor: Signal::new(0),
        theme: Signal::new(Theme::Cobalt),
        dark: Signal::new(initial_dark_mode()),
    });
    use_hook(move || journal.refresh());
    use_effect(move || apply_dark_mode((journal.dark)()));
    rsx! {
        ui::Sky {}
        Router::<Route> {}
    }
}

fn initial_dark_mode() -> bool {
    let window = gloo_utils::window();
    let stored = window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(MODE_KEY).ok().flatten());
    match stored.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => false,
    }
}

fn toggle_dark(mut journal: Journal) {
    let next = !*journal.dark.peek();
    *journal.dark.write() = next;
    if let Some(storage) = gloo_utils::window().local_storage().ok().flatten() {
        let _ = storage.set_item(MODE_KEY, if next { "dark" } else { "light" });
    }
}

fn apply_dark_mode(dark: bool) {
    let document = gloo_utils::document();
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("data-mode", if dark { "dark" } else { "light" });
    }
    if let Ok(Some(meta)) = document.query_selector("meta[name=theme-color]") {
        let _ = meta.set_attribute("content", if dark { "#121211" } else { "#efede6" });
    }
}

fn typing() -> bool {
    gloo_utils::document()
        .active_element()
        .is_some_and(|element| matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA"))
}

fn scroll_to_top() {
    gloo_utils::window().scroll_to_with_x_and_y(0.0, 0.0);
}

fn handle_key(
    key: &web_sys::KeyboardEvent,
    nav: dioxus::router::Navigator,
    route: Route,
    mut journal: Journal,
) {
    if typing() || key.ctrl_key() || key.meta_key() || key.alt_key() {
        return;
    }
    let count = journal.entries.peek().len();
    let owner = journal.token.peek().is_some();
    match (key.key().as_str(), &route) {
        ("j" | "ArrowDown", Route::Index {}) if count > 0 => {
            let next = (*journal.cursor.peek() + 1) % count;
            *journal.cursor.write() = next;
        }
        ("k" | "ArrowUp", Route::Index {}) if count > 0 => {
            let next = (*journal.cursor.peek() + count - 1) % count;
            *journal.cursor.write() = next;
        }
        ("Enter", Route::Index {}) => {
            let slug = journal
                .entries
                .peek()
                .get(*journal.cursor.peek())
                .map(|entry| entry.slug.clone());
            match slug {
                Some(slug) => {
                    nav.push(Route::Post { slug });
                }
                None => return,
            }
        }
        ("[" | "]", Route::Post { slug }) => {
            let entries = journal.entries.peek();
            let Some(position) = entries.iter().position(|entry| &entry.slug == slug) else {
                return;
            };
            let target = if key.key() == "]" {
                entries.get(position + 1)
            } else {
                position.checked_sub(1).and_then(|index| entries.get(index))
            };
            let Some(target) = target.map(|entry| entry.slug.clone()) else {
                return;
            };
            drop(entries);
            nav.push(Route::Post { slug: target });
        }
        ("Escape" | "Backspace", Route::Post { .. } | Route::Write {} | Route::Edit { .. }) => {
            nav.push(Route::Index {});
        }
        ("n", _) if owner => {
            nav.push(Route::Write {});
        }
        ("e", Route::Post { slug }) if owner => {
            nav.push(Route::Edit { slug: slug.clone() });
        }
        ("t" | "T", _) => {
            let next = journal.theme.peek().next();
            *journal.theme.write() = next;
        }
        ("d" | "D", _) => toggle_dark(journal),
        _ => return,
    }
    key.prevent_default();
}

#[component]
fn Frame() -> Element {
    let mut journal = use_context::<Journal>();
    let route = use_route::<Route>();
    let mut current = use_signal(|| route.clone());
    use_effect(use_reactive!(|route| *current.write() = route));

    use_hook(move || {
        let nav = navigator();
        let runtime = dioxus::core::Runtime::current();
        let scope = dioxus::core::current_scope_id();
        Rc::new(EventListener::new(
            &gloo_utils::document(),
            "keydown",
            move |event| {
                let Some(key) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                    return;
                };
                runtime.in_scope(scope, || {
                    handle_key(key, nav, current.peek().clone(), journal)
                });
            },
        ))
    });

    let owner = journal.owner();
    rsx! {
        Meta {
            title: "life2harsh.xyz || blog".to_string(),
            description: "Posts by Harsh Jha on systems, tools and the web.".to_string(),
        }
        main { class: "app {(journal.theme)().class()}",
            header { class: "app-header",
                span { class: "app-header__brand",
                    a { href: PORTFOLIO, "life2harsh.xyz" }
                    span { class: "app-header__sep", " || " }
                    Link { to: Route::Index {}, "blog" }
                }
                nav { class: "app-header__links", aria_label: "Primary navigation",
                    a { href: PORTFOLIO, "home" }
                    Link {
                        to: Route::Index {},
                        class: if matches!(route, Route::Index {} | Route::Post { .. }) { "is-active" } else { "" },
                        "posts"
                    }
                    if owner {
                        Link {
                            to: Route::Write {},
                            class: if matches!(route, Route::Write {} | Route::Edit { .. }) { "is-active" } else { "" },
                            span { class: "key-hint", "[N] " }
                            "new post"
                        }
                    }
                }
            }
            section { class: "app-main", Outlet::<Route> {} }
            footer { class: "status",
                match route {
                    Route::Post { .. } => rsx! {
                        span { "esc posts" }
                        span { "[ ] newer / older" }
                    },
                    _ => rsx! {
                        span { "↑↓ posts" }
                        span { "enter read" }
                    },
                }
                if owner {
                    span { "n new" }
                    span { "e edit" }
                }
                span { "t theme" }
                span { if (journal.dark)() { "d light" } else { "d dark" } }
                button {
                    class: "status__right",
                    onclick: move |_| {
                        let next = journal.theme.peek().next();
                        *journal.theme.write() = next;
                    },
                    "THEME: {(journal.theme)().label()}"
                }
                span { class: "status__sep", "|" }
                button { onclick: move |_| toggle_dark(journal), if (journal.dark)() { "DARK" } else { "LIGHT" } }
                if owner {
                    span { class: "status__sep", "|" }
                    button {
                        onclick: move |_| {
                            api::store_token(None);
                            *journal.token.write() = None;
                            journal.refresh();
                        },
                        "LOCK"
                    }
                }
            }
        }
    }
}

#[component]
fn Index() -> Element {
    let mut journal = use_context::<Journal>();
    let nav = navigator();
    let entries = journal.entries.read().clone();
    let cursor = (journal.cursor)().min(entries.len().saturating_sub(1));
    let owner = journal.owner();
    let published = entries.iter().filter(|entry| entry.published).count();

    use_effect(move || {
        let index = (journal.cursor)();
        if let Some(card) = gloo_utils::document().get_element_by_id(&format!("post-card-{index}"))
        {
            let rect = card.get_bounding_client_rect();
            let viewport = gloo_utils::window()
                .inner_height()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            if rect.top() < 60.0 || rect.bottom() > viewport - 50.0 {
                card.scroll_into_view_with_bool(rect.top() < 60.0);
            }
        }
    });

    rsx! {
        div { class: "index",
            header { class: "index__intro",
                p { class: "label", "{published:02} POSTS" }
                h1 { class: "index__title step-in", "blog." }
                p { class: "index__lede",
                    "long-form notes on systems, terminals, the web and whatever i'm currently taking apart."
                }
                if owner {
                    Link { class: "index__new", to: Route::Write {}, "+ new post" }
                }
            }
            if (journal.loading)() && entries.is_empty() {
                p { class: "index__state", "loading posts ░▒▓" }
            } else if let Some(failure) = (journal.error)() {
                p { class: "index__state", "{failure}. posts will show up once it answers." }
            } else if entries.is_empty() {
                p { class: "index__state", "nothing here yet. check back soon." }
            } else {
                div { class: "cards",
                    for (index, entry) in entries.iter().cloned().enumerate() {
                        PostCard {
                            key: "{entry.slug}",
                            index,
                            selected: index == cursor,
                            entry,
                            on_open: move |slug: String| {
                                nav.push(Route::Post { slug });
                            },
                            on_hover: move |index| *journal.cursor.write() = index,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PostCard(
    index: usize,
    selected: bool,
    entry: Entry,
    on_open: EventHandler<String>,
    on_hover: EventHandler<usize>,
) -> Element {
    let journal = use_context::<Journal>();
    let featured = index == 0;
    let slug = entry.slug.clone();
    let mut class = String::from("card");
    if featured {
        class.push_str(" card--featured");
    }
    if selected {
        class.push_str(" is-selected");
    }
    rsx! {
        article {
            id: "post-card-{index}",
            class: "{class}",
            onclick: move |_| on_open.call(slug.clone()),
            onmouseenter: move |_| on_hover.call(index),
            div { class: "card__body",
                h2 { class: "card__title",
                    span { class: "card__number", "{index + 1:02}" }
                    " {entry.heading()}"
                }
                p { class: "card__meta",
                    "{format_date(entry.created_at)}, {entry.reading_minutes()} MIN"
                    if !entry.published {
                        span { class: "chip chip--draft", "DRAFT" }
                    }
                }
                p { class: "card__summary", "{entry.description(180)}" }
                if !entry.tags.is_empty() {
                    div { class: "chips",
                        for tag in entry.tags.iter() {
                            span { class: "stack-chip", "{tag}" }
                        }
                    }
                }
            }
            if featured {
                art::EntryArt {
                    config: art::config_for(entry.seed()),
                    accent: (journal.theme)().art_tint(),
                    ink: journal.ink(),
                }
            }
        }
    }
}

#[component]
fn Post(slug: String) -> Element {
    let journal = use_context::<Journal>();
    use_effect(use_reactive!(|slug| {
        let _ = slug;
        scroll_to_top();
    }));
    let entries = journal.entries.read();
    let position = entries.iter().position(|entry| entry.slug == slug);
    let Some(position) = position else {
        let state = if (journal.loading)() && entries.is_empty() {
            "loading ░▒▓".to_string()
        } else if let Some(failure) = (journal.error)() {
            format!("{failure}.")
        } else {
            "no post by that name.".to_string()
        };
        return rsx! { PostState { text: state } };
    };
    let entry = entries[position].clone();
    let newer = position.checked_sub(1).map(|index| entries[index].clone());
    let older = entries.get(position + 1).cloned();
    drop(entries);
    let html = parse_markdown(&entry.body);
    rsx! {
        article { class: "post", key: "{entry.slug}",
            nav { class: "post__crumbs",
                Link { to: Route::Index {}, "← all posts" }
                if journal.owner() {
                    Link { to: Route::Edit { slug: entry.slug.clone() }, span { class: "key-hint", "[e] " } "edit" }
                }
            }
            header { class: "post__head",
                div { class: "post__titles",
                    p { class: "label",
                        "{format_date(entry.created_at)}, {entry.reading_minutes()} MIN READ"
                        if !entry.published {
                            span { class: "chip chip--draft", "DRAFT" }
                        }
                    }
                    h1 { class: "step-in", "{entry.heading()}" }
                    if let Some(summary) = entry.summary.clone() {
                        p { class: "post__lede", "{summary}" }
                    }
                    if !entry.tags.is_empty() {
                        div { class: "chips",
                            for tag in entry.tags.iter() {
                                span { class: "stack-chip", "{tag}" }
                            }
                        }
                    }
                }
                art::EntryArt {
                    config: art::config_for(entry.seed()),
                    accent: (journal.theme)().art_tint(),
                    ink: journal.ink(),
                }
            }
            div { class: "prose", dangerous_inner_html: "{html}" }
            footer { class: "post__pager",
                if let Some(newer) = newer {
                    Link { class: "pager pager--newer", to: Route::Post { slug: newer.slug.clone() },
                        span { class: "label", "← NEWER" }
                        strong { "{newer.heading()}" }
                    }
                } else {
                    span {}
                }
                if let Some(older) = older {
                    Link { class: "pager pager--older", to: Route::Post { slug: older.slug.clone() },
                        span { class: "label", "OLDER →" }
                        strong { "{older.heading()}" }
                    }
                }
            }
        }
    }
}

#[component]
fn PostState(text: String) -> Element {
    rsx! {
        article { class: "post",
            nav { class: "post__crumbs", Link { to: Route::Index {}, "← all posts" } }
            p { class: "index__state", "{text}" }
        }
    }
}

#[component]
fn Write() -> Element {
    rsx! { Editor { slug: None } }
}

#[component]
fn Edit(slug: String) -> Element {
    rsx! { Editor { slug: Some(slug) } }
}

#[component]
fn Editor(slug: Option<String>) -> Element {
    let journal = use_context::<Journal>();
    if !journal.owner() {
        return rsx! { TokenGate {} };
    }
    rsx! { Composer { key: "{slug:?}", slug } }
}

#[component]
fn TokenGate() -> Element {
    let mut journal = use_context::<Journal>();
    let mut token = use_signal(String::new);
    let mut status = use_signal(String::new);
    let unlock = move |event: FormEvent| {
        event.prevent_default();
        let candidate = token.peek().trim().to_string();
        if candidate.is_empty() {
            return;
        }
        *status.write() = "checking…".into();
        spawn(async move {
            match api::list(Some(&candidate)).await {
                Ok(entries) => {
                    api::store_token(Some(&candidate));
                    *journal.entries.write() = entries;
                    *journal.token.write() = Some(candidate);
                    *status.write() = String::new();
                }
                Err(failure) => *status.write() = failure.to_string(),
            }
        });
    };
    rsx! {
        div { class: "gate",
            form { class: "gate__form", onsubmit: unlock,
                p { class: "label", "WRITE ACCESS" }
                h1 { "unlock the editor" }
                p { class: "gate__copy", "paste your write token. it stays in this browser only." }
                input {
                    r#type: "password",
                    autocomplete: "current-password",
                    placeholder: "token",
                    value: "{token}",
                    oninput: move |event| *token.write() = event.value(),
                }
                div { class: "gate__bar",
                    span { class: "gate__status", "{status}" }
                    button { r#type: "submit", "→ unlock" }
                }
            }
        }
    }
}

#[component]
fn Composer(slug: Option<String>) -> Element {
    let mut journal = use_context::<Journal>();
    let nav = navigator();
    let mut title = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut tags = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut published = use_signal(|| true);
    let mut status = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut armed_delete = use_signal(|| false);
    let mut filled = use_signal(|| slug.is_none());

    let existing = slug.as_ref().and_then(|slug| {
        journal
            .entries
            .read()
            .iter()
            .find(|entry| &entry.slug == slug)
            .cloned()
    });
    if let Some(entry) = existing.clone()
        && !*filled.peek()
    {
        *filled.write() = true;
        *title.write() = entry.title.clone().unwrap_or_default();
        *summary.write() = entry.summary.clone().unwrap_or_default();
        *tags.write() = entry.tags.join(", ");
        *body.write() = entry.body.clone();
        *published.write() = entry.published;
    }

    let target = slug.clone();
    let save = use_callback(move |()| {
        if *busy.peek() {
            return;
        }
        let Some(token) = journal.token.peek().clone() else {
            return;
        };
        if title.peek().trim().is_empty() {
            *status.write() = "give it a title".into();
            return;
        }
        if body.peek().trim().is_empty() {
            *status.write() = "write something first".into();
            return;
        }
        let draft = EntryDraft {
            kind: Some(EntryKind::Essay),
            title: Some(title.peek().clone()),
            summary: Some(summary.peek().clone()),
            body: Some(body.peek().clone()),
            tags: Some(
                tags.peek()
                    .split(',')
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect(),
            ),
            published: Some(*published.peek()),
        };
        *busy.write() = true;
        *status.write() = "saving…".into();
        let target = target.clone();
        spawn(async move {
            match api::save(target.as_deref(), &draft, &token).await {
                Ok(entry) => {
                    *status.write() = "saved".into();
                    let mut entries = journal.entries.write();
                    entries.retain(|existing| existing.slug != entry.slug);
                    entries.insert(0, entry.clone());
                    entries.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));
                    drop(entries);
                    journal.refresh();
                    nav.replace(Route::Post { slug: entry.slug });
                }
                Err(failure) => *status.write() = failure.to_string(),
            }
            *busy.write() = false;
        });
    });

    let target = slug.clone();
    let delete = move |_| {
        let Some(slug) = target.clone() else {
            return;
        };
        if !*armed_delete.peek() {
            *armed_delete.write() = true;
            *status.write() = "press delete again to remove this post".into();
            return;
        }
        let Some(token) = journal.token.peek().clone() else {
            return;
        };
        spawn(async move {
            match api::remove(&slug, &token).await {
                Ok(()) => {
                    journal.entries.write().retain(|entry| entry.slug != slug);
                    journal.refresh();
                    nav.replace(Route::Index {});
                }
                Err(failure) => *status.write() = failure.to_string(),
            }
        });
    };

    let shortcut = move |event: KeyboardEvent| {
        if event.key() == Key::Enter && (event.modifiers().ctrl() || event.modifiers().meta()) {
            event.prevent_default();
            save.call(());
        }
    };

    let preview = Entry {
        slug: slug.clone().unwrap_or_else(|| title.read().clone()),
        kind: EntryKind::Essay,
        title: Some(title()).filter(|title| !title.trim().is_empty()),
        summary: Some(summary()).filter(|summary| !summary.trim().is_empty()),
        body: body(),
        tags: Vec::new(),
        published: published(),
        created_at: existing.as_ref().map_or_else(
            || (js_sys::Date::now() / 1000.0) as i64,
            |entry| entry.created_at,
        ),
        updated_at: 0,
    };
    let html = parse_markdown(&preview.body);
    let editing = slug.is_some();

    rsx! {
        div { class: "composer",
            section { class: "composer__form",
                p { class: "label", if editing { "EDIT POST" } else { "NEW POST" } }
                input {
                    class: "composer__title",
                    placeholder: "title",
                    value: "{title}",
                    oninput: move |event| *title.write() = event.value(),
                    onkeydown: shortcut,
                }
                input {
                    class: "composer__summary",
                    placeholder: "one line summary, shown on the index",
                    value: "{summary}",
                    oninput: move |event| *summary.write() = event.value(),
                    onkeydown: shortcut,
                }
                textarea {
                    class: "composer__body",
                    placeholder: "write in markdown…",
                    value: "{body}",
                    oninput: move |event| *body.write() = event.value(),
                    onkeydown: shortcut,
                }
                input {
                    class: "composer__tags",
                    placeholder: "tags, comma separated",
                    value: "{tags}",
                    oninput: move |event| *tags.write() = event.value(),
                    onkeydown: shortcut,
                }
                div { class: "composer__bar",
                    label { class: "composer__toggle",
                        input {
                            r#type: "checkbox",
                            checked: published(),
                            onchange: move |event| *published.write() = event.checked(),
                        }
                        if published() { "published" } else { "draft" }
                    }
                    span { class: "composer__status", "{status}" }
                    if editing {
                        button { class: "composer__delete", onclick: delete, if armed_delete() { "confirm delete" } else { "delete" } }
                    }
                    button { class: "composer__save", disabled: busy(), onclick: move |_| save.call(()), "→ save" }
                }
                p { class: "composer__hint key-hint", "ctrl+enter save  ·  esc leave" }
            }
            section { class: "composer__preview",
                p { class: "label", "PREVIEW  {preview.reading_minutes()} MIN READ" }
                h1 { if preview.title.is_some() { "{preview.heading()}" } else { "untitled" } }
                if let Some(summary) = preview.summary.clone() {
                    p { class: "post__lede", "{summary}" }
                }
                div { class: "prose", dangerous_inner_html: "{html}" }
            }
        }
    }
}
