use content::{ArtConfig, EXPERIENCES, Experience, PROJECTS, Project, ShapeFamily};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_events::EventListener;
use gloo_timers::callback::{Interval, Timeout};
use ui::Meta;
use wasm_bindgen::JsCast;

mod canvas;
mod eq;
mod gallery;
mod motion;
mod nav;
mod preview;
mod swipe;

fn main() {
    console_log::init_with_level(log::Level::Info).ok();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}

#[derive(Debug, Clone, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Home {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Projects,
    About,
    Experience,
}

impl View {
    fn number(self) -> usize {
        match self {
            Self::About => 1,
            Self::Projects => 2,
            Self::Experience => 3,
        }
    }
}

const PROJECT_LOOP_COPIES: i64 = 9;
const PROJECT_LOOP_CENTER: i64 = PROJECT_LOOP_COPIES / 2;

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

#[derive(Clone, Copy, PartialEq)]
struct DetailDisplay {
    view: View,
    project_index: usize,
    experience_index: usize,
    revision: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailMotion {
    Still,
    Leaving,
    Entering,
}

#[component]
fn Home() -> Element {
    let mut view = use_signal(|| View::About);
    let mut project_index = use_signal(|| 0usize);
    let mut project_cursor = use_signal(|| centered_project_cursor(0));
    let mut experience_index = use_signal(|| 0usize);
    let theme = use_signal(|| Theme::Cobalt);
    let dark = use_signal(initial_dark_mode);
    use_effect(move || apply_dark_mode(dark()));
    use_effect(move || {
        if view() == View::About {
            nav::back_to(0);
        } else {
            nav::push(1);
        }
    });
    use_hook(move || {
        let mut view = view;
        std::rc::Rc::new(EventListener::new(
            &gloo_utils::window(),
            "popstate",
            move |_| {
                if nav::layer() == 0 && *view.peek() != View::About {
                    *view.write() = View::About;
                    if compact_layout() {
                        nav::return_to_anchor();
                    }
                }
            },
        ))
    });
    let mut detail_display = use_signal(|| DetailDisplay {
        view: View::About,
        project_index: 0,
        experience_index: 0,
        revision: 0,
    });
    let mut detail_motion = use_signal(|| DetailMotion::Still);
    let mut detail_sequence = use_signal(|| 0_u64);
    let mut detail_initialized = use_signal(|| false);
    let swipe_origin = use_signal(|| None);

    {
        let view = view;
        let project_index = project_index;
        let experience_index = experience_index;
        use_effect(move || {
            let next_view = view();
            let next_project = project_index();
            let next_experience = experience_index();
            if !*detail_initialized.peek() {
                *detail_initialized.write() = true;
                return;
            }
            if next_view == View::About && detail_display.peek().view == View::About {
                return;
            }

            let sequence = (*detail_sequence.peek()).wrapping_add(1);
            *detail_sequence.write() = sequence;
            *detail_motion.write() = DetailMotion::Leaving;
            Timeout::new(motion::VEIL_COVER_MS, move || {
                if detail_sequence() != sequence {
                    return;
                }
                let previous = *detail_display.peek();
                *detail_display.write() = DetailDisplay {
                    view: next_view,
                    project_index: next_project,
                    experience_index: next_experience,
                    revision: previous.revision.wrapping_add(1),
                };
                *detail_motion.write() = DetailMotion::Entering;
                Timeout::new(motion::VEIL_REVEAL_MS, move || {
                    if detail_sequence() == sequence {
                        *detail_motion.write() = DetailMotion::Still;
                    }
                })
                .forget();
            })
            .forget();
        });
    }

    {
        let mut view = view;
        let mut project_index = project_index;
        let mut project_cursor = project_cursor;
        let mut experience_index = experience_index;
        let mut theme = theme;
        let mut dark = dark;
        use_effect(move || {
            let listener =
                EventListener::new(&gloo_utils::document().into(), "keydown", move |event| {
                    let key = event
                        .dyn_ref::<web_sys::KeyboardEvent>()
                        .expect("keyboard event");
                    if gallery::viewer_open() || key.ctrl_key() || key.meta_key() || key.alt_key() {
                        return;
                    }

                    match key.key().as_str() {
                        "1" | "a" | "A" => *view.write() = View::About,
                        "2" | "p" | "P" => *view.write() = View::Projects,
                        "3" | "e" | "E" => *view.write() = View::Experience,
                        "w" | "W" => {
                            let _ =
                                gloo_utils::window().open_with_url("https://blog.life2harsh.xyz");
                        }
                        "r" | "R" => {
                            let _ = gloo_utils::window().open_with_url("/resume.pdf");
                        }
                        "t" | "T" => *theme.write() = theme().next(),
                        "d" | "D" => {
                            let next = !dark();
                            *dark.write() = next;
                            remember_dark_mode(next);
                        }
                        "m" | "M" => eq::toggle(),
                        "Enter" if view() == View::About => *view.write() = View::Experience,
                        "j" | "ArrowDown" if matches!(view(), View::About | View::Experience) => {
                            *experience_index.write() =
                                (experience_index() + 1) % EXPERIENCES.len();
                        }
                        "k" | "ArrowUp" if matches!(view(), View::About | View::Experience) => {
                            *experience_index.write() =
                                (experience_index() + EXPERIENCES.len() - 1) % EXPERIENCES.len();
                        }
                        "j" | "ArrowDown" => {
                            let next = project_cursor() + 1;
                            *project_cursor.write() = next;
                            *project_index.write() = project_index_for_cursor(next);
                            *view.write() = View::Projects;
                        }
                        "k" | "ArrowUp" => {
                            let next = project_cursor() - 1;
                            *project_cursor.write() = next;
                            *project_index.write() = project_index_for_cursor(next);
                            *view.write() = View::Projects;
                        }
                        "g" => {
                            let next = centered_project_cursor(0);
                            *project_cursor.write() = next;
                            *project_index.write() = 0;
                        }
                        "G" => {
                            let next = centered_project_cursor(PROJECTS.len() - 1);
                            *project_cursor.write() = next;
                            *project_index.write() = PROJECTS.len() - 1;
                        }
                        "Enter" if view() == View::Projects => {
                            if let Some(link) = PROJECTS[project_index()].links.first() {
                                let _ = gloo_utils::window().open_with_url(link.url);
                            }
                        }
                        "Escape" => *view.write() = View::About,
                        _ => return,
                    }
                    key.prevent_default();
                });
            std::mem::forget(listener);
        });
    }

    let active = view();
    let selected_project = &PROJECTS[project_index()];
    let selected_experience = &EXPERIENCES[experience_index()];
    let displayed = detail_display();
    let art = match active {
        View::About => about_art(),
        View::Projects => selected_project.art.clone(),
        View::Experience => selected_experience.art.clone(),
    };
    let detail_class = match detail_motion() {
        DetailMotion::Still => "left-detail",
        DetailMotion::Leaving => "left-detail is-leaving",
        DetailMotion::Entering => "left-detail is-entering",
    };

    rsx! {
        Meta {
            title: "life2harsh.xyz".to_string(),
            description: "Harsh Jha — systems software, WebAssembly, terminal interfaces and media tooling.".to_string(),
        }
        main { class: "app {theme().class()}",
            AppHeader { active, on_view: move |next| *view.write() = next }
            section { class: "app-main",
                section { class: "shell-left",
                    div { class: "left-upper",
                        IdentityBlock { active }
                        canvas::ArtCanvas { config: art, accent: theme().art_tint(), ink: if dark() { DARK_INK } else { LIGHT_INK } }
                    }
                    div {
                        class: "{detail_class}",
                        aria_live: "polite",
                        ontouchstart: move |event| swipe::begin(swipe_origin, &event),
                        ontouchend: move |event| {
                            let Some(delta) = swipe::end(swipe_origin, &event).and_then(|gesture| gesture.step()) else {
                                return;
                            };
                            match view() {
                                View::Projects => {
                                    let next = project_cursor() + delta;
                                    *project_cursor.write() = next;
                                    *project_index.write() = project_index_for_cursor(next);
                                }
                                View::About | View::Experience => {
                                    let count = EXPERIENCES.len() as i64;
                                    *experience_index.write() =
                                        (experience_index() as i64 + delta).rem_euclid(count) as usize;
                                }
                            }
                        },
                        div { class: "detail-stage", key: "{displayed.view:?}-{displayed.project_index}-{displayed.experience_index}-{displayed.revision}",
                            match displayed.view {
                                View::Projects => rsx! { ProjectDetail { project: &PROJECTS[displayed.project_index] } },
                                View::About => rsx! {
                                    AboutDetail {
                                        selected: experience_index(),
                                        on_hover: move |index| *experience_index.write() = index,
                                        on_experience: move |index| {
                                            nav::remember(format!("exp-row-{index}"));
                                            *experience_index.write() = index;
                                            *view.write() = View::Experience;
                                        },
                                    }
                                },
                                View::Experience => rsx! { ExperienceDetail { item: &EXPERIENCES[displayed.experience_index], index: displayed.experience_index } },
                            }
                        }
                        motion::PixelVeil { motion: detail_motion() }
                    }
                }
                aside { class: "shell-right",
                    div { class: "right-navigator",
                        ProjectNavigator {
                            cursor: project_cursor,
                            active: active == View::Projects,
                            on_select: move |cursor| {
                                *project_cursor.write() = cursor;
                                *project_index.write() = project_index_for_cursor(cursor);
                                *view.write() = View::Projects;
                                nav::remember(format!("project-row-{cursor}"));
                                reveal_detail();
                            },
                        }
                    }
                    LinkList { on_view: move |next| *view.write() = next }
                }
            }
            StatusBar {
                theme: theme(),
                dark: dark(),
                on_theme: move |_| {
                    let mut theme = theme;
                    *theme.write() = theme().next();
                },
                on_dark: move |_| {
                    let mut dark = dark;
                    let next = !dark();
                    *dark.write() = next;
                    remember_dark_mode(next);
                },
            }
        }
    }
}

fn compact_layout() -> bool {
    gloo_utils::window()
        .match_media("(max-width: 900px)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}

fn reveal_detail() {
    if !compact_layout() {
        return;
    }
    if let Ok(Some(detail)) = gloo_utils::document().query_selector(".left-detail") {
        detail.scroll_into_view();
    }
}

fn project_index_for_cursor(cursor: i64) -> usize {
    cursor.rem_euclid(PROJECTS.len() as i64) as usize
}

fn centered_project_cursor(index: usize) -> i64 {
    PROJECTS.len() as i64 * PROJECT_LOOP_CENTER + index as i64
}

const LIGHT_INK: &str = "#141413";
const DARK_INK: &str = "#e6e3d9";
const MODE_KEY: &str = "life2harsh.mode";

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
        _ => window
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
            .is_some_and(|query| query.matches()),
    }
}

fn remember_dark_mode(dark: bool) {
    if let Some(storage) = gloo_utils::window().local_storage().ok().flatten() {
        let _ = storage.set_item(MODE_KEY, if dark { "dark" } else { "light" });
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

fn about_art() -> ArtConfig {
    ArtConfig {
        shape: ShapeFamily::Knot,
        rotation_x: 0.52,
        rotation_y: -0.24,
        twist: 2.1,
        radius: 1.0,
        secondary_radius: 0.32,
        density: 72,
        warp: 0.12,
        symmetry: 3.0,
        phase: 0.2,
        scale: 0.94,
        seed: 84921,
    }
}

#[derive(Clone, PartialEq, Props)]
struct AppHeaderProps {
    active: View,
    on_view: EventHandler<View>,
}

#[component]
fn AppHeader(props: AppHeaderProps) -> Element {
    rsx! {
        header { class: "app-header",
            a { class: "app-header__brand", href: "/", "life2harsh.xyz" }
            eq::Equalizer {}
            nav { class: "app-header__links", aria_label: "Primary navigation",
                button { class: if props.active == View::Projects { "is-active" } else { "" }, onclick: move |_| props.on_view.call(View::Projects), span { class: "key-hint", "[P] " } "projects" }
                a { href: "https://blog.life2harsh.xyz", target: "_blank", rel: "noopener noreferrer", span { class: "key-hint", "[W] " } "writing" }
                button { class: if props.active == View::About { "is-active" } else { "" }, onclick: move |_| props.on_view.call(View::About), span { class: "key-hint", "[A] " } "about" }
                a { href: "/resume.pdf", target: "_blank", span { class: "key-hint", "[R] " } "résumé" }
            }
        }
    }
}

fn kolkata_clock() -> String {
    let offset_ms = 5.5 * 60.0 * 60.0 * 1_000.0;
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
        js_sys::Date::now() + offset_ms,
    ));
    format!("{:02}:{:02}", date.get_utc_hours(), date.get_utc_minutes())
}

#[derive(Clone, PartialEq, Props)]
struct IdentityBlockProps {
    active: View,
}

#[component]
fn IdentityBlock(props: IdentityBlockProps) -> Element {
    rsx! {
        section { class: "identity",
            p { class: "micro", "{props.active.number():02}  ||  03" }
            h1 { class: "identity__headline",
                "i make software and other things computers probably didn’t ask for."
                span { class: "block-cursor", aria_hidden: "true" }
            }
            hr { class: "short-rule" }
            p { class: "identity__copy", "systems, interfaces, creative tools, and weird experiments." }
            dl { class: "kv",
                div { dt { "LOCATION" } dd { "somewhere on earth" } }
                div { dt { "STACK" } dd { "rust || web || systems" } }
                div { dt { "STATUS" } dd { "searching for a W" } }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct ProjectDetailProps {
    project: &'static Project,
}

#[component]
fn ProjectDetail(props: ProjectDetailProps) -> Element {
    let project = props.project;
    let github = project.links.first();
    rsx! {
        article { class: "detail",
            div { class: "detail__main",
                button { class: "detail__back", onclick: move |_| nav::back_to(0), "← go back" }
                p { class: "label", "SELECTED PROJECT" }
                h2 { class: "step-in", "{project.name.to_lowercase()}" }
                p { class: "detail__summary", "{project.description}" }
                dl { class: "kv kv--detail",
                    div { dt { "YEAR" } dd { "{project.year}" } }
                    div { dt { "STATUS" } dd { "{project.status.label()}" } }
                    div { dt { "STACK" } dd { class: "chips", for item in project.stack { span { class: "stack-chip", "{item}" } } } }
                }
                if let Some(long) = project.long_description {
                    p { class: "detail__long", "{long}" }
                }
                nav { class: "detail__links", aria_label: "Project links",
                    if let Some(link) = github {
                        a { href: "{link.url}", target: "_blank", rel: "noopener noreferrer", span { "→" } u { "view on github" } }
                    }
                    if let Some(live) = project.live_url {
                        a { href: "{live}", target: "_blank", rel: "noopener noreferrer", span { "→" } u { "live site" } }
                    }
                }
            }
            if !project.previews.is_empty() {
                gallery::Gallery { project }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct AboutDetailProps {
    selected: usize,
    on_hover: EventHandler<usize>,
    on_experience: EventHandler<usize>,
}

#[component]
fn AboutDetail(props: AboutDetailProps) -> Element {
    rsx! {
        article { class: "detail",
            div { class: "detail__main",
                p { class: "label", "ABOUT" }
                h2 { class: "step-in", "harsh jha" }
                p { class: "detail__summary", "Developer, student and epistemically curious. I like terminal tools, systems software, WebAssembly and anything that turns math into pictures." }
                p { class: "label label--spaced", "EXPERIENCE" }
                div { class: "exp-list",
                    for (index, item) in EXPERIENCES.iter().enumerate() {
                        button {
                            id: "exp-row-{index}",
                            class: if index == props.selected { "exp-row is-selected" } else { "exp-row" },
                            onclick: move |_| props.on_experience.call(index),
                            onmouseenter: move |_| props.on_hover.call(index),
                            span { class: "exp-row__pointer", aria_hidden: "true", ">" }
                            strong { "{item.organization}" }
                            small { "{item.role}" }
                            small { class: "exp-row__period", "{item.period}" }
                        }
                    }
                }
                nav { class: "detail__links", aria_label: "Profile links",
                    a { href: "https://github.com/life2harsh", target: "_blank", rel: "noopener noreferrer", span { "→" } u { "github" } }
                    a { href: "https://linkedin.com/in/life2harsh", target: "_blank", rel: "noopener noreferrer", span { "→" } u { "linkedin" } }
                    a { href: "https://x.com/life2harsh", target: "_blank", rel: "noopener noreferrer", span { "→" } u { "x" } }
                    a { href: "mailto:jhaharsh29@gmail.com", span { "→" } u { "email" } }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct ExperienceDetailProps {
    item: &'static Experience,
    index: usize,
}

#[component]
fn ExperienceDetail(props: ExperienceDetailProps) -> Element {
    let item = props.item;
    rsx! {
        article { class: "detail",
            div { class: "detail__main",
                button { class: "detail__back", onclick: move |_| nav::back_to(0), "← go back" }
                p { class: "label", "EXPERIENCE  {props.index + 1:02} || {EXPERIENCES.len():02}" }
                h2 { class: "step-in", "{item.organization.to_lowercase()}" }
                p { class: "detail__summary", "{item.role}" }
                dl { class: "kv kv--detail",
                    div { dt { "PERIOD" } dd { "{item.period}" } }
                    div { dt { "STACK" } dd { class: "chips", for tech in item.stack { span { class: "stack-chip", "{tech}" } } } }
                }
                p { class: "detail__long", "{item.description}" }
                p { class: "detail__hint key-hint", "j || k  cycle experience  ·  esc  back" }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct ProjectNavigatorProps {
    cursor: Signal<i64>,
    active: bool,
    on_select: EventHandler<i64>,
}

#[component]
fn ProjectNavigator(props: ProjectNavigatorProps) -> Element {
    let list_element = use_signal(|| None::<web_sys::Element>);
    let mut plate_element = use_signal(|| None::<web_sys::Element>);
    let selected_cursor = (props.cursor)();
    {
        let mut cursor = props.cursor;
        use_effect(move || {
            let Some(element) = list_element() else {
                return;
            };
            let selected = cursor();
            if let Some(rebased) = rebase_project_track(&element, selected) {
                *cursor.write() = rebased;
                return;
            }
            scroll_cursor_into_view(element, selected);
        });
    }
    {
        use_effect(move || {
            let Some(plate) = plate_element() else {
                return;
            };
            let cursor = (props.cursor)();
            let row_cursor = if compact_layout() {
                centered_project_cursor(project_index_for_cursor(cursor))
            } else {
                cursor
            };
            let row_id = format!("project-row-{row_cursor}");
            let Some(row) = gloo_utils::document().get_element_by_id(&row_id) else {
                return;
            };
            motion::retarget_highlight(&plate, &row);
        });
    }
    rsx! {
        section { class: if props.active { "navigator" } else { "navigator navigator--idle" },
            header { class: "navigator__header",
                h2 { "PROJECTS" }
                span { "{PROJECTS.len():02} ITEMS" }
            }
            div { class: "navigator__list", role: "listbox", aria_label: "Projects",
                onmounted: move |event| {
                    let element: web_sys::Element = event
                        .as_web_event()
                        .dyn_into()
                        .expect("project navigator mount target");
                    let cursor = props.cursor;
                    let mut mounted_list = list_element;
                    spawn(async move {
                        for _ in 0..120 {
                            let scrollable = element.client_height() > 0
                                && element.scroll_height() > element.client_height();
                            if scrollable {
                                center_project_track(&element, *cursor.peek());
                                if element.scroll_top() > 0 || *cursor.peek() == 0 {
                                    break;
                                }
                            }
                            gloo_timers::future::TimeoutFuture::new(16).await;
                        }
                        *mounted_list.write() = Some(element);
                    });
                },
                div { class: "navigator__highlight", aria_hidden: "true",
                    onmounted: move |event| {
                        let element: web_sys::Element = event
                            .as_web_event()
                            .dyn_into()
                            .expect("project highlight mount target");
                        *plate_element.write() = Some(element);
                    }
                }
                for loop_index in 0..PROJECT_LOOP_COPIES {
                    for (index, project) in PROJECTS.iter().enumerate() {
                        ProjectRow {
                            index,
                            cursor: loop_index * PROJECTS.len() as i64 + index as i64,
                            primary: loop_index == PROJECT_LOOP_CENTER,
                            project,
                            selected: loop_index * PROJECTS.len() as i64 + index as i64 == selected_cursor,
                            on_select: props.on_select,
                        }
                    }
                }
            }
        }
    }
}

fn center_project_track(element: &web_sys::Element, cursor: i64) {
    let total_height = element.scroll_height();
    let viewport_height = element.client_height();
    if total_height == 0 || viewport_height == 0 {
        return;
    }
    let row_height = total_height as f64 / (PROJECTS.len() as i64 * PROJECT_LOOP_COPIES) as f64;
    let target =
        (row_height * cursor as f64).clamp(0.0, (total_height - viewport_height).max(0) as f64);
    element.set_scroll_top(target.round() as i32);
}

fn rebase_project_track(element: &web_sys::Element, cursor: i64) -> Option<i64> {
    let project_count = PROJECTS.len() as i64;
    let row_height = element.scroll_height() as f64 / (project_count * PROJECT_LOOP_COPIES) as f64;
    let shift = project_count * PROJECT_LOOP_CENTER;
    if row_height <= 0.0 {
        return None;
    }
    if cursor >= project_count * (PROJECT_LOOP_COPIES - 2) {
        element.set_scroll_top(
            (element.scroll_top() as f64 - row_height * shift as f64).round() as i32,
        );
        Some(cursor - shift)
    } else if cursor < project_count {
        element.set_scroll_top(
            (element.scroll_top() as f64 + row_height * shift as f64).round() as i32,
        );
        Some(cursor + shift)
    } else {
        None
    }
}

fn scroll_cursor_into_view(element: web_sys::Element, cursor: i64) {
    let total_height = element.scroll_height();
    let viewport_height = element.client_height();
    if total_height == 0 || viewport_height == 0 {
        return;
    }
    let row_height = total_height as f64 / (PROJECTS.len() as i64 * PROJECT_LOOP_COPIES) as f64;
    let selected_top = row_height * cursor as f64;
    let selected_bottom = selected_top + row_height;
    let current_top = element.scroll_top() as f64;
    let current_bottom = current_top + viewport_height as f64;
    let comfort = row_height * 0.6;
    let target = if selected_top < current_top + comfort {
        (selected_top - comfort).max(0.0)
    } else if selected_bottom > current_bottom - comfort {
        (selected_bottom - viewport_height as f64 + comfort)
            .min((total_height - viewport_height).max(0) as f64)
    } else {
        return;
    };
    motion::retarget_scroll(&element, target);
}

#[derive(Clone, PartialEq, Props)]
struct ProjectRowProps {
    index: usize,
    cursor: i64,
    primary: bool,
    project: &'static Project,
    selected: bool,
    on_select: EventHandler<i64>,
}

#[component]
fn ProjectRow(props: ProjectRowProps) -> Element {
    let stack = props
        .project
        .stack
        .iter()
        .take(3)
        .map(|item| item.to_lowercase())
        .collect::<Vec<_>>()
        .join(" || ");
    rsx! {
        button {
            id: "project-row-{props.cursor}",
            class: match (props.selected, props.primary) {
                (true, true) => "project-row is-selected is-primary",
                (true, false) => "project-row is-selected",
                (false, true) => "project-row is-primary",
                (false, false) => "project-row",
            },
            role: "option",
            aria_selected: "{props.selected}",
            onclick: move |_| props.on_select.call(props.cursor),
            span { class: "project-row__number", "{props.index + 1:02}" }
            span { class: "project-row__copy",
                strong { "{props.project.name.to_lowercase()}" }
                small { "{stack}" }
            }
            span { class: "project-row__year", "{props.project.year}" }
            span { class: "project-row__arrow", "→" }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct LinkListProps {
    on_view: EventHandler<View>,
}

#[component]
fn LinkList(props: LinkListProps) -> Element {
    rsx! {
        nav { class: "link-list", aria_label: "Sections",
            a { href: "https://blog.life2harsh.xyz", target: "_blank", rel: "noopener noreferrer",
                span { "→" } strong { "writing" } small { "notes, ideas, experiments" }
            }
            button { onclick: move |_| props.on_view.call(View::About),
                span { "→" } strong { "about" } small { "who i am, what i do" }
            }
            a { href: "/resume.pdf", target: "_blank",
                span { "→" } strong { "résumé" } small { "pdf, experience, etc" }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct StatusBarProps {
    theme: Theme,
    dark: bool,
    on_theme: EventHandler<()>,
    on_dark: EventHandler<()>,
}

#[component]
fn StatusBar(props: StatusBarProps) -> Element {
    let clock = use_signal(kolkata_clock);
    {
        let mut clock = clock;
        use_effect(move || {
            let interval = Interval::new(10_000, move || *clock.write() = kolkata_clock());
            std::mem::forget(interval);
        });
    }
    rsx! {
        footer { class: "status",
            span { class: "status__touch", "⇆ swipe" }
            span { "↑↓ navigate" }
            span { "enter open" }
            span { "a about" }
            span { "w writing" }
            span { "r résumé" }
            span { "t theme" }
            span { "m music" }
            span { if props.dark { "d light" } else { "d dark" } }
            button { class: "status__right", onclick: move |_| props.on_theme.call(()), "THEME: {props.theme.label()}" }
            span { class: "status__sep", "|" }
            button { onclick: move |_| props.on_dark.call(()), if props.dark { "DARK" } else { "LIGHT" } }
            span { class: "status__sep", "|" }
            span { "{clock}" }
        }
    }
}
