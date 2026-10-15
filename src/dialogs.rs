//! Modal dialogs and notifications in the Platinum style.

mod alert;
mod file;
mod find;
mod modal;
mod notifications;
mod page_setup;

pub use alert::{Alert, AlertEvent, AlertKind, Dialog, DialogEvent, ProgressDialog, ProgressEvent};
pub use file::{FileDialogEvent, OpenDialog, SaveDialog, SaveTarget};
pub use find::{FindEvent, FindOptions, FindReplace, find_next, find_ranges, replace_all};
pub use modal::init;
pub use notifications::{Banner, BannerEvent, Toast, ToastEvent, Toasts};
pub use page_setup::{PAPERS, PageSetup, PageSetupEvent, PageSetupState, Paper};
