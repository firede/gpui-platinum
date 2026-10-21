//! Pixel-drawn controls in the Platinum desktop style.
//!
//! Call [`init`] before opening the first GPUI window. Retain stateful controls
//! as entities and rebuild lightweight controls from application state.
//!
//! ```
//! use gpui::prelude::*;
//! use gpui_platinum::{button, button_scope};
//!
//! let actions = button_scope(
//!     "document-actions",
//!     gpui::div().child(button("save", "Save").default_button()),
//! );
//! ```

pub mod bitmap_icons;
pub mod calendar;
pub mod choice;
pub mod collections;
pub mod dialogs;
pub mod display;
pub mod extras;
pub mod font;
pub mod forms;
mod inactive;
pub mod list;
pub mod locale;
pub mod menu;
pub mod menu_bar;
pub mod models;
pub mod navigation;
pub mod paint;
pub mod rich_text;
pub mod sheet;
pub mod slider;
pub mod text_input;
pub mod theme;
pub mod toolbar;
pub mod widgets;
pub mod window;

pub use bitmap_icons::*;
pub use calendar::{CalendarView, DateChanged, DateChosen};
pub use choice::{ChoiceGroup, ChoiceItem, draw_tab_pane};
pub use collections::{Table, TableColumn, TableEvent, TreeEvent, TreeView};
pub use dialogs::*;
pub use display::*;
pub use extras::*;
pub use font::{Font, Language};
pub use forms::*;
pub use list::{LIST_ROW_HEIGHT, ListBox, ListEvent, ListItem, draw_list_box};
pub use locale::tr;
pub use menu::{MENU_FLASH, MenuEvent, MenuItem, PopupMenu};
pub use menu_bar::{ContextMenu, Menu, MenuBar, MenuChoice};
pub use navigation::*;
pub use rich_text::{
    Alignment, FormatState, RichText, RichTextArea, Run, SizeMenu, StyleAttribute, TextStyle,
    size_menu,
};
pub use sheet::{SheetEvent, SheetView};
pub use slider::{ScrollBar, Slider};
pub use text_input::{InputEvent, TextField};
pub use theme::{Accent, Theme};
pub use widgets::*;
pub use window::{
    PlatinumWindow, PlatinumWindowOptions, WindowEvent, WindowPart, WindowState, draw_grow_box,
    draw_title_box, draw_window_frame, window_part_at,
};

/// Registers native text editing, font fallback and dialog keyboard bindings.
/// Call this once before opening the application's first window.
pub fn init(cx: &mut gpui::App) {
    text_input::init(cx);
    dialogs::init(cx);
}
