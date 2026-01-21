use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
    time::Duration,
};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gtk4::{
    gio::ListStore,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
    Box, CustomFilter, CustomSorter, Entry, FilterChange, FilterListModel, GridView, ListItem,
    ListScrollFlags, Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection,
    SortListModel, SorterChange,
};

pub mod components;
pub use components::{create_app_row, populate_app_row};

use crate::{
    config::Config,
    desktop::{DesktopEntry, LoaderMsg},
};

const SCORE_NO_MATCH: i64 = i64::MIN;
const BATCH_CHUNK: usize = 150;
const SAVE_PINS_DEBOUNCE_MS: u64 = 200;

type ObjById = Rc<RefCell<HashMap<String, AppEntryObject>>>;

#[derive(Clone)]
pub struct UiController {
    base: ListStore,
    by_id: ObjById,
    query: Rc<RefCell<String>>,
    pinned: Rc<RefCell<HashSet<String>>>,
    filter: CustomFilter,
    sorter: CustomSorter,
    grid_view: glib::WeakRef<GridView>,
    selection_guard: Rc<Cell<bool>>,
    pins_save_source: Rc<RefCell<Option<glib::SourceId>>>,
}

impl UiController {
    pub fn set_query(&self, new_query: String) {
        let was_empty = self.query.borrow().is_empty();

        let prev_selected_id = self.selected_app_id();

        *self.query.borrow_mut() = new_query.clone();
        self.update_scores(&new_query);

        self.filter.changed(FilterChange::Different);
        self.sorter.changed(SorterChange::Different);

        if new_query.is_empty() && !was_empty {
            if let Some(grid_view) = self.grid_view.upgrade() {
                if let Some(model) = grid_view.model() {
                    if let Some(selection) = model.downcast_ref::<SingleSelection>() {
                        if selection.n_items() > 0 {
                            self.selection_guard.set(true);
                            selection.set_selected(0);
                            self.selection_guard.set(false);
                            grid_view.scroll_to(0, ListScrollFlags::NONE, None);
                        }
                    }
                }
            }
            return;
        }

        self.reselect_by_id(prev_selected_id);
    }

    pub fn pinned_snapshot(&self) -> HashSet<String> {
        self.pinned.borrow().iter().cloned().collect()
    }

    pub fn toggle_pin(&self, app_id: &str) -> bool {
        let prev_selected_id = self.selected_app_id();

        let now_pinned = {
            let mut pinned = self.pinned.borrow_mut();
            if pinned.remove(app_id) {
                false
            } else {
                pinned.insert(app_id.to_string());
                true
            }
        };

        self.request_save_pins();
        self.sorter.changed(SorterChange::Different);
        self.reselect_by_id(prev_selected_id);

        now_pinned
    }

    pub fn cleanup_stale_pins(&self) {
        let existing: HashSet<String> = self.by_id.borrow().keys().cloned().collect();

        let changed = {
            let mut pinned = self.pinned.borrow_mut();
            let before = pinned.len();
            pinned.retain(|id| existing.contains(id));
            pinned.len() != before
        };

        if changed {
            self.request_save_pins();
            self.sorter.changed(SorterChange::Different);
        }
    }

    fn request_save_pins(&self) {
        if let Some(old) = self.pins_save_source.borrow_mut().take() {
            old.remove();
        }

        let pinned_vec: Vec<String> = self.pinned.borrow().iter().cloned().collect();

        let mut pinned_opt = Some(pinned_vec);

        let source_id =
            glib::timeout_add_local(Duration::from_millis(SAVE_PINS_DEBOUNCE_MS), move || {
                if let Some(pins) = pinned_opt.take() {
                    Config { pinned: pins }.save();
                }
                glib::ControlFlow::Break
            });

        *self.pins_save_source.borrow_mut() = Some(source_id);
    }

    pub fn upsert_entry(&self, entry: DesktopEntry) {
        let id = entry.id.clone();

        let query = self.query.borrow().clone();
        let score = compute_score(&entry.name, &query);
        let obj = AppEntryObject::new(entry, score);

        if self.by_id.borrow().contains_key(&id) {
            if let Some(idx) = find_in_base(&self.base, &id) {
                self.base.remove(idx);
                self.base.insert(idx, &obj);
            } else {
                self.base.append(&obj);
            }
        } else {
            self.base.append(&obj);
        }

        self.by_id.borrow_mut().insert(id, obj);
    }

    pub fn remove_ids<I>(&self, ids: I)
    where
        I: IntoIterator<Item = String>,
    {
        let mut pins_changed = false;

        for id in ids {
            self.by_id.borrow_mut().remove(&id);
            if let Some(idx) = find_in_base(&self.base, &id) {
                self.base.remove(idx);
            }
            if self.pinned.borrow_mut().remove(&id) {
                pins_changed = true;
            }
        }

        if pins_changed {
            self.request_save_pins();
            self.sorter.changed(SorterChange::Different);
        }
    }

