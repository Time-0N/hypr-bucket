use gtk4::{
    glib::object::Cast,
    prelude::{BoxExt, WidgetExt},
    Box, Image, Label, Orientation,
};

use super::AppEntryObject;

pub fn create_app_row() -> Box {
    let row = Box::new(Orientation::Horizontal, 12);
    row.add_css_class("app-row");

    // Icon placeholder
    let icon = Image::new();
    icon.set_pixel_size(32);
    icon.set_widget_name("app-icon");
    row.append(&icon);

    // Text container
    let text_box = Box::new(Orientation::Vertical, 2);
    text_box.set_hexpand(true);

    let name_label = Label::new(None);
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_widget_name("app-name");
    name_label.add_css_class("app-name");
    text_box.append(&name_label);

    row.append(&text_box);

    row
}

pub fn populate_app_row(row: &Box, entry_obj: &AppEntryObject) {
    let entry = entry_obj.entry();

    if let Some(icon_widget) = row.first_child() {
        if let Some(icon) = icon_widget.downcast_ref::<Image>() {
            if let Some(icon_name) = &entry.icon {
                if icon_name.starts_with('/') {
                    icon.set_from_file(Some(icon_name));
                } else {
                    icon.set_icon_name(Some(icon_name));
                }
            } else {
                icon.set_icon_name(Some("application-x-executable"));
            }
        }
    }

    if let Some(icon_widget) = row.first_child() {
        if let Some(text_box) = icon_widget.next_sibling() {
            if let Some(text_box) = text_box.downcast_ref::<Box>() {
                if let Some(name_label) = text_box.first_child() {
                    if let Some(label) = name_label.downcast_ref::<Label>() {
                        label.set_text(&entry.name);
                    }
                }
            }
        }
    }
}
