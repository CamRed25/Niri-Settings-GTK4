// settings_ui/helpers_ui.rs — Shared GTK4 widget builders for settings pages.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    glib, Align, Box as GtkBox, Button, Entry, Label, Orientation, Scale, Separator, Switch,
};

use crate::settings_backend::SettingsConfig;

// ── Row-tuple type aliases (avoids clippy::type_complexity) ───────────────────

pub type AnimBoolRow = (
    &'static str,
    &'static str,
    fn(&crate::settings_backend::AnimSettings) -> bool,
    fn(&mut crate::settings_backend::AnimSettings, bool),
);

pub type CfgU32Row = (
    &'static str,
    &'static str,
    u32,
    fn(&SettingsConfig) -> u32,
    fn(&mut SettingsConfig, u32),
);

pub type CfgBoolRow = (
    &'static str,
    &'static str,
    fn(&SettingsConfig) -> bool,
    fn(&mut SettingsConfig, bool),
);

pub type U32Row = (
    &'static str,
    &'static str,
    u32,
    fn(&SettingsConfig) -> u32,
    fn(&mut SettingsConfig, u32),
);

// ── Layout helpers ────────────────────────────────────────────────────────────

/// A scrollable content page with consistent padding.
pub fn page_box() -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 0);
    b.add_css_class("settings-content");
    b.set_margin_top(20);
    b.set_margin_start(20);
    b.set_margin_end(20);
    b.set_margin_bottom(20);
    b
}

/// Page title + subtitle header widget.
pub fn page_header(title: &str, sub: &str) -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 2);
    b.set_margin_bottom(6);
    let t = Label::new(Some(title));
    t.add_css_class("page-title");
    t.set_xalign(0.0);
    b.append(&t);
    if !sub.is_empty() {
        let s = Label::new(Some(sub));
        s.add_css_class("page-sub");
        s.set_xalign(0.0);
        b.append(&s);
    }
    b
}

/// A visually distinct card container with a title row.
pub fn card(title: &str) -> GtkBox {
    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.add_css_class("settings-card");

    let hdr = GtkBox::new(Orientation::Horizontal, 0);
    let lbl = Label::new(Some(&title.to_uppercase()));
    lbl.add_css_class("card-title");
    lbl.set_xalign(0.0);
    hdr.append(&lbl);
    outer.append(&hdr);

    let sep = Separator::new(Orientation::Horizontal);
    sep.add_css_class("row-sep");
    outer.append(&sep);

    outer
}

/// A horizontal row box used inside a card.
pub fn row_box() -> GtkBox {
    let r = GtkBox::new(Orientation::Horizontal, 12);
    r.add_css_class("settings-row");
    r
}

/// A vertical label + sublabel column.
pub fn label_col(label: &str, sub: &str) -> GtkBox {
    let col = GtkBox::new(Orientation::Vertical, 2);
    let lbl = Label::new(Some(label));
    lbl.add_css_class("row-label");
    lbl.set_xalign(0.0);
    col.append(&lbl);
    if !sub.is_empty() {
        let s = Label::new(Some(sub));
        s.add_css_class("row-sublabel");
        s.set_xalign(0.0);
        col.append(&s);
    }
    col
}

// ── Toggle / color / slider rows ──────────────────────────────────────────────

/// Appends a toggle (label + Switch) row to a card.
pub fn append_toggle<F: Fn(bool) + 'static>(
    card: &GtkBox,
    label: &str,
    sub: &str,
    initial: bool,
    on_change: F,
) {
    let row = row_box();
    let lc = label_col(label, sub);
    lc.set_hexpand(true);
    row.append(&lc);

    let sw = Switch::new();
    sw.set_active(initial);
    sw.set_valign(Align::Center);
    sw.connect_state_set(move |_, state| {
        on_change(state);
        glib::Propagation::Proceed
    });
    row.append(&sw);
    card.append(&row);
}

/// Appends a color hex-entry row to a card.
pub fn append_color<F: Fn(String) + 'static>(
    card: &GtkBox,
    label: &str,
    initial: &str,
    on_change: F,
) {
    let row = row_box();
    let lbl = Label::new(Some(label));
    lbl.add_css_class("row-label");
    lbl.set_xalign(0.0);
    lbl.set_hexpand(true);
    row.append(&lbl);

    let entry = Entry::new();
    entry.add_css_class("color-entry");
    entry.set_text(initial);
    entry.set_width_chars(10);
    entry.set_max_width_chars(10);
    entry.set_placeholder_text(Some("#rrggbb"));
    entry.connect_changed(move |e| {
        on_change(e.text().to_string());
    });
    row.append(&entry);
    card.append(&row);
}

