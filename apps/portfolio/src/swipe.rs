use dioxus::prelude::*;

const MIN_DISTANCE: f64 = 45.0;

pub(crate) type Origin = Signal<Option<(f64, f64)>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gesture {
    Next,
    Previous,
    Down,
}

impl Gesture {
    pub(crate) fn step(self) -> Option<i64> {
        match self {
            Self::Next => Some(1),
            Self::Previous => Some(-1),
            Self::Down => None,
        }
    }
}

pub(crate) fn begin(mut origin: Origin, event: &TouchEvent) {
    *origin.write() = event.touches().first().map(|touch| {
        let point = touch.client_coordinates();
        (point.x, point.y)
    });
}

pub(crate) fn end(mut origin: Origin, event: &TouchEvent) -> Option<Gesture> {
    let (start_x, start_y) = origin.write().take()?;
    let point = event.touches_changed().first()?.client_coordinates();
    let (dx, dy) = (point.x - start_x, point.y - start_y);
    if dx.abs() >= MIN_DISTANCE && dx.abs() >= dy.abs() * 1.5 {
        return Some(if dx < 0.0 {
            Gesture::Next
        } else {
            Gesture::Previous
        });
    }
    if dy >= MIN_DISTANCE * 1.6 && dy >= dx.abs() * 1.5 {
        return Some(Gesture::Down);
    }
    None
}
