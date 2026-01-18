use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use gtk4::{
    gio::ListStore,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
    Box, Entry, GridView, ListItem, ListScrollFlags, Orientation, ScrolledWindow,
    SignalListItemFactory, SingleSelection,
};

pub mod components;
pub use components::{create_app_row, populate_app_row};

use crate::desktop::{DesktopEntry, LoaderMsg};

#[derive(Default)]
struct RebuildTimer {
    id: Option<glib::SourceId>,
}

type EntryStore = Rc<RefCell<HashMap<String, DesktopEntry>>>;

#[derive(Clone)]
pub struct UiRebuildController {
    rebuild: Rc<dyn Fn()>,
}

impl UiRebuildController {
    pub fn new(rebuild: impl Fn() + 'static) -> Self {
        Self {
            rebuild: Rc::new(rebuild),
        }
    }

    pub fn rebuild(&self) {
        (self.rebuild)();
    }
}

pub fn build_content() -> (Box, ListStore, UiRebuildController) {
    let container = Box::new(Orientation::Vertical, 0);
    container.set_hexpand(false);
    container.set_vexpand(false);
    container.set_overflow(gtk4::Overflow::Hidden);

    let (search_box, search_entry) = create_search_box();
    container.append(&search_box);

    let store: EntryStore = Rc::new(RefCell::new(HashMap::new()));

    let (list_view, model) = create_virtual_list();

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled.set_min_content_height(400);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&list_view));
    container.append(&scrolled);

    let status_bar = create_status_bar();
    container.append(&status_bar);

    setup_search(&search_entry, &model, &list_view, store.clone());
    start_loader(&search_entry, &model, &list_view, store.clone());

    let rebuild_controller = UiRebuildController::new({
        let model = model.clone();
        let grid_view = list_view.clone();
        let store = store.clone();
        let search_entry = search_entry.clone();
        move || {
            let query = search_entry.text().to_string();
            rebuild_model(&model, &grid_view, &store.borrow(), &query);
        }
    });

    (container, model, rebuild_controller)
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

    let status_label = gtk4::Label::new(Some("↑↓ Navigate  •  Enter to launch  •  Esc to close"));
    status_label.add_css_class("status-label");

    status_bar.append(&status_label);
    status_bar
}

fn create_virtual_list() -> (GridView, ListStore) {
    let model = ListStore::new::<AppEntryObject>();

    let selection = SingleSelection::new(Some(model.clone()));
    selection.set_autoselect(true);

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
                let selected = grid_view_clone
                    .model()
                    .and_then(|m| m.downcast_ref::<SingleSelection>().map(|s| s.selected()))
                    .unwrap_or(u32::MAX);
                if selected != u32::MAX {
                    grid_view_clone.scroll_to(selected, ListScrollFlags::NONE, None);
                }
            });
        }
    }
}

fn setup_search(search_entry: &Entry, model: &ListStore, grid_view: &GridView, store: EntryStore) {
    let model = model.clone();
    let grid_view = grid_view.clone();
    let last_query = Rc::new(RefCell::new(String::new()));

    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string();

        let was_empty = last_query.borrow().is_empty();
        let is_empty = query.is_empty();
        *last_query.borrow_mut() = query.clone();

        rebuild_model(&model, &grid_view, &store.borrow(), &query);

        if is_empty && !was_empty && model.n_items() > 0 {
            if let Some(sel_model) = grid_view.model() {
                if let Some(selection) = sel_model.downcast_ref::<SingleSelection>() {
                    selection.set_selected(0);
                }
            }
            grid_view.scroll_to(0, ListScrollFlags::NONE, None);
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

fn start_loader(search_entry: &Entry, model: &ListStore, grid_view: &GridView, store: EntryStore) {
    let (tx, rx) = async_channel::unbounded::<LoaderMsg>();
    crate::desktop::spawn_load_entries(tx);

    let search_entry = search_entry.clone();
    let model = model.clone();
    let grid_view = grid_view.clone();

    let timer = Rc::new(RefCell::new(RebuildTimer::default()));

    let schedule_rebuild = {
        let timer = timer.clone();
        let search_entry = search_entry.clone();
        let model = model.clone();
        let grid_view = grid_view.clone();
        let store = store.clone();

        move || {
            if timer.borrow().id.is_some() {
                return;
            }

            let timer2 = timer.clone();
            let search_entry2 = search_entry.clone();
            let model2 = model.clone();
            let grid_view2 = grid_view.clone();
            let store2 = store.clone();

            let id = glib::timeout_add_local(Duration::from_millis(50), move || {
                let query = search_entry2.text().to_string();
                rebuild_model(&model2, &grid_view2, &store2.borrow(), &query);

                timer2.borrow_mut().id = None;
                glib::ControlFlow::Break
            });

            timer.borrow_mut().id = Some(id);
        }
    };

    glib::MainContext::default().spawn_local(async move {
        while let Ok(msg) = rx.recv().await {
            let done = matches!(&msg, LoaderMsg::Done);

            {
                let mut map = store.borrow_mut();
                match msg {
                    LoaderMsg::Batch(apps) => {
                        for app in apps {
                            map.insert(app.id.clone(), app);
                        }
                    }
                    LoaderMsg::App(app) => {
                        map.insert(app.id.clone(), app);
                    }
                    LoaderMsg::Remove(ids) => {
                        for id in ids {
                            map.remove(&id);
                        }
                    }
                    LoaderMsg::Done => {}
                }
            }

            if done {
                if let Some(id) = timer.borrow_mut().id.take() {
                    id.remove();
                }
                let query = search_entry.text().to_string();
                rebuild_model(&model, &grid_view, &store.borrow(), &query);
                break;
            } else {
                schedule_rebuild();
            }
        }
    });
}

fn rebuild_model(
    model: &ListStore,
    grid_view: &GridView,
    store: &HashMap<String, DesktopEntry>,
    query: &str,
) {
    let prev_selected_id: Option<String> = grid_view
        .model()
        .and_then(|m| m.downcast::<SingleSelection>().ok())
        .and_then(|sel| sel.selected_item())
        .and_then(|item| item.downcast::<AppEntryObject>().ok())
        .map(|obj| obj.entry().id);

    let mut all: Vec<DesktopEntry> = store.values().cloned().collect();

    if query.is_empty() {
        all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    let filtered: Vec<DesktopEntry> = if query.is_empty() {
        all
    } else {
        crate::search::search_apps(query, &all)
    };

    let config = crate::config::Config::load();
    let (mut pinned, unpinned): (Vec<_>, Vec<_>) = filtered
        .into_iter()
        .partition(|app| config.pinned.contains(&app.id));

    pinned.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    model.remove_all();

    let mut idx: u32 = 0;
    let mut new_selected: Option<u32> = None;

    for app in pinned.iter().chain(unpinned.iter()) {
        if new_selected.is_none() {
            if let Some(ref want) = prev_selected_id {
                if &app.id == want {
                    new_selected = Some(idx);
                }
            }
        }
        model.append(&AppEntryObject::new(app));
        idx += 1;
    }

    if let Some(sel_model) = grid_view.model() {
        if let Some(selection) = sel_model.downcast_ref::<SingleSelection>() {
            let sel = if model.n_items() == 0 {
                u32::MAX
            } else {
                new_selected.unwrap_or(0)
            };
            selection.set_selected(sel);
        }
    }
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
