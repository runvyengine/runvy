use egui;
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use egui::{Id, Ui, WidgetText};

// First, let's pick a type that we'll use to attach some data to each tab.
// It can be any type.
type Tab = String;

// To define the contents and properties of individual tabs, we implement the `TabViewer`
// trait. Only three things are mandatory: the `Tab` associated type, and the `ui` and
// `title` methods. There are more methods in `TabViewer` which you can also override.
struct MyTabViewer;

impl TabViewer for MyTabViewer {
    // This associated type is used to attach some data to each tab.
    type Tab = Tab;

    // Returns the unique ID for the `tab`.
    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(tab)
    }

    // Returns the current `tab`'s title.
    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.as_str().into()
    }

    // Defines the contents of a given `tab`.
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
    }
}

// Here is a simple example of how you can manage a `DockState` of your application.
struct MyTabs {
    dock_state: DockState<Tab>
}

impl MyTabs {
    pub fn new() -> Self {
        // Create a `DockState` with an initial tab "tab1" in the main `Surface`'s root node.
        let tabs = ["tab1", "tab2", "tab3"].map(str::to_string).into_iter().collect();
        let dock_state = DockState::new(tabs);
        Self { dock_state }
    }

    fn ui(&mut self, ui: &mut Ui) {
        // Here we just display the `DockState` using a `DockArea`.
        // This is where egui handles rendering and all the integrations.
        //
        // We can specify a custom `Style` for the `DockArea`, or just inherit
        // all of it from egui.
        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, &mut MyTabViewer);
    }
}

pub fn run() {
    
}