// settings_ui/mod.rs — GTK4 niri Settings window.
//
// Sidebar navigation + stack of pages.
// Launched via `niri-shell --settings`.

mod extra_pages_ui;
mod helpers_ui;
mod input_pages_ui;
mod outputs_page_ui;
mod rules_pages_ui;
mod shell_pages_ui;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk4::prelude::*;
use gtk4::{
    glib, Box as GtkBox, Button, CssProvider, Label, Orientation, ScrolledWindow, Stack, Window,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};

use extra_pages_ui::{
    build_animations_page, build_debug_page, build_gestures_page, build_miscellaneous_page,
    build_recent_windows_page, build_workspaces_page,
};
use input_pages_ui::{build_behaviour_page, build_input_page, build_layout_page};
use outputs_page_ui::build_outputs_page;
use rules_pages_ui::{
    build_keybindings_page, build_layer_rules_page, build_switch_events_page,
    build_window_rules_page,
};
use shell_pages_ui::{build_shell_behaviour_page, build_software_page, build_theme_page};

// ── Public entry point ────────────────────────────────────────────────────────

/// Creates a standalone GTK Application and opens the settings window.
/// Called from `main()` when `--settings` is passed.
pub fn run() -> Result<(), crate::error::ShellError> {
    gtk4::init().map_err(|_| crate::error::ShellError::GtkInit)?;

    let app = gtk4::Application::builder()
        .application_id("org.niri.settings")
        // Each invocation may request a different initial page. Keeping the
        // editor non-unique makes `--page outputs` deterministic even when an
        // existing settings window is open.
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    let initial_page = requested_page();

    app.connect_activate(move |app| {
        let win = SettingsWindow::new(&initial_page);
        win.window.set_application(Some(app));
        win.show();
    });

    let argv0 = std::env::args().next().unwrap_or_default();
    app.run_with_args(&[argv0]);
    Ok(())
}

// ── SettingsWindow ────────────────────────────────────────────────────────────

pub struct SettingsWindow {
    pub window: Window,
}

impl SettingsWindow {
    pub fn new(initial_page: &str) -> Self {
        // Track load failures so the user is told their edits start from defaults
        // (rather than silently losing an existing-but-unreadable config).
        let mut load_warning: Option<String> = None;
        let cfg = Rc::new(RefCell::new(
            crate::settings_backend::load().unwrap_or_else(|error| {
                log::warn!("settings: could not load niri config ({error}); using defaults");
                load_warning = Some(format!(
                    "could not load config ({error}); editing from defaults"
                ));
                Default::default()
            }),
        ));
        let shell_cfg = Rc::new(RefCell::new(
            crate::settings_backend::shell::load().unwrap_or_else(|error| {
                log::warn!("settings: could not load shell config ({error}); using defaults");
                if load_warning.is_none() {
                    load_warning = Some(format!(
                        "could not load shell config ({error}); editing from defaults"
                    ));
                }
                Default::default()
            }),
        ));

        let window = Window::new();
        window.set_title(Some("niri settings"));
        window.set_default_size(840, 580);
        window.set_resizable(true);
        window.add_css_class("settings-window");

        let provider = CssProvider::new();
        provider.load_from_string(include_str!("settings.css"));
        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&window),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let root = GtkBox::new(Orientation::Horizontal, 0);
        root.add_css_class("settings-window");

        let sidebar = build_sidebar();

        let stack = Rc::new(Stack::new());
        stack.add_named(
            &build_keybindings_page(Rc::clone(&cfg)),
            Some("keybindings"),
        );
        stack.add_named(
            &build_window_rules_page(Rc::clone(&cfg)),
            Some("window-rules"),
        );
        stack.add_named(&build_outputs_page(Rc::clone(&cfg)), Some("outputs"));
        stack.add_named(&build_theme_page(Rc::clone(&shell_cfg)), Some("theme"));
        stack.add_named(
            &build_shell_behaviour_page(Rc::clone(&shell_cfg)),
            Some("behaviour"),
        );
        stack.add_named(&build_software_page(), Some("software"));
        stack.add_named(
            &build_behaviour_page(Rc::clone(&cfg)),
            Some("compositor-behaviour"),
        );
        stack.add_named(&build_input_page(Rc::clone(&cfg)), Some("input"));
        stack.add_named(&build_layout_page(Rc::clone(&cfg)), Some("layout"));
        stack.add_named(&build_animations_page(Rc::clone(&cfg)), Some("animations"));
        stack.add_named(&build_workspaces_page(Rc::clone(&cfg)), Some("workspaces"));
        stack.add_named(&build_gestures_page(Rc::clone(&cfg)), Some("gestures"));
        stack.add_named(
            &build_recent_windows_page(Rc::clone(&cfg)),
            Some("recent-windows"),
        );
        stack.add_named(
            &build_miscellaneous_page(Rc::clone(&cfg)),
            Some("miscellaneous"),
        );
        stack.add_named(&build_debug_page(Rc::clone(&cfg)), Some("debug"));
        stack.add_named(
            &build_layer_rules_page(Rc::clone(&cfg)),
            Some("layer-rules"),
        );
        stack.add_named(
            &build_switch_events_page(Rc::clone(&cfg)),
            Some("switch-events"),
        );
        let initial_page = if NAV_ITEMS.iter().any(|(name, _, _)| *name == initial_page) {
            initial_page
        } else {
            "keybindings"
        };
        stack.set_visible_child_name(initial_page);

