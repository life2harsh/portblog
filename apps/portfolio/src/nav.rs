use wasm_bindgen::JsValue;

fn history() -> Option<web_sys::History> {
    gloo_utils::window().history().ok()
}

pub(crate) fn layer() -> u32 {
    history()
        .and_then(|history| history.state().ok())
        .and_then(|state| js_sys::Reflect::get(&state, &JsValue::from_str("layer")).ok())
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0) as u32
}

pub(crate) fn push(target: u32) {
    if layer() >= target {
        return;
    }
    let state = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&state, &JsValue::from_str("layer"), &JsValue::from(target));
    if let Some(history) = history() {
        let _ = history.set_scroll_restoration(web_sys::ScrollRestoration::Manual);
        let _ = history.push_state(&state, "");
    }
}

pub(crate) fn back_to(target: u32) {
    if layer() > target
        && let Some(history) = history()
    {
        let _ = history.back();
    }
}

thread_local! {
    static ANCHOR: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn remember(anchor: String) {
    ANCHOR.with(|slot| *slot.borrow_mut() = Some(anchor));
}

pub(crate) fn return_to_anchor() {
    let Some(anchor) = ANCHOR.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    gloo_timers::callback::Timeout::new(380, move || {
        let window = gloo_utils::window();
        let Some(element) = gloo_utils::document().get_element_by_id(&anchor) else {
            return;
        };
        let top = element.get_bounding_client_rect().top();
        let viewport = window
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let current = window.scroll_y().unwrap_or(0.0);
        window.scroll_to_with_x_and_y(0.0, (current + top - viewport / 3.0).max(0.0));
    })
    .forget();
}
