use gtk4::{
    gdk::{Key, ModifierType},
    gio::prelude::ListModelExt,
    glib::Propagation,
    prelude::{Cast, EventControllerExt, GtkWindowExt, ObjectExt, WidgetExt},
    ApplicationWindow, GridView,
};

use crate::launcher;
use crate::ui::UiController;

pub fn setup_keybinds(
    window: &ApplicationWindow,
    grid_view: Option<&GridView>,
    ui: Option<UiController>,
) {
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let window_weak = window.downgrade();
    let grid_view = grid_view.cloned();

    key_controller.connect_key_pressed(move |_, keyval, _, state| match keyval {
        Key::Escape => {
            if let Some(window) = window_weak.upgrade() {
                window.close();
            }
            Propagation::Stop
        }
        Key::Return => {
            if let Some(ref grid_view) = grid_view {
                launcher::launch_selected_app(grid_view);
            }
            if let Some(window) = window_weak.upgrade() {
                window.close();
            }
            Propagation::Stop
        }
        Key::Down => {
            if let Some(ref grid_view) = grid_view {
                move_selection(grid_view, 1);
            }
            Propagation::Stop
        }
        Key::Up => {
            if let Some(ref grid_view) = grid_view {
                move_selection(grid_view, -1);
            }
            Propagation::Stop
        }
        Key::Right => {
            if let Some(ref grid_view) = grid_view {
                move_selection(grid_view, 1);
            }
            Propagation::Stop
        }
        Key::Left => {
            if let Some(ref grid_view) = grid_view {
                move_selection(grid_view, -1);
            }
            Propagation::Stop
        }
        Key::p | Key::P if state.contains(ModifierType::CONTROL_MASK) => {
            if let Some(ref grid_view) = grid_view {
                crate::launcher::toggle_pin_selected(grid_view, ui.as_ref());
            }
            Propagation::Stop
        }
        _ => Propagation::Proceed,
    });

    window.add_controller(key_controller);
}

fn move_selection(grid_view: &GridView, direction: i32) {
    grid_view.set_can_target(false);

    let Some(model) = grid_view.model() else {
        return;
    };
    let Some(selection) = model.downcast_ref::<gtk4::SingleSelection>() else {
        return;
    };

    let current = selection.selected();
    let n_items = selection.n_items();

    if n_items == 0 {
        return;
    }

    let new_pos = if direction > 0 {
        if current == u32::MAX {
            0
        } else {
            (current + 1).min(n_items - 1)
        }
    } else {
        if current == u32::MAX || current == 0 {
            0
        } else {
            current - 1
        }
    };

    selection.set_selected(new_pos);
}

