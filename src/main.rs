use gtk4::{
    Application, ApplicationWindow,
    gio::prelude::{ApplicationExt, ApplicationExtManual},
    glib,
    prelude::{GtkWindowExt, WidgetExt},
    subclass::window,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

mod config;
mod desktop;
mod search;
mod ui;

const APP_ID: &str = "com.github.timeon.hyprbucket";
const APP_NAME: &str = "hyprbucket";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    load_styles();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Hypr Bucket")
        .default_width(700)
        .default_height(500)
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);

    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);

    window.auto_exclusive_zone_enable();

    window.set_namespace("hyprbucket");

    let content = ui::build_content();
    window.set_child(Some(&content));

    setup_keybinds(&window);

    window.present();
}

fn setup_keybinds(window: &ApplicationWindow) {
    let key_controller = gtk4::EventControllerKey::new();

    let window_clone = window.clone();

    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        use gtk4::gdk::Key;

        match keyval {
            Key::Escape => {
                window_clone.close();
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    window.add_controller(key_controller);
}

fn load_styles() {
    use std::path::Path;

    let provider = gtk4::CssProvider::new();

    let default_css = "resources/default.css";

    if Path::new(default_css).exists() {
        provider.load_from_path(default_css);
    } else {
        println!("Warning: Could not find default CSS at {}", default_css);
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_css = format!("{}/.config/{}/style.css", home, APP_NAME);

        if Path::new(&user_css).exists() {
            provider.load_from_path(&user_css);
        }
    }

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
