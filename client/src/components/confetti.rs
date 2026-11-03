//! Confetti, for the moments worth celebrating: a burst where a vote lands,
//! a shower at a milestone. Plain elements animated by CSS (`.confetti`),
//! which remove themselves; nothing for those who ask for less motion.

const COLOURS: [&str; 6] = [
    "#f5b436", "#742a85", "#ff6b9a", "#2ec4b6", "#4dabf7", "#ff922b",
];

/// A burst of `n` pieces from the point (`x`, `y`) of the viewport.
pub fn burst(x: f64, y: f64, n: usize) {
    for _ in 0..n {
        let angle = rand() * std::f64::consts::TAU;
        let speed = 60.0 + rand() * 110.0;
        let (dx, dy) = (angle.cos() * speed, angle.sin() * speed - 40.0);
        spawn(
            x,
            y,
            &format!(
                "--dx:{dx:.0}px;--dy:{dy:.0}px;--fall:{:.0}px",
                80.0 + rand() * 60.0
            ),
            "confetti burst",
            900.0 + rand() * 400.0,
        );
    }
}

/// A shower of `n` pieces over the whole width.
pub fn shower(n: usize) {
    let width = web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|w| w.as_f64())
        .unwrap_or(400.0);
    for _ in 0..n {
        let x = rand() * width;
        let sway = (rand() - 0.5) * 160.0;
        spawn(
            x,
            -20.0,
            &format!("--dx:{sway:.0}px;--delay:{:.0}ms", rand() * 700.0),
            "confetti shower",
            2200.0 + rand() * 1200.0,
        );
    }
}

/// A short buzz under the finger, where the device can: Safari has no
/// `navigator.vibrate`, and calling it there throws.
pub fn tap() {
    let Some(navigator) = web_sys::window().map(|w| w.navigator()) else {
        return;
    };
    if js_sys::Reflect::has(&navigator, &"vibrate".into()).unwrap_or(false) {
        let _ = navigator.vibrate_with_duration(12);
    }
}

fn spawn(x: f64, y: f64, vars: &str, class: &str, life_ms: f64) {
    if reduced_motion() {
        return;
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(piece) = doc.create_element("i") else {
        return;
    };
    piece.set_class_name(class);
    let colour = COLOURS[(rand() * COLOURS.len() as f64) as usize % COLOURS.len()];
    let _ = piece.set_attribute(
        "style",
        &format!(
            "left:{x:.0}px;top:{y:.0}px;background:{colour};--spin:{:.0}deg;--life:{life_ms:.0}ms;{vars}",
            (rand() - 0.5) * 1080.0
        ),
    );
    let Some(body) = doc.body() else { return };
    let _ = body.append_child(&piece);
    crate::timeout::Timeout::new(life_ms as u32 + 800, move || piece.remove()).forget();
}

fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches())
}

fn rand() -> f64 {
    js_sys::Math::random()
}

/// The centre of the element `id` in the viewport, if it is there.
pub fn centre_of(id: &str) -> Option<(f64, f64)> {
    let el = web_sys::window()?.document()?.get_element_by_id(id)?;
    let rect = el.get_bounding_client_rect();
    Some((
        rect.left() + rect.width() / 2.0,
        rect.top() + rect.height() * 0.4,
    ))
}
