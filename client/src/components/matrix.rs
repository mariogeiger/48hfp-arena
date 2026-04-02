use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

const CELL: f64 = 20.0;
const HEADER: f64 = 70.0;
const FONT: &str = "7px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
const CELL_FONT: &str = "bold 8px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";

struct Colors {
    bg: String,
    card_alt: String,
    text: String,
    text_muted: String,
    win: String,
    win_bg: String,
    loss: String,
    loss_bg: String,
    diag: String,
    legacy: String,
    legacy_bg: String,
}

fn get_colors() -> Colors {
    let s = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .document_element()
        .unwrap();
    let cs = web_sys::window()
        .unwrap()
        .get_computed_style(&s)
        .unwrap()
        .unwrap();
    let g = |name: &str| {
        cs.get_property_value(name)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    Colors {
        bg: g("--bg"),
        card_alt: g("--card-alt"),
        text: g("--text"),
        text_muted: g("--text-muted"),
        win: g("--win"),
        win_bg: g("--win-bg"),
        loss: g("--loss"),
        loss_bg: g("--loss-bg"),
        diag: g("--matrix-diag"),
        legacy: g("--matrix-legacy"),
        legacy_bg: g("--matrix-legacy-bg"),
    }
}

fn short_title(t: &str) -> String {
    let mut chars = t.char_indices();
    if let Some((byte_pos, _)) = chars.nth(11) {
        format!("{}\u{2026}", &t[..byte_pos])
    } else {
        t.to_string()
    }
}

#[derive(Clone)]
pub enum CellBg {
    Win,
    Loss,
    Legacy,
    Empty,
    Residual(f64),
}

#[derive(Clone)]
pub struct CellInfo {
    pub bg: CellBg,
    pub text: String,
}

pub struct MatrixConfig {
    pub films: Vec<MatrixFilmInfo>,
    pub cell_info: Box<dyn Fn(usize, usize) -> CellInfo>,
    pub tooltip: Box<dyn Fn(usize, usize) -> String>,
    pub on_click: Option<Box<dyn Fn(usize, usize)>>,
}

#[derive(Clone)]
pub struct MatrixFilmInfo {
    pub title: String,
}

fn residual_color(r: f64) -> String {
    let t = (r.abs() * 3.0).min(1.0);
    if r > 0.0 {
        format!("rgba(39,174,96,{})", t * 0.6)
    } else if r < 0.0 {
        format!("rgba(231,76,60,{})", t * 0.6)
    } else {
        "#e8e4ed".to_string()
    }
}

pub fn render_matrix_canvas(container_id: &str, config: MatrixConfig) {
    let doc = web_sys::window().unwrap().document().unwrap();
    let container = match doc.get_element_by_id(container_id) {
        Some(el) => el,
        None => return,
    };

    let n = config.films.len();
    let w = HEADER + n as f64 * CELL;
    let h = HEADER + n as f64 * CELL;

    // iOS Safari caps canvas area at ~16.7M pixels
    let max_area: f64 = 16777216.0;
    let dpr = {
        let raw = web_sys::window().unwrap().device_pixel_ratio();
        if w * raw * h * raw > max_area {
            let d = (max_area / (w * h)).sqrt().floor();
            if d < 1.0 { 1.0 } else { d }
        } else {
            raw
        }
    };

    container.set_inner_html("");
    let canvas: HtmlCanvasElement = doc.create_element("canvas").unwrap().dyn_into().unwrap();
    canvas.set_class_name("matrix-canvas");
    let _ = canvas.style().set_property("width", &format!("{}px", w));
    let _ = canvas.style().set_property("height", &format!("{}px", h));
    canvas.set_width((w * dpr) as u32);
    canvas.set_height((h * dpr) as u32);
    container.append_child(&canvas).unwrap();

    // Top scrollbar
    let wrapper = container.closest(".matrix-scroll-wrapper").ok().flatten();
    if let Some(ref wrapper) = wrapper {
        setup_scroll(wrapper, w);
    }

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();
    ctx.scale(dpr, dpr).unwrap();
    let c = get_colors();

    // Background
    ctx.set_fill_style_str(&c.bg);
    ctx.fill_rect(0.0, 0.0, w, h);

    // Corner label
    ctx.set_fill_style_str(&c.text_muted);
    ctx.set_font(FONT);
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let _ = ctx.fill_text("\u{2193} beat \u{2192}", HEADER / 2.0, HEADER / 2.0);

    // Column headers (rotated)
    ctx.save();
    ctx.set_font(FONT);
    ctx.set_text_align("left");
    ctx.set_text_baseline("middle");
    ctx.set_fill_style_str(&c.text_muted);
    for i in 0..n {
        let x = HEADER + i as f64 * CELL + CELL / 2.0;
        ctx.save();
        ctx.translate(x, HEADER - 3.0).unwrap();
        ctx.rotate(-std::f64::consts::FRAC_PI_2).unwrap();
        let _ = ctx.fill_text(&short_title(&config.films[i].title), 0.0, 0.0);
        ctx.restore();
    }
    ctx.restore();

    // Row headers
    ctx.set_font(FONT);
    ctx.set_text_align("right");
    ctx.set_text_baseline("middle");
    ctx.set_fill_style_str(&c.text_muted);
    for i in 0..n {
        let y = HEADER + i as f64 * CELL + CELL / 2.0;
        let _ = ctx.fill_text(&short_title(&config.films[i].title), HEADER - 4.0, y);
    }

    // Grid lines
    ctx.set_stroke_style_str(&c.card_alt);
    ctx.set_line_width(0.5);
    for i in 0..=n {
        let pos = HEADER + i as f64 * CELL;
        ctx.begin_path();
        ctx.move_to(HEADER, pos);
        ctx.line_to(w, pos);
        ctx.stroke();
        ctx.begin_path();
        ctx.move_to(pos, HEADER);
        ctx.line_to(pos, h);
        ctx.stroke();
    }

    // Cells
    ctx.set_font(CELL_FONT);
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    for ri in 0..n {
        for ci in 0..n {
            let x = HEADER + ci as f64 * CELL;
            let y = HEADER + ri as f64 * CELL;
            if ri == ci {
                ctx.set_fill_style_str(&c.diag);
                ctx.fill_rect(x, y, CELL, CELL);
                continue;
            }
            let info = (config.cell_info)(ri, ci);
            let bg = match &info.bg {
                CellBg::Win => Some(c.win_bg.clone()),
                CellBg::Loss => Some(c.loss_bg.clone()),
                CellBg::Legacy => Some(c.legacy_bg.clone()),
                CellBg::Empty => None,
                CellBg::Residual(r) => Some(residual_color(*r)),
            };
            if let Some(bg) = bg {
                ctx.set_fill_style_str(&bg);
                ctx.fill_rect(x, y, CELL, CELL);
            }
            if !info.text.is_empty() {
                let fg = match &info.bg {
                    CellBg::Win => &c.win,
                    CellBg::Loss => &c.loss,
                    CellBg::Legacy => &c.legacy,
                    CellBg::Empty => &c.text,
                    CellBg::Residual(_) => &c.text,
                };
                ctx.set_fill_style_str(fg);
                let _ = ctx.fill_text(&info.text, x + CELL / 2.0, y + CELL / 2.0);
            }
        }
    }

    // Tooltip + click handling
    let tooltip_el = get_or_create_tooltip();
    let n_copy = n;
    let _films = config.films.clone();
    let has_click = config.on_click.is_some();

    // Store callbacks in Rc for sharing between closures
    let tooltip_fn = std::rc::Rc::new(config.tooltip);
    let click_fn = std::rc::Rc::new(config.on_click);

    let canvas_ref = canvas.clone();
    let tooltip_ref = tooltip_el.clone();
    let tooltip_fn2 = tooltip_fn.clone();
    let on_mousemove =
        Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let rect = canvas_ref.get_bounding_client_rect();
            let mx = e.client_x() as f64 - rect.left();
            let my = e.client_y() as f64 - rect.top();
            let ci = ((mx - HEADER) / CELL).floor() as i32;
            let ri = ((my - HEADER) / CELL).floor() as i32;
            if ci >= 0 && ci < n_copy as i32 && ri >= 0 && ri < n_copy as i32 && ci != ri {
                let text = (tooltip_fn2)(ri as usize, ci as usize);
                tooltip_ref.set_text_content(Some(&text));
                let _ = tooltip_ref.style().set_property("display", "");
                let _ = tooltip_ref
                    .style()
                    .set_property("left", &format!("{}px", e.client_x() + 12));
                let _ = tooltip_ref
                    .style()
                    .set_property("top", &format!("{}px", e.client_y() + 12));
                if has_click {
                    let _ = canvas_ref.style().set_property("cursor", "pointer");
                }
            } else {
                let _ = tooltip_ref.style().set_property("display", "none");
                let _ = canvas_ref.style().set_property("cursor", "default");
            }
        });
    canvas
        .add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref())
        .unwrap();
    on_mousemove.forget();

    let canvas_ref2 = canvas.clone();
    let tooltip_ref2 = tooltip_el.clone();
    let on_leave = Closure::<dyn Fn()>::new(move || {
        let _ = tooltip_ref2.style().set_property("display", "none");
    });
    canvas_ref2
        .add_event_listener_with_callback("mouseleave", on_leave.as_ref().unchecked_ref())
        .unwrap();
    on_leave.forget();

    if click_fn.is_some() {
        let canvas_ref3 = canvas.clone();
        let on_click =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let rect = canvas_ref3.get_bounding_client_rect();
                let mx = e.client_x() as f64 - rect.left();
                let my = e.client_y() as f64 - rect.top();
                let ci = ((mx - HEADER) / CELL).floor() as i32;
                let ri = ((my - HEADER) / CELL).floor() as i32;
                if ci >= 0
                    && ci < n_copy as i32
                    && ri >= 0
                    && ri < n_copy as i32
                    && ci != ri
                    && let Some(ref f) = *click_fn
                {
                    f(ri as usize, ci as usize);
                }
            });
        canvas
            .add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())
            .unwrap();
        on_click.forget();
    }
}

