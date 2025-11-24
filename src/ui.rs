use gtk4::{
    Box, Entry, Image, Label, ListBox, Orientation, ScrolledWindow,
    prelude::{BoxExt, EntryExt, WidgetExt},
};

pub fn build_content() -> Box {
    let container = Box::new(Orientation::Vertical, 0);

    let search_container = Box::new(Orientation::Vertical, 0);
    search_container.set_margin_top(16);
    search_container.set_margin_bottom(12);
    search_container.set_margin_start(16);
    search_container.set_margin_end(16);

    let search = Entry::new();
    search.set_placeholder_text(Some("Search applications..."));
    search.add_css_class("search-input");
    search.set_hexpand(true);

    search_container.append(&search);
    container.append(&search_container);

    let separator = gtk4::Separator::new(Orientation::Horizontal);
    separator.add_css_class("separator");
    container.append(&separator);

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let list = ListBox::new();
    list.add_css_class("app-list");
    list.set_selection_mode(gtk4::SelectionMode::Single);

    populate_list(&list);

    scrolled.set_child(Some(&list));
    container.append(&scrolled);

    let status_bar = Box::new(Orientation::Horizontal, 8);
    status_bar.set_margin_top(8);
    status_bar.set_margin_bottom(12);
    status_bar.set_margin_start(16);
    status_bar.set_margin_end(16);
    status_bar.add_css_class("status-bar");

    let status_label = Label::new(Some("↑↓ Navigate  •  Enter to launch  •  Esc to close"));
    status_label.add_css_class("status-label");
    status_bar.append(&status_label);

    container.append(&status_bar);

    container
}

fn populate_list(list: &ListBox) {
    use crate::desktop::load_desktop_entries;

    let entries = load_desktop_entries();

    if entries.is_empty() {
        list.append(&create_empty_row());
        return;
    }

    entries
        .iter()
        .take(50)
        .map(|entry| create_app_row(entry))
        .for_each(|row| list.append(&row));
}

fn create_app_row(entry: &crate::desktop::DesktopEntry) -> Box {
    let row = Box::new(Orientation::Horizontal, 12);
    row.set_margin_top(8);
    row.set_margin_bottom(8);
    row.set_margin_start(12);
    row.set_margin_end(12);
    row.add_css_class("app-row");

    let icon = if let Some(icon_name) = &entry.icon {
        Image::from_icon_name(icon_name)
    } else {
        Image::from_icon_name("application-x-executable")
    };
    icon.set_pixel_size(32);
    icon.add_css_class("app-icon");

    let text_box = Box::new(Orientation::Vertical, 2);
    text_box.set_hexpand(true);

    let name_label = Label::new(Some(&entry.name));
    name_label.set_xalign(0.0);
    name_label.add_css_class("app-name");

    let exec_display = if entry.exec.len() > 50 {
        format!("{}...", &entry.exec[..47])
    } else {
        entry.exec.clone()
    };
    let exec_label = Label::new(Some(&exec_display));
    exec_label.set_xalign(0.0);
    exec_label.add_css_class("app-exec");

    text_box.append(&name_label);
    text_box.append(&exec_label);

    row.append(&icon);
    row.append(&text_box);

    row
}

fn create_empty_row() -> Box {
    let row = Box::new(Orientation::Vertical, 8);
    row.set_margin_top(32);
    row.set_margin_bottom(32);
    row.set_halign(gtk4::Align::Center);

    let icon = Image::from_icon_name("action-unavailable-symbolic");
    icon.set_pixel_size(48);
    icon.set_opacity(0.5);

    let label = Label::new(Some("No applications found"));
    label.add_css_class("empty-label");

    row.append(&icon);
    row.append(&label);

    row
}
