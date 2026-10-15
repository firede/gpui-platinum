//! Pixel-drawn controls in the Platinum desktop style.

pub mod calendar;
pub mod choice;
pub mod collections;
pub mod display;
pub mod font;
pub mod list;
pub mod menu;
pub mod models;
pub mod paint;
pub mod rich_text;
pub mod sheet;
pub mod slider;
pub mod text_input;
pub mod theme;
pub mod widgets;
pub mod window;

pub use calendar::{CalendarView, DateChanged};
pub use choice::{ChoiceGroup, ChoiceItem};
pub use collections::{Table, TableEvent, TreeEvent, TreeView};
pub use display::*;
pub use font::{Font, Language};
pub use list::{ListBox, ListEvent, ListItem};
pub use menu::{MenuEvent, MenuItem, PopupMenu};
pub use rich_text::{
    Alignment, FormatState, RichText, RichTextArea, Run, StyleAttribute, TextStyle,
};
pub use sheet::{SheetEvent, SheetView};
pub use slider::{ScrollBar, Slider};
pub use text_input::{InputEvent, TextField};
pub use theme::{Accent, Theme};
pub use widgets::*;
pub use window::{PlatinumWindow, WindowEvent, WindowState};
