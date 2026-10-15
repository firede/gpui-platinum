//! Pixel-drawn controls in the Platinum desktop style.

pub mod bitmap_icons;
pub mod calendar;
pub mod choice;
pub mod collections;
pub mod dialogs;
pub mod display;
pub mod extras;
pub mod font;
pub mod forms;
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
pub use calendar::{CalendarView, DateChanged};
pub use choice::{ChoiceGroup, ChoiceItem};
pub use collections::{Table, TableEvent, TreeEvent, TreeView};
pub use dialogs::*;
pub use display::*;
pub use extras::*;
pub use font::{Font, Language};
pub use forms::*;
pub use list::{ListBox, ListEvent, ListItem};
pub use locale::tr;
pub use menu::{MenuEvent, MenuItem, PopupMenu};
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
pub use window::{PlatinumWindow, WindowEvent, WindowState};