fn get_or_create_tooltip() -> web_sys::HtmlElement {
    let doc = web_sys::window().unwrap().document().unwrap();
    if let Some(el) = doc.get_element_by_id("matrix-tooltip") {
        return el.dyn_into().unwrap();
    }
    let el = doc.create_element("div").unwrap();
    el.set_id("matrix-tooltip");
    let el: web_sys::HtmlElement = el.dyn_into().unwrap();
    let _ = el.style().set_property("display", "none");
    doc.body().unwrap().append_child(&el).unwrap();
    el
}

fn setup_scroll(wrapper: &web_sys::Element, content_width: f64) {
    // Remove old top scrollbar if present
    if let Some(prev) = wrapper.previous_element_sibling()
        && prev.class_list().contains("matrix-top-scroll")
    {
        prev.remove();
    }

    let doc = web_sys::window().unwrap().document().unwrap();
    let top_bar = doc.create_element("div").unwrap();
    top_bar.set_class_name("matrix-top-scroll");
    let spacer = doc.create_element("div").unwrap();
    let spacer_el: web_sys::HtmlElement = spacer.dyn_into().unwrap();
    let _ = spacer_el
        .style()
        .set_property("width", &format!("{}px", content_width));
    let _ = spacer_el.style().set_property("height", "1px");
    top_bar.append_child(&spacer_el).unwrap();
    if let Some(parent) = wrapper.parent_node() {
        let _ = parent.insert_before(&top_bar, Some(wrapper));
    }

    // Sync scrolls
    let wrapper_el: web_sys::HtmlElement = wrapper.clone().dyn_into().unwrap();
    let top_el: web_sys::HtmlElement = top_bar.clone().dyn_into().unwrap();

    let w2 = wrapper_el.clone();
    let t2 = top_el.clone();
    let on_top_scroll = Closure::<dyn Fn()>::new(move || {
        w2.set_scroll_left(t2.scroll_left());
    });
    top_bar
        .add_event_listener_with_callback("scroll", on_top_scroll.as_ref().unchecked_ref())
        .unwrap();
    on_top_scroll.forget();

    let t3 = top_el.clone();
    let w3 = wrapper_el.clone();
    let on_wrapper_scroll = Closure::<dyn Fn()>::new(move || {
        t3.set_scroll_left(w3.scroll_left());
    });
    wrapper
        .add_event_listener_with_callback("scroll", on_wrapper_scroll.as_ref().unchecked_ref())
        .unwrap();
    on_wrapper_scroll.forget();
}
