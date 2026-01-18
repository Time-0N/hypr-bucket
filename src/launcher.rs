use gtk4::{prelude::Cast, GridView};
use std::process::Command;

use crate::{
    config::Config,
    desktop::DesktopEntry,
    ui::{AppEntryObject, UiRebuildController},
};

fn get_selected_entry(grid_view: &GridView) -> Option<DesktopEntry> {
    let model = grid_view.model()?;
    let selection = model.downcast_ref::<gtk4::SingleSelection>()?;

    if selection.selected() == u32::MAX {
        return None;
    }

    let item = selection.selected_item()?;
    item.downcast::<AppEntryObject>()
        .ok()
        .map(|obj| obj.entry())
}

pub fn launch_selected_app(grid_view: &GridView) {
    if let Some(entry) = get_selected_entry(grid_view) {
        launch_app(&entry);
    }
}

pub fn launch_app(entry: &DesktopEntry) {
    let exec = entry
        .exec
        .split_whitespace()
        .filter(|arg| !arg.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    println!("Launching: {} ({})", entry.name, exec);

    let result = if entry.terminal {
        let terminal = find_terminal();
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "{} -e sh -c '{}'",
                terminal,
                exec.replace("'", "\\'")
            ))
            .spawn()
    } else {
        Command::new("sh").arg("-c").arg(&exec).spawn()
    };

    match result {
        Ok(_) => println!("Launched: {}", entry.name),
        Err(e) => eprintln!("Failed to launch {}: {}", entry.name, e),
    }
}

fn find_terminal() -> &'static str {
    let terminals = [
        "kitty",
        "alacritty",
        "wezterm",
        "foot",
        "gnome-terminal",
        "konsole",
        "xterm",
    ];

    for terminal in &terminals {
        if Command::new("which")
            .arg(terminal)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(())
                } else {
                    None
                }
            })
            .is_some()
        {
            return terminal;
        }
    }

    "sh"
}

pub fn toggle_pin_selected(grid_view: &GridView, ui_rebuild: Option<&UiRebuildController>) {
    if let Some(entry) = get_selected_entry(grid_view) {
        let mut config = Config::load();
        let was_pinned = config.pinned.contains(&entry.id);
        config.toggle_pin(&entry.id);

        println!(
            "{}: {}",
            if was_pinned { "Unpinned" } else { "Pinned" },
            entry.name
        );

        if let Some(ui) = ui_rebuild {
            ui.rebuild();
        }
    }
}
