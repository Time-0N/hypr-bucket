use std::rc::Rc;

use gtk4::{
    gio::ListStore, glib::Object, prelude::*, subclass::prelude::*, Box, Entry, GridView, ListItem,
    ListScrollFlags, Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection,
};

pub mod components;

pub use components::{create_app_row, populate_app_row};

pub fn build_content() -> (Box, ListStore) {
    let container = Box::new(Orientation::Vertical, 0);
    container.set_hexpand(false);
    container.set_vexpand(false);
    container.set_overflow(gtk4::Overflow::Hidden);

    let (search_box, search_entry) = create_search_box();
    container.append(&search_box);

    let all_entries = crate::desktop::load_desktop_entries();
    let (list_view, model) = create_virtual_list(&all_entries);

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled.set_min_content_height(400);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&list_view));

    container.append(&scrolled);

    let status_bar = create_status_bar();
    container.append(&status_bar);

    setup_search(&search_entry, &model, &list_view, all_entries);
    (container, model)
}

fn create_search_box() -> (Box, Entry) {
    let container = Box::new(Orientation::Vertical, 0);
    container.set_margin_top(16);
    container.set_margin_bottom(12);
    container.set_margin_start(16);
    container.set_margin_end(16);

    let search = Entry::new();
    search.set_placeholder_text(Some("Search applications..."));
    search.add_css_class("search-input");
    search.set_hexpand(true);

    container.append(&search);
    (container, search)
}

fn create_status_bar() -> Box {
    let status_bar = Box::new(Orientation::Horizontal, 8);
    status_bar.set_margin_top(8);
    status_bar.set_margin_bottom(12);
    status_bar.set_margin_start(16);
    status_bar.set_margin_end(16);
    status_bar.add_css_class("status-bar");

    let status_label = gtk4::Label::new(Some(
        "↑↓ Navigate  •  Enter to launch  •  Esc to close   •  Ctrl+p Pin",
    ));
    status_label.add_css_class("status-label");

    status_bar.append(&status_label);
    status_bar
}

fn create_virtual_list(entries: &[crate::desktop::DesktopEntry]) -> (GridView, ListStore) {
    let model = ListStore::new::<AppEntryObject>();

    let config = crate::config::Config::load();
    let (mut pinned, unpinned): (Vec<_>, Vec<_>) = entries
        .iter()
        .cloned()
        .partition(|app| config.pinned.contains(&app.id));

    pinned.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    pinned
        .iter()
        .chain(unpinned.iter())
        .map(|entry| AppEntryObject::new(entry))
        .for_each(|obj| model.append(&obj));

    let selection = SingleSelection::new(Some(model.clone()));
    selection.set_autoselect(true);
    selection.set_selected(0);

    let factory = SignalListItemFactory::new();

    factory.connect_setup(move |_, list_item| {
        let row = create_app_row();
        list_item
            .downcast_ref::<ListItem>()
            .expect("ListItem expected")
            .set_child(Some(&row));
    });

    factory.connect_bind(move |_, list_item| {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("ListItem expected");

        let entry_obj = list_item
            .item()
            .and_then(|item| item.downcast::<AppEntryObject>().ok())
            .expect("AppEntryObject expected");

        let row = list_item
            .child()
            .and_downcast::<Box>()
            .expect("Box expected");

        populate_app_row(&row, &entry_obj);
    });

    let grid_view = GridView::new(Some(selection), Some(factory));

    grid_view.set_max_columns(1);
    grid_view.set_min_columns(1);

    grid_view.set_single_click_activate(true);

    grid_view.set_can_target(false);

    setup_selection_scroll(&grid_view);

    setup_activation(&grid_view);

    (grid_view, model)
}

fn setup_selection_scroll(grid_view: &GridView) {
    let grid_view_clone = grid_view.clone();

    if let Some(model) = grid_view.model() {
        if let Some(selection) = model.downcast_ref::<SingleSelection>() {
            selection.connect_selection_changed(move |_, _, _| {
                let grid_view = &grid_view_clone;
                grid_view.scroll_to(
                    grid_view
                        .model()
                        .and_then(|m| m.downcast_ref::<SingleSelection>().map(|s| s.selected()))
                        .unwrap_or(u32::MAX),
                    ListScrollFlags::NONE,
                    None,
                );
            });
        }
    }
}

fn setup_search(
    search_entry: &Entry,
    model: &ListStore,
    grid_view: &GridView,
    all_entries: Vec<crate::desktop::DesktopEntry>,
) {
    let all_entries = Rc::new(all_entries);
    let model = model.clone();
    let grid_view = grid_view.clone();

    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string();
        let config = crate::config::Config::load();

        model.remove_all();

        let filtered = if query.is_empty() {
            all_entries.as_ref().clone()
        } else {
            crate::search::search_apps(&query, &all_entries)
        };

        let (mut pinned, unpinned): (Vec<_>, Vec<_>) = filtered
            .into_iter()
            .partition(|app| config.pinned.contains(&app.id));

        pinned.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        pinned
            .iter()
            .chain(unpinned.iter())
            .map(AppEntryObject::new)
            .for_each(|obj| model.append(&obj));

        if let Some(selection_model) = grid_view.model() {
            if let Some(selection) = selection_model.downcast_ref::<SingleSelection>() {
                selection.set_selected(0);
            }
        }
    });

    search_entry.grab_focus();
}

fn setup_activation(grid_view: &GridView) {
    let grid_view_clone = grid_view.clone();

    grid_view.connect_activate(move |_, _| {
        launch_selected(&grid_view_clone);
    });
}

fn launch_selected(grid_view: &GridView) {
    if let Some(model) = grid_view.model() {
        if let Some(selection) = model.downcast_ref::<SingleSelection>() {
            if selection.selected() != u32::MAX {
                if let Some(item) = selection.selected_item() {
                    if let Ok(entry_obj) = item.downcast::<AppEntryObject>() {
                        crate::launcher::launch_app(&entry_obj.entry());
                        if let Some(window) = grid_view
                            .root()
                            .and_then(|root| root.downcast::<gtk4::Window>().ok())
                        {
                            window.close();
                        }
                    }
                }
            }
        }
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk4::glib::{self, subclass::prelude::*};

    #[derive(Default)]
    pub struct AppEntryObject {
        pub entry: RefCell<Option<crate::desktop::DesktopEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppEntryObject {
        const NAME: &'static str = "AppEntryObject";
        type Type = super::AppEntryObject;
    }

    impl ObjectImpl for AppEntryObject {}
}

gtk4::glib::wrapper! {
    pub struct AppEntryObject(ObjectSubclass<imp::AppEntryObject>);
}

impl AppEntryObject {
    pub fn new(entry: &crate::desktop::DesktopEntry) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().entry.replace(Some(entry.clone()));
        obj
    }

    pub fn entry(&self) -> crate::desktop::DesktopEntry {
        self.imp()
            .entry
            .borrow()
            .as_ref()
            .expect("Entry should be set")
            .clone()
    }
}
