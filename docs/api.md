# API guide

[English overview](../README.md) · [中文介绍](../README.zh.md) · [Design specification](spec.md)

`gpui-platinum` is a Rust component library built on GPUI. Its public crate name is `gpui_platinum`. The component library lives in `src`; the desktop applications live in `examples/support`.

The crate currently uses the `longbridge/gpui-fast` fork at revision `9690cf8`. An application must use that same GPUI revision: types from a different GPUI package are not interchangeable. Generate the source reference with `cargo doc --no-deps --open`.

## Application setup

Initialize the library once before constructing the first window:

```rust
use gpui::{prelude::*, *};
use gpui_platinum::{TextField, label};

struct Greeting {
    name: Entity<TextField>,
}

impl Render for Greeting {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(16.))
            .child(label("Your name:"))
            .child(self.name.clone())
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        gpui_platinum::init(cx);
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|cx| Greeting {
                name: cx.new(|cx| TextField::new(cx).label("Your name").size(220., 22.)),
            })
        })
        .expect("open application window");
        cx.activate(true);
    });
}
```

This opens one native window. Application-level commands, focus traversal and window lifetime belong to the application. The examples show bindings for Tab/Shift-Tab, Quit, menu commands and closing the last window.

## Ownership and updates

There are two component patterns.

**Lightweight elements** are constructed on each render. They include `button`, `checkbox`, `disclosure`, `ChoiceGroup`, `Slider`, `ScrollBar`, labels and display helpers. Give each interactive element a stable ID; supply its current state; update application state in the callback and call `cx.notify()`.

**Entity controls** retain their state between renders. Construct them with `cx.new`, store the returned `Entity<T>` in the owning view, and render a clone of the handle. Examples include `TextField`, `ListBox`, `Table`, `PopupMenu` and `RichTextArea`. Use `entity.update(cx, |control, cx| ...)` to call setters.

Do not recreate entity controls inside `Render::render`: doing so discards selection, scroll position, composition and undo history. Keep each GPUI `Subscription` alive in the owner or explicitly detach it when it should last as long as the subscribed entity.

### Callbacks

Click handlers can use `cx.listener` directly:

```rust,ignore
button("apply", "Apply").on_click(cx.listener(|this, _, _, cx| {
    this.apply_settings();
    cx.notify();
}))
```

Value callbacks such as `Slider::on_change` pass a value, whereas `cx.listener` expects a reference to an event. Adapt the listener at the boundary:

```rust,ignore
let change = cx.listener(|this, value: &f64, _, cx| {
    this.volume = *value;
    cx.notify();
});

Slider::new("volume", self.volume, 0., 100.)
    .label("Volume")
    .on_change(move |value, window, cx| change(&value, window, cx))
```

Alternatively, capture `cx.weak_entity()` and update the owner from the callback. Examples use this when a value callback needs to coordinate several entities.

### Entity events

Subscribe to a control's typed events in the owning entity's constructor:

```rust,ignore
let field = cx.new(|cx| TextField::new(cx).label("Search"));
let subscription = cx.subscribe(&field, |this, field, event: &InputEvent, cx| {
    if *event == InputEvent::Changed {
        this.query = field.read(cx).value().to_owned();
        cx.notify();
    }
});
```

`InputEvent::Changed` reports an edit; `Submitted` reports Return in a single-line field. Other controls expose events such as `DateChanged`, `ListEvent`, `TreeEvent`, `TableEvent`, `MenuChoice`, `SheetEvent` and `WindowEvent`.

## Buttons and choice groups

- `button(id, label)` returns `Control`. `.default_button()` gives it the outer default ring; `.cancel_button()` assigns its cancel behavior.
- Wrap related buttons and fields in `button_scope(id, content)` to route Return/Enter and Escape/Command-period to that scope's default and cancel buttons. Nested dialogs manage their own scope.
- `checkbox` accepts `CheckState::Off`, `On` or `Mixed`. The callback decides the next value.
- `radio` and `tab` draw individual choices. Use `ChoiceGroup::radio`, `ChoiceGroup::tabs` or `ChoiceGroup::segments` for a group with one tab stop and arrow navigation.
- `disclosure` reports a click through its callback; Left/Right also close/open it when a change is needed.
- `bevel_button` and `icon_button` provide compact buttons. `Control::selected` makes a bevel button a toggle; `Toolbar` arranges controls and moves overflow items into a menu.

Use `.disabled(true)` where supported to make a control skip input and keyboard traversal. Give icon-only controls a meaningful label.

## Text and rich text