/// Appends a sequence of u32 slider rows to a card.
pub fn append_u32_sliders(card: &GtkBox, rows: &[U32Row], cfg: Rc<RefCell<SettingsConfig>>) {
    let mut first = true;
    for (label, desc, max, getter, setter) in rows {
        if !first {
            card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;

        let row = row_box();
        let lbl_col = label_col(label, desc);
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = getter(&cfg.borrow());
        let val_lbl = Label::new(Some(&if init == 0 {
            "default".to_string()
        } else {
            init.to_string()
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(7);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, *max as f64, 1.0);
        scale.set_value(init as f64);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        let setter_copy = *setter;
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            setter_copy(&mut cfg_s.borrow_mut(), v);
            val_lbl_c.set_text(&if v == 0 {
                "default".to_string()
            } else {
                v.to_string()
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        card.append(&row);
    }
}

// ── Validated entry binders ───────────────────────────────────────────────────
//
// These wire an `Entry::connect_changed` so that garbage input is *visibly*
// rejected (the `invalid` CSS class draws a red border, styled in settings.css)
// without silently writing a wrong value. An empty field is always valid and
// maps to the field's "no override" zero, matching the rest of the UI.

/// Bind a `u32` entry: writes through on a valid parse or empty (→ 0), and marks
/// the entry `invalid` otherwise without changing the stored value.
pub fn bind_u32_entry<F: Fn(u32) + 'static>(entry: &Entry, on_valid: F) {
    entry.connect_changed(move |e| {
        let text = e.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            e.remove_css_class("invalid");
            on_valid(0);
        } else if let Ok(v) = trimmed.parse::<u32>() {
            e.remove_css_class("invalid");
            on_valid(v);
        } else {
            e.add_css_class("invalid");
        }
    });
}

/// Bind an `i32` entry (e.g. output position): writes through on a valid parse
/// or empty (→ 0), and marks the entry `invalid` otherwise. Accepts negatives.
pub fn bind_i32_entry<F: Fn(i32) + 'static>(entry: &Entry, on_valid: F) {
    entry.connect_changed(move |e| {
        let text = e.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            e.remove_css_class("invalid");
            on_valid(0);
        } else if let Ok(v) = trimmed.parse::<i32>() {
            e.remove_css_class("invalid");
            on_valid(v);
        } else {
            e.add_css_class("invalid");
        }
    });
}

/// Bind an `f64` entry: writes through on a valid parse or empty (→ 0.0), and
/// marks the entry `invalid` otherwise without changing the stored value.
pub fn bind_f64_entry<F: Fn(f64) + 'static>(entry: &Entry, on_valid: F) {
    entry.connect_changed(move |e| {
        let text = e.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            e.remove_css_class("invalid");
            on_valid(0.0);
        } else if let Ok(v) = trimmed.parse::<f64>() {
            e.remove_css_class("invalid");
            on_valid(v);
        } else {
            e.add_css_class("invalid");
        }
    });
}

/// Bind a regex match entry. niri performs the authoritative regex validation at
/// apply time, so the value is always stored; this only flags *obviously*
/// malformed patterns (unbalanced brackets/parens) so the user gets early
/// feedback rather than a surprise at save.
pub fn bind_regex_entry<F: Fn(String) + 'static>(entry: &Entry, on_change: F) {
    entry.connect_changed(move |e| {
        let value = e.text().to_string();
        if looks_like_balanced_regex(&value) {
            e.remove_css_class("invalid");
        } else {
            e.add_css_class("invalid");
        }
        on_change(value);
    });
}

/// Cheap structural check: brackets and parentheses are balanced (ignoring
/// escaped ones). Not a full regex parser — just catches the common typos.
fn looks_like_balanced_regex(pattern: &str) -> bool {
    let mut round: i32 = 0;
    let mut square: i32 = 0;
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next(); // skip the escaped character
            }
            '(' => round += 1,
            ')' => round -= 1,
            '[' => square += 1,
            ']' => square -= 1,
            _ => {}
        }
        if round < 0 || square < 0 {
            return false;
        }
    }
    round == 0 && square == 0
}

// ── Save button ───────────────────────────────────────────────────────────────

/// Legacy placeholder retained while advanced pages are kept modular. Saving
/// is centralized in the window toolbar so validation never blocks GTK.
pub fn save_button(_cfg: Rc<RefCell<SettingsConfig>>) -> Button {
    let btn = Button::new();
    btn.set_visible(false);
    btn
}

// ── Closure helpers ───────────────────────────────────────────────────────────

/// Wraps a config mutation into a `Fn(bool)` that borrows the shared config.
pub fn cfg_toggle<F>(cfg: Rc<RefCell<SettingsConfig>>, f: F) -> impl Fn(bool) + 'static
where
    F: Fn(&mut SettingsConfig, bool) + 'static,
{
    move |v| f(&mut cfg.borrow_mut(), v)
}

/// Same as `cfg_toggle` but for `String` values (text entries).
pub fn cfg_text<F>(cfg: Rc<RefCell<SettingsConfig>>, f: F) -> impl Fn(String) + 'static
where
    F: Fn(&mut SettingsConfig, String) + 'static,
{
    move |v| f(&mut cfg.borrow_mut(), v)
}

// ── Format helpers ────────────────────────────────────────────────────────────

pub fn idle_ms_label(ms: u32) -> String {
    if ms == 0 {
        "never".to_string()
    } else if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.1} s", ms as f64 / 1000.0)
    }
}

pub fn zoom_label(v: f64) -> String {
    if v <= 0.0 {
        "default".to_string()
    } else {
        format!("{:.0}%", v * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::looks_like_balanced_regex;

    #[test]
    fn balanced_regexes_are_accepted() {
        assert!(looks_like_balanced_regex("^firefox$"));
        assert!(looks_like_balanced_regex("(foo|bar)"));
        assert!(looks_like_balanced_regex("[a-z]+"));
        assert!(looks_like_balanced_regex("")); // empty = match any
        assert!(looks_like_balanced_regex(r"\(literal\)")); // escaped parens
    }

    #[test]
    fn unbalanced_regexes_are_rejected() {
        assert!(!looks_like_balanced_regex("^[invalid("));
        assert!(!looks_like_balanced_regex("foo)"));
        assert!(!looks_like_balanced_regex("[a-z"));
    }
}