    fn update_scores(&self, query: &str) {
        let matcher = SkimMatcherV2::default().ignore_case();

        for i in 0..self.base.n_items() {
            let Some(item) = self.base.item(i) else {
                continue;
            };
            let Ok(obj) = item.downcast::<AppEntryObject>() else {
                continue;
            };

            let score = if query.is_empty() {
                0
            } else {
                matcher
                    .fuzzy_match(&*obj.name_ref(), query)
                    .unwrap_or(SCORE_NO_MATCH)
            };

            obj.set_score(score);
        }
    }

    fn selected_app_id(&self) -> Option<String> {
        let grid_view = self.grid_view.upgrade()?;
        let model = grid_view.model()?;
        let selection = model.downcast_ref::<SingleSelection>()?;
        let item = selection.selected_item()?;
        let obj = item.downcast::<AppEntryObject>().ok()?;
        let id = obj.id_ref().to_string();
        Some(id)
    }

    fn reselect_by_id(&self, want_id: Option<String>) {
        let Some(want_id) = want_id else { return };

        let Some(grid_view) = self.grid_view.upgrade() else {
            return;
        };
        let Some(model) = grid_view.model() else {
            return;
        };
        let Some(selection) = model.downcast_ref::<SingleSelection>() else {
            return;
        };
        let Some(list_model) = selection.model() else {
            return;
        };

        for idx in 0..list_model.n_items() {
            let Some(item) = list_model.item(idx) else {
                continue;
            };
            let Ok(obj) = item.downcast::<AppEntryObject>() else {
                continue;
            };

            if &*obj.id_ref() == want_id {
                self.selection_guard.set(true);
                selection.set_selected(idx);
                self.selection_guard.set(false);
                grid_view.scroll_to(idx, ListScrollFlags::NONE, None);
                return;
            }
        }
    }
}

pub fn build_content() -> (Box, UiController) {
    let selection_guard = Rc::new(Cell::new(false));
    let pins_save_source = Rc::new(RefCell::new(None));

    let container = Box::new(Orientation::Vertical, 0);
    container.set_hexpand(false);
    container.set_vexpand(false);
    container.set_overflow(gtk4::Overflow::Hidden);

    let (search_box, search_entry) = create_search_box();
    container.append(&search_box);

    let base = ListStore::new::<AppEntryObject>();
    let by_id: ObjById = Rc::new(RefCell::new(HashMap::new()));

    let query: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let pinned: Rc<RefCell<HashSet<String>>> =
        Rc::new(RefCell::new(Config::load().pinned.into_iter().collect()));

    let filter = CustomFilter::new({
        let query = query.clone();
        move |obj| {
            let q = query.borrow();
            if q.is_empty() {
                return true;
            }
            let entry = obj
                .downcast_ref::<AppEntryObject>()
                .expect("AppEntryObject expected");
            entry.score() != SCORE_NO_MATCH
        }
    });

    let filtered = FilterListModel::new(Some(base.clone()), Some(filter.clone()));

    let sorter = CustomSorter::new({
        let pinned = pinned.clone();
        let query = query.clone();

        move |a, b| {
            let a = a
                .downcast_ref::<AppEntryObject>()
                .expect("AppEntryObject expected");
            let b = b
                .downcast_ref::<AppEntryObject>()
                .expect("AppEntryObject expected");

            let pinned_set = pinned.borrow();
            let ap = pinned_set.contains(&*a.id_ref());
            let bp = pinned_set.contains(&*b.id_ref());
            drop(pinned_set);

            match bp.cmp(&ap) {
                std::cmp::Ordering::Equal => {
                    if ap {
                        return a.name_key_ref().cmp(&*b.name_key_ref()).into();
                    }

                    if query.borrow().is_empty() {
                        a.name_key_ref().cmp(&*b.name_key_ref()).into()
                    } else {
                        b.score()
                            .cmp(&a.score())
                            .then_with(|| a.name_key_ref().cmp(&*b.name_key_ref()))
                            .into()
                    }
                }
                other => other.into(),
            }
        }
    });

    let sorted = SortListModel::new(Some(filtered), Some(sorter.clone()));

    let selection = SingleSelection::new(Some(sorted));
    selection.set_autoselect(true);

    let grid_view = create_virtual_list(&selection);

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled.set_min_content_height(400);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&grid_view));
    container.append(&scrolled);

    let status_bar = create_status_bar();
    container.append(&status_bar);

    let ui = UiController {
        base,
        by_id,
        query,
        pinned,
        filter,
        sorter,
        grid_view: grid_view.downgrade(),
        selection_guard,
        pins_save_source,
    };

    setup_search(&search_entry, ui.clone());

    glib::idle_add_local_once({
        let ui = ui.clone();
        move || start_loader(ui)
    });

    (container, ui)
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