`TextField` provides single-line text by default. `.password(true)` masks the display and prevents copying; `.multiline(true)` enables a text area with wrapping and scrolling; `.read_only(true)` allows reading and selection without edits. `.with_text` sets the initial document; `.set_value` replaces it.

Selections and public ranges use UTF-8 byte offsets, not character counts. The input handler translates GPUI's UTF-16 composition ranges at the platform boundary. User navigation follows grapheme boundaries. `set_value` imports a new document and resets its editing history; use selection and replacement APIs for an edit that should participate in undo.

`RichTextArea` combines a native-input text editor with a formatting bar. `RichText` is the corresponding model without a view. `TextStyle`, `StyleAttribute`, `Run`, `Alignment` and `FormatState` describe formatting. Four bitmap scales, bold, italic, underline and paragraph alignment are available. `editor()` exposes the underlying `Entity<TextField>` for commands and `FindReplace`.

`SizeMenu` connects a size selector to an editor. `FontPicker` produces a `FontChoice`; the application decides how to apply the choice. Bitmap families are deliberately limited; selecting a family in the picker does not replace the library's rendering system.

## Collections and navigation

### Lists and trees

`ListBox` contains `ListItem` values with labels and disabled states. `ListEvent::Selected` reports a row; `Activated` reports Return or double-click. Navigation includes arrows, Home/End, Page Up/Down and prefix typing.

`TreeView::from_nodes` accepts `Vec<models::TreeNode>`. A node has a stable ID, title, open state and children. `TreeNode::item`, `.with_children`, `.with_icon` and `.with_data` allow application-owned outline data; `data::<T>()` retrieves a typed payload. `TreeView::new` also accepts the existing `FileTree` model used by the desktop.

`ColumnView` uses the same nodes and presents their ancestor path in adjacent columns. `set_selection(Some(id), cx)` reconstructs the path and reveals the item. A tree's silent selection setter opens ancestors as needed.

### Tables

`Table::new(headers, rows, cx)` is a convenient string-table constructor. `Table::with_columns` supplies explicit presentation:

```rust,ignore
let table = cx.new(|cx| {
    Table::with_columns(
        vec![
            TableColumn::new("Name").width(180),
            TableColumn::new("Size").width(72).right(true),
            TableColumn::new("Kind").width(100).fixed(true),
        ],
        rows,
        cx,
    )
    .size(372, 180)
});
```

`fixed(true)` prevents sorting by a column; its edge can still be resized. `TableColumn::icon` supplies an optional 16-pixel `gpui::ImageSource` by model row index. `set_column_comparator` sorts on underlying values when display strings such as “2 KB” do not sort numerically. Selection indices always refer to the supplied model rows, even after sorting.

Click selects one row; Shift extends a range; Command/Control toggles individual rows. `TableEvent::Selected` reports the current row, `SelectionChanged(BTreeSet<usize>)` reports the selected set and `Activated` reports activation. Observe the collection event when the operation concerns several rows; the cursor may stay on the same row while the selection changes.

`set_selection` and `set_selected_rows` synchronize a view without emitting user-selection events. Use those setters to coordinate several views of one model; avoid writing a single selection back to a table in response to its own multi-selection event.

### Icons and other navigation

`IconItem::new(title, IconKind)` uses the bundled artwork. `.image(source)` or `IconItem::from_image` accepts a custom `ImageSource`, drawn in a 32-pixel square. `IconView` scrolls, selects and activates items; its events also support the desktop example's drag/drop behavior.

`DocumentTabs` manages document selection and close events. `PathPopup`, `Breadcrumbs` and `Pagination` emit path/page changes. `Ruler` emits indentation changes; `Split` reports a divider position and `ScrollView` wraps GPUI scroll state.

## Menus, frames and dialogs

`Menu::icon` accepts static pixel rows for a menu title; `.icon_title(true)` displays its text beside the icon. Keep the title meaningful because it also names the menu for accessibility.

`MenuItem` describes a title, separator, checked/disabled state, shortcut and optional submenu. `MenuBar` and `ContextMenu` report the selected menu and item path through `MenuChoice`. `PopupMenu` reports `MenuEvent(index)` for a single selection.

The containing window should pass keyboard equivalents to `MenuBar::dispatch_shortcut` from its key handler. The menu bar is drawn inside the GPUI window; it does not install an operating-system menu bar.

`PlatinumWindow` is a movable and resizable frame **inside** an existing GPUI window. It does not create a new OS window. The owner handles activation and stacking, and subscribes to close, move, resize and path-selection events. `set_active(false, cx)` propagates inactive presentation to its content. `WindowState` stores position, size and collapsed state. `PopupWindow` provides a bottom-edge tab.