        wire_sidebar(&sidebar.container, &stack, initial_page);

        root.append(&sidebar.container);

        let content = GtkBox::new(Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_vexpand(true);

        let toolbar = GtkBox::new(Orientation::Horizontal, 8);
        toolbar.add_css_class("settings-toolbar");
        let draft = Label::new(Some("no unsaved changes"));
        draft.add_css_class("draft-label");
        draft.set_hexpand(true);
        draft.set_xalign(0.0);
        if let Some(warning) = &load_warning {
            draft.set_text(warning);
            draft.add_css_class("error");
        }
        let revert = Button::with_label("revert");
        revert.add_css_class("toolbar-button");
        let apply = Button::with_label("apply changes");
        apply.add_css_class("save-btn");
        apply.set_margin_top(0);
        apply.set_margin_bottom(0);
        toolbar.append(&draft);
        toolbar.append(&revert);
        toolbar.append(&apply);
        content.append(&toolbar);

        let scroll = ScrolledWindow::new();
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&*stack));
        content.append(&scroll);
        root.append(&content);

        window.set_child(Some(&root));

        {
            let cfg = Rc::clone(&cfg);
            let shell_cfg = Rc::clone(&shell_cfg);
            let draft = draft.clone();
            apply.connect_clicked(move |button| {
                let conflicts = crate::settings_backend::binding_conflicts(&cfg.borrow().binds);
                if !conflicts.is_empty() {
                    let keys = conflicts
                        .iter()
                        .map(|(key, _)| key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    draft.set_text(&format!("resolve keybinding conflicts: {keys}"));
                    draft.add_css_class("error");
                    return;
                }
                draft.remove_css_class("error");
                button.set_sensitive(false);
                button.set_label("validating…");
                let niri_draft = cfg.borrow().clone();
                let shell_draft = shell_cfg.borrow().clone();
                let result_thread = Arc::new(std::sync::Mutex::new(None));
                let writer = Arc::clone(&result_thread);
                std::thread::spawn(move || {
                    let saved = crate::settings_backend::save(&niri_draft).and_then(|status| {
                        crate::settings_backend::shell::save(&shell_draft).map(|_| status)
                    });
                    if let Ok(mut slot) = writer.lock() {
                        *slot = Some(saved);
                    }
                });
                let button = button.clone();
                let draft = draft.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(25), move || {
                    let saved = result_thread.lock().ok().and_then(|mut slot| slot.take());
                    if let Some(saved) = saved {
                        button.set_sensitive(true);
                        match saved {
                            Ok(crate::settings_backend::SaveStatus::Validated) => {
                                button.set_label("saved ✓");
                                draft.set_text("configuration applied");
                            }
                            Ok(crate::settings_backend::SaveStatus::SavedWithoutValidation) => {
                                button.set_label("saved ✓");
                                draft.set_text("saved — niri unavailable, not validated");
                            }
                            Err(error) => {
                                button.set_label("apply changes");
                                draft.set_text(&format!("save failed: {error}"));
                                draft.add_css_class("error");
                            }
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        {
            let window = window.clone();
            let stack = Rc::clone(&stack);
            revert.connect_clicked(move |_| {
                let page = stack
                    .visible_child_name()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| "keybindings".into());
                let Some(app) = window.application() else {
                    return;
                };
                window.close();
                let replacement = SettingsWindow::new(&page);
                replacement.window.set_application(Some(&app));
                replacement.show();
            });
        }

        wire_dirty_tracking(root.upcast_ref(), &draft);

        Self { window }
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.present();
    }
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

struct Sidebar {
    container: GtkBox,
}

const NAV_ITEMS: &[(&str, &str, &str)] = &[
    ("keybindings", "keybindings", "niri"),
    ("window-rules", "window rules", "niri"),
    ("outputs", "outputs", "niri"),
    ("theme", "theme", "niri"),
    ("behaviour", "behaviour", "niri"),
    ("software", "software", "tools"),
    ("compositor-behaviour", "compositor behaviour", "advanced"),
    ("input", "input", "advanced"),
    ("layout", "layout", "advanced"),
    ("animations", "animations", "advanced"),
    ("workspaces", "workspaces", "advanced"),
    ("gestures", "gestures", "advanced"),
    ("recent-windows", "recent windows", "advanced"),
    ("miscellaneous", "miscellaneous", "advanced"),
    ("debug", "debug", "advanced"),
    ("layer-rules", "layer rules", "advanced"),
    ("switch-events", "switch events", "advanced"),
];

fn build_sidebar() -> Sidebar {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.add_css_class("settings-sidebar");
    container.set_hexpand(false);
    container.set_vexpand(true);
    container.set_width_request(170);

    let header = GtkBox::new(Orientation::Vertical, 2);
    header.add_css_class("sidebar-header");
    let title = Label::new(Some("settings"));
    title.add_css_class("sidebar-title");
    title.set_xalign(0.0);
    let sub = Label::new(Some("niri-shell"));
    sub.add_css_class("row-sublabel");
    sub.set_xalign(0.0);
    header.append(&title);
    header.append(&sub);
    container.append(&header);

    let mut current_section = "";
    for (page_name, label, section) in NAV_ITEMS {
        if *section != current_section {
            let section_label = Label::new(Some(section));
            section_label.add_css_class("sidebar-section-label");
            section_label.set_xalign(0.0);
            container.append(&section_label);
            current_section = section;
        }
        let btn = Button::with_label(label);
        btn.add_css_class("sidebar-item");
        btn.set_hexpand(true);
        btn.set_widget_name(page_name);
        container.append(&btn);
    }

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    container.append(&spacer);

    let ver = Label::new(Some(concat!("niri-settings v", env!("CARGO_PKG_VERSION"))));
    ver.add_css_class("ver-label");
    ver.set_xalign(0.0);
    container.append(&ver);

    Sidebar { container }
}

fn wire_sidebar(sidebar: &GtkBox, stack: &Rc<Stack>, initial_page: &str) {
    let mut buttons: Vec<Button> = Vec::new();
    let mut child = sidebar.first_child();
    while let Some(w) = child {
        if w.has_css_class("sidebar-item") {
            if let Ok(btn) = w.clone().downcast::<Button>() {
                buttons.push(btn);
            }
        }
        child = w.next_sibling();
    }

    let buttons = Rc::new(buttons);

    for btn in buttons.iter() {
        let page_name = btn.widget_name().to_string();
        let stack_c = Rc::clone(stack);
        let buttons_c = Rc::clone(&buttons);
        let btn_weak = btn.downgrade();
        btn.connect_clicked(move |_| {
            stack_c.set_visible_child_name(&page_name);
            for b in buttons_c.iter() {
                b.remove_css_class("active");
            }
            if let Some(b) = btn_weak.upgrade() {
                b.add_css_class("active");
            }
        });
    }

    if let Some(initial) = buttons
        .iter()
        .find(|button| button.widget_name() == initial_page)
    {
        initial.add_css_class("active");
    }
}

fn requested_page() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--page" {
            return args.next().unwrap_or_else(|| "keybindings".into());
        }
        if let Some(page) = arg.strip_prefix("--page=") {
            return page.to_owned();
        }
    }
    "keybindings".into()
}

fn wire_dirty_tracking(root: &gtk4::Widget, indicator: &Label) {
    if let Some(entry) = root.downcast_ref::<gtk4::Entry>() {
        let indicator = indicator.clone();
        entry.connect_changed(move |_| {
            indicator.remove_css_class("error");
            indicator.set_text("unsaved changes");
        });
    } else if let Some(toggle) = root.downcast_ref::<gtk4::Switch>() {
        let indicator = indicator.clone();
        toggle.connect_active_notify(move |_| {
            indicator.remove_css_class("error");
            indicator.set_text("unsaved changes");
        });
    } else if let Some(scale) = root.downcast_ref::<gtk4::Scale>() {
        let indicator = indicator.clone();
        scale.connect_value_changed(move |_| {
            indicator.remove_css_class("error");
            indicator.set_text("unsaved changes");
        });
    } else if let Some(dropdown) = root.downcast_ref::<gtk4::DropDown>() {
        let indicator = indicator.clone();
        dropdown.connect_selected_notify(move |_| {
            indicator.remove_css_class("error");
            indicator.set_text("unsaved changes");
        });
    } else if let Some(button) = root.downcast_ref::<gtk4::Button>() {
        let label = button.label().unwrap_or_default();
        if label.starts_with('+')
            || label.starts_with("delete")
            || label == "import from niri config"
        {
            let indicator = indicator.clone();
            button.connect_clicked(move |_| {
                indicator.remove_css_class("error");
                indicator.set_text("unsaved changes");
            });
        }
    }

    let mut child = root.first_child();
    while let Some(widget) = child {
        wire_dirty_tracking(&widget, indicator);
        child = widget.next_sibling();
    }
}