fn create_virtual_list(selection: &SingleSelection) -> GridView {
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

    let grid_view = GridView::new(Some(selection.clone()), Some(factory));
    grid_view.set_max_columns(1);
    grid_view.set_min_columns(1);
    grid_view.set_single_click_activate(true);
    grid_view.set_can_target(false);

    setup_selection_scroll(&grid_view);
    setup_activation(&grid_view);

    grid_view
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

fn setup_search(search_entry: &Entry, ui: UiController) {
    search_entry.connect_changed(move |entry| {
        ui.set_query(entry.text().to_string());
    });
    search_entry.grab_focus();
}

fn setup_activation(grid_view: &GridView) {
    let grid_view_clone = grid_view.clone();
    grid_view.connect_activate(move |_, _| launch_selected(&grid_view_clone));
}

fn start_loader(ui: UiController) {
    let done_received = Rc::new(Cell::new(false));
    let finalized = Rc::new(Cell::new(false));

    let (tx, rx) = async_channel::unbounded::<LoaderMsg>();
    let pinned_snapshot = ui.pinned_snapshot();
    crate::desktop::spawn_load_entries(tx, pinned_snapshot);

    let pending: Rc<RefCell<VecDeque<DesktopEntry>>> = Rc::new(RefCell::new(VecDeque::new()));
    let draining = Rc::new(Cell::new(false));

    let schedule_drain = {
        let ui = ui.clone();
        let pending = pending.clone();
        let draining = draining.clone();
        let done_received = done_received.clone();
        let finalized = finalized.clone();

        move || {
            if draining.get() {
                return;
            }
            draining.set(true);

            let ui2 = ui.clone();
            let pending2 = pending.clone();
            let draining2 = draining.clone();
            let done2 = done_received.clone();
            let finalized2 = finalized.clone();

            glib::idle_add_local(move || {
                let mut q = pending2.borrow_mut();

                for _ in 0..BATCH_CHUNK {
                    if let Some(entry) = q.pop_front() {
                        ui2.upsert_entry(entry);
                    } else {
                        draining2.set(false);

                        if done2.get() && !finalized2.get() {
                            finalized2.set(true);
                            drop(q);
                            ui2.cleanup_stale_pins();
                        }

                        return glib::ControlFlow::Break;
                    }
                }

                glib::ControlFlow::Continue
            });
        }
    };

    glib::MainContext::default().spawn_local(async move {
        while let Ok(msg) = rx.recv().await {
            let done = matches!(msg, LoaderMsg::Done);

            match msg {
                LoaderMsg::Batch(apps) => {
                    pending.borrow_mut().extend(apps);
                    schedule_drain();
                }
                LoaderMsg::App(app) => ui.upsert_entry(app),
                LoaderMsg::Remove(ids) => ui.remove_ids(ids),
                LoaderMsg::Done => {
                    done_received.set(true);
                    schedule_drain();
                }
            }

            if done {
                break;
            }
        }
    });

    glib::timeout_add_local(Duration::from_millis(1), || glib::ControlFlow::Break);
}

fn compute_score(name: &str, query: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }

    SkimMatcherV2::default()
        .ignore_case()
        .fuzzy_match(name, query)
        .unwrap_or(SCORE_NO_MATCH)
}

fn find_in_base(base: &ListStore, id: &str) -> Option<u32> {
    for i in 0..base.n_items() {
        let item = base.item(i)?;
        let obj = item.downcast::<AppEntryObject>().ok()?;
        if &*obj.id_ref() == id {
            return Some(i);
        }
    }
    None
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
    use std::cell::{Cell, RefCell};

    use gtk4::glib::{self, subclass::prelude::*};

    #[derive(Default)]
    pub struct AppEntryObject {
        pub entry: RefCell<Option<crate::desktop::DesktopEntry>>,
        pub id: RefCell<String>,
        pub name: RefCell<String>,
        pub name_key: RefCell<String>,
        pub score: Cell<i64>,
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
    pub fn new(entry: crate::desktop::DesktopEntry, score: i64) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().id.replace(entry.id.clone());
        obj.imp().name.replace(entry.name.clone());
        // Preserve previous case-insensitive ordering without allocating inside the sorter.
        obj.imp().name_key.replace(entry.name.to_lowercase());
        obj.imp().entry.replace(Some(entry));
        obj.imp().score.set(score);
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

    pub fn score(&self) -> i64 {
        self.imp().score.get()
    }

    pub fn set_score(&self, score: i64) {
        self.imp().score.set(score);
    }

    pub fn id_ref(&self) -> std::cell::Ref<'_, str> {
        std::cell::Ref::map(self.imp().id.borrow(), |s| s.as_str())
    }

    pub fn name_ref(&self) -> std::cell::Ref<'_, str> {
        std::cell::Ref::map(self.imp().name.borrow(), |s| s.as_str())
    }

    pub fn name_key_ref(&self) -> std::cell::Ref<'_, str> {
        std::cell::Ref::map(self.imp().name_key.borrow(), |s| s.as_str())
    }
}