`Alert`, `Dialog`, `FindReplace`, `PageSetup` and `ProgressDialog` are retained entities. Construct and retain them, use their `show`/`close` methods and render them with the owner. Dialogs save and restore focus, keep Tab traversal within the modal layer and provide default/cancel actions.

`OpenDialog` and `SaveDialog` operate on an `Entity<models::FileTree>`. They browse an application's model and emit an open/save result. The owner performs the document operation. Use GPUI's platform APIs separately when a native filesystem picker is needed.

`Banner` and `Toasts` provide retained notifications. `Balloon` attaches delayed help to an element. Display helpers such as `status_bar`, `placard`, `badge`, `card` and `empty_state` compose directly with GPUI layout.

## Focus, input and accessibility

Bind Tab and Shift-Tab to `window.focus_next(cx)` and `window.focus_prev(cx)` at the application level, as the examples do. Retained controls implement `Focusable`, so their handles can be focused through GPUI. Choice groups manage their own arrow navigation; sheets move through cells and release Tab at the outer boundary.

Embedded scroll bars can use `.tab_stop(false)` to preserve the containing editor's or collection's focus. Standalone scroll bars remain keyboard-focusable by default.

Controls expose GPUI/AccessKit roles, labels and relevant states. Tables and sheets expose cell nodes in addition to their pixel painting. Names should describe the action or value; visible labels alone are not a substitute for a label on an icon-only input. Accessibility behavior still needs runtime checks with the screen reader and platform targeted by an application.

## Themes, language and drawing

`Theme` contains the background and an `Accent`; `theme::ACCENTS` contains six accents. Most controls accept a theme builder and entity controls expose `set_theme`. The application owns theme propagation.

`Language` has `English`, `SimplifiedChinese`, `TraditionalChinese`, `Japanese` and `Korean`. `.language(...)` selects control wording and glyph forms; `set_language` updates retained controls. Literal application labels remain application-owned. `tr(language, key)` reads a control catalog entry; `locale::desk` reads the desktop catalog. Catalog JSON is embedded at compile time.

`Font::truncate` and `Font::truncate_in` return text shortened with an ellipsis to fit a logical-pixel width, with the latter measuring the selected language's regional glyphs.

The regular and bold Latin bitmap faces are bundled with four regional CJK faces. System-font fallback handles missing bitmap characters after `init`. Date/time controls use the selected language's built-in order and hour cycle, with explicit overrides where the control offers them.

`paint::Pen` draws logical-pixel fills and text, and `drawing` embeds a pen callback in GPUI. `draw_*` functions are useful for a custom control or appearance fixture; they do not create focus, input handling or accessibility nodes. A custom interactive control must add those separately.

## Examples

| Example | Purpose |
|---|---|
| `gallery` | A paginated visual overview of the component families |
| `controls` | A browsable catalog with working entity controls and dialogs |
| `desktop` | Window stacking, menus, virtual files and small desktop applications |

Run them with `cargo run --example <name>`. `./scripts/bundle.sh --gallery` produces a macOS app bundle for the gallery. `./scripts/screenshots.sh` regenerates the five gallery images and two desktop images with the GPUI renderer; `--offline` uses cached dependencies. The desktop accepts `--lang en`, `zh-Hans`, `zh-Hant`, `ja` or `ko`. PNG export requires the platform's GPUI offscreen renderer, which the pinned revision provides on macOS and Linux; it is unavailable on Windows.

## Relationship to the Go project

The original Go library binds values by pointer and reports changes when an immediate-mode view is rebuilt. This port uses GPUI's retained entities and typed events. Names follow Rust conventions:

| Go API | GPUI API |
|---|---|
| `Button`, `Checkbox`, `Disclosure` | `button`, `checkbox`, `disclosure` |
| `RadioGroup`, `TabControl`, `Segmented` | `ChoiceGroup::radio`, `::tabs`, `::segments` |
| `TextArea` | `TextField::multiline(true)` |
| `Calendar`, `Sheet` | `CalendarView`, `SheetView` |
| `FormulaBar`, `FormatBar` | composed into `SheetView` and `RichTextArea` |
| `Window` | `PlatinumWindow` |
| `Find`, `ReplaceAll` | `find_next`, `replace_all` |
| `SpringOpen`, `ZoomRects` | `PopupWindow::spring_open`, `zoom_rects` |

There is no drop-in compatibility with the Go API. In particular, the Go `WindowOptions` configuration and arbitrary locale catalogs do not have an identical Rust API. The design specification describes the shared visual target; the reference generated from the current Rust source is authoritative for callable methods.
