use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use gtk4::{
    prelude::{BoxExt, Cast, CastNone, EventControllerExt, GtkWindowExt, ObjectExt, WidgetExt},
    Application, ApplicationWindow, Box as GtkBox, EventControllerMotion, GestureClick, GridView,
    Orientation, PropagationPhase,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};

use crate::{keybinds, ui};

const APP_NAME: &str = "hyprbucket";

pub fn build_ui(app: &Application) {
    load_styles();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Hypr Bucket")
        .decorated(false)
        .build();

    setup_layer_shell(&window);

    let wrapper = GtkBox::new(Orientation::Horizontal, 0);
    wrapper.set_halign(gtk4::Align::Center);
    wrapper.set_valign(gtk4::Align::Center);
    wrapper.set_overflow(gtk4::Overflow::Hidden);
    wrapper.add_css_class("hyprbucket-wrapper");

    let (content, model) = ui::build_content();
    content.add_css_class("hyprbucket-panel");

    wrapper.append(&content);
    window.set_child(Some(&wrapper));

    let grid_view = find_grid_view(&content);

    setup_click_to_close(&window);
    setup_mouse_motion_tracking(&window, grid_view.as_ref());
    keybinds::setup_keybinds(&window, grid_view.as_ref());

    window.present();

    crate::desktop::refresh_desktop_entries_async(move |new_apps| {
        for app in new_apps {
            let obj = ui::AppEntryObject::new(&app);
            model.append(&obj);
        }
    });
}

fn setup_layer_shell(window: &ApplicationWindow) {
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_namespace(Some("hyprbucket"));
    window.set_exclusive_zone(-1);
}

fn find_grid_view(container: &GtkBox) -> Option<gtk4::GridView> {
    let mut child = container.first_child();

    while let Some(widget) = child {
        if let Some(scrolled) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
            if let Some(grid_view) = scrolled.child().and_downcast::<gtk4::GridView>() {
                return Some(grid_view);
            }
        }
        child = widget.next_sibling();
    }
    None
}

fn setup_click_to_close(window: &ApplicationWindow) {
    let window_weak = window.downgrade();
    let click = GestureClick::new();
    click.set_propagation_phase(PropagationPhase::Target);
    click.connect_pressed(move |_, _, _, _| {
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    window.add_controller(click);
}

fn load_styles() {
    let provider = gtk4::CssProvider::new();

    if let Ok(home) = std::env::var("HOME") {
        let user_css = format!("{}/.config/{}/default.css", home, APP_NAME);

        if Path::new(&user_css).exists() {
            provider.load_from_path(&user_css);
        } else {
            let default_css = "resources/default.css";
            if Path::new(default_css).exists() {
                provider.load_from_path(default_css);
            }
        }
    } else {
        let default_css = "resources/default.css";
        if Path::new(default_css).exists() {
            provider.load_from_path(default_css);
        }
    }

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn setup_mouse_motion_tracking(window: &ApplicationWindow, grid_view: Option<&GridView>) {
    let Some(grid_view) = grid_view else {
        return;
    };

    let grid_view = grid_view.clone();
    let motion = EventControllerMotion::new();
    let last_x = Rc::new(Cell::new(-1.0));
    let last_y = Rc::new(Cell::new(-1.0));

    let last_x_clone = last_x.clone();
    let last_y_clone = last_y.clone();
    let grid_view_clone = grid_view.clone();

    motion.connect_motion(move |_, x, y| {
        let prev_x = last_x_clone.get();
        let prev_y = last_y_clone.get();

        if prev_x < 0.0 || prev_y < 0.0 {
            last_x_clone.set(x);
            last_y_clone.set(y);
            return;
        }

        let dx = (x - prev_x).abs();
        let dy = (y - prev_y).abs();

        if dx > 1.0 || dy > 1.0 {
            if !grid_view_clone.can_target() {
                grid_view_clone.set_can_target(true);
            }
            last_x_clone.set(x);
            last_y_clone.set(y);
        }
    });

    window.add_controller(motion);
}
