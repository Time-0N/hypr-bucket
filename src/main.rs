use gtk4::prelude::{ApplicationExt, ApplicationExtManual};
use gtk4::Application;
use std::sync::{Arc, Mutex};

mod app;
mod config;
mod desktop;
mod keybinds;
mod launcher;
mod search;
mod single_instance;
mod ui;

const APP_ID: &str = "com.github.timeon.hyprbucket";

fn main() -> gtk4::glib::ExitCode {
    if single_instance::notify_existing_instance() {
        return gtk4::glib::ExitCode::SUCCESS;
    }

    let app = Application::builder().application_id(APP_ID).build();

    let app_ref = Arc::new(Mutex::new(Some(app.clone())));
    let app_ref_clone = app_ref.clone();

    app.connect_activate(move |app| {
        single_instance::setup_socket_listener(app_ref_clone.clone());
        app::build_ui(app);
    });

    app.run()
}
