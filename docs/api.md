# API guide

[README](../README.md) · [简体中文](../README.zh.md) ·
[Design specification](spec.md)

The crate is `gpui-platinum`, imported as `gpui_platinum`. The controls are
in `src`; the desktop's applications are in `examples/support`. It is built
on gpui-fast v0.1.12 from crates.io, as limu-gpui is, and an application
must use that crate too: types from another GPUI are not interchangeable.
`cargo doc --no-deps --open` opens the reference generated from the
source, which is the last word on what can be called.

## Application setup

Call `gpui_platinum::init` once, before the first window. It registers text
editing, font fallback and the dialogs' keys.

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
        .expect("open a window");
        cx.activate(true);
    });
}
```

The application owns its commands, Tab order and windows. The examples bind
Tab and Shift-Tab, Quit and menu commands, and quit when the last window
closes.

## Two kinds of control

**Elements** are built anew on every render from the application's state:
`button`, `checkbox`, `disclosure`, `ChoiceGroup`, `Slider`, `ScrollBar`,
`Split`, the text functions and the displays. Give each interactive element
a stable ID, pass its current value, and in its callback update the state
and call `cx.notify()`.

**Entities** keep state between renders: `TextField`, `RichTextArea`,
`ListBox`, `Table`, `PopupMenu`, the pickers and the dialogs among them.
Make each once with `cx.new`, keep the `Entity<T>` in the view that owns
it, and render a clone of the handle. Call setters with
`entity.update(cx, |control, cx| …)`.

Do not make an entity in `Render::render`: a new one loses its selection,
scroll position, composition and undo history. Keep each `Subscription` in
its owner, or `detach` it to let it live as long as the entity it watches.

### Callbacks

A click handler takes `cx.listener` as it is:

```rust,ignore
button("apply", "Apply").on_click(cx.listener(|this, _, _, cx| {
    this.apply_settings();
    cx.notify();
}))
```

A value callback, such as `Slider::on_change`, passes the value, where
`cx.listener` expects a reference. Adapt it:

```rust,ignore
let change = cx.listener(|this, value: &f64, _, cx| {
    this.volume = *value;
    cx.notify();
});

Slider::new("volume", self.volume, 0., 100.)
    .label("Volume")
    .on_change(move |value, window, cx| change(&value, window, cx))
```

Or capture `cx.weak_entity()` and update the owner through it, as the
examples do where one callback reaches several entities.

### Events

Subscribe to an entity's events where its owner is made:

```rust,ignore
let field = cx.new(|cx| TextField::new(cx).label("Search"));
let subscription = cx.subscribe(&field, |this, field, event: &InputEvent, cx| {
    if *event == InputEvent::Changed {
        this.query = field.read(cx).value().to_owned();
        cx.notify();
    }
});
```

`InputEvent::Changed` reports an edit and `Submitted` Return in a
single-line field. Other entities report `MenuEvent`, `MenuChoice`,
`ListEvent`, `TreeEvent`, `TableEvent`, `ColumnEvent`, `IconEvent`,
`DateChanged`, `TimeChanged`, `ColorChanged`, `FontChanged`, `SheetEvent`,
`WindowEvent`, `FileDialogEvent` and others; each is named in the
reference.

## Buttons and choices

- `button(id, label)` returns a `Control`. `.default_button()` gives it the
  default ring and `.cancel_button()` makes it the cancel button.
- `button_scope(id, content)` sends Return and Enter to the default button
  inside it, and Escape and Command-period to the cancel button. A dialog
  is a scope of its own.
- `checkbox` takes `CheckState::Off`, `On` or `Mixed`; its callback decides
  the next state.
- `radio` and `tab` draw one choice. `ChoiceGroup::radio`, `::tabs` and
  `::segments` make a group with one tab stop, moved through with the
  arrow keys. `draw_tab_pane` draws the pane under a row of tabs.
- `disclosure` reports a click; Left and Right also close and open it.
- `bevel_button` and `icon_button` make small square buttons.
  `Control::selected` makes a bevel button a toggle, and
  `Control::menu_arrow` shows the pop-down arrow of a button that opens a
  menu.
- `Toolbar` lays out controls and moves those that do not fit into an
  overflow menu.

`.disabled(true)` dims a control and takes it out of Tab order. Give a
control that shows only an icon a label for screen readers.

## Text

`label` and `text_in` return a `TextIn`: one line sized to its text. Made
as tall as a control beside it, it centers its capitals as the control
centers its own. `paragraph` wraps text to a width.

`TextField` is a single-line field. `.password(true)` masks it and forbids
copying; `.multiline(true)` makes a text area that wraps and scrolls;
`.read_only(true)` lets text be selected but not changed; `.indent(first)`
starts each paragraph's first line further in. `.document(true)` makes a
text area the content of a document window: white to the window's edges,
its scroll bar on the window's line, with no ring of its own;
`set_document_layout` sets its margins. `.with_text` gives the first text
and `set_value` replaces it, clearing the undo history.

Selections and ranges are UTF-8 byte offsets. The field converts GPUI's
UTF-16 composition ranges, and the caret moves by grapheme.

Lines of text make room for East Asian glyphs, as the original's do once it
has East Asian faces. `text_input::set_east_asian_line_room(false)` fits
English lines to the Latin face alone, as the original's are without them.
The setting holds for the whole process; lines in the other languages
always make room.

`RichTextArea` is a text area with a format bar: bold, italic, underline,
four sizes and paragraph alignment. `.show_format_bar(false)` hides the
bar, leaving its shortcuts; `.bar_gap` puts room between the bar and the
text. `format_items` adds the bar's buttons to a `Toolbar` of one's own,
for a window that keeps them with its other tools. `editor()` returns the `Entity<TextField>` under it, for commands and
for `FindReplace`. `RichText` is the model without a view; `TextStyle`,
`StyleAttribute`, `Run`, `Alignment` and `FormatState` describe its
formatting. `SizeMenu` sets an editor's size.

## Collections

### Lists, trees and columns

`ListBox` shows `ListItem`s, each with a label and a `disabled` flag, and
`.icons` gives rows pictures. `ListEvent::Selected` reports a row and
`Activated` Return or a double click. The arrow keys, Home, End, Page Up,
Page Down and typing a name's first letters move the selection.

`TreeView::from_nodes` takes a `Vec<models::TreeNode>`: each node has an
ID, a title, an open state and children. `TreeNode::item`,
`.with_children`, `.with_icon` and `.with_data` build an outline of the
application's own, and `data::<T>()` reads a node's data back.
`TreeView::new` takes a `models::FileTree`, as the desktop's disk.

`ColumnView` shows the same nodes in columns, one per level of the path to
the item chosen. `set_selection(Some(id), cx)` opens the path to an item
and shows it.

### Tables

`Table::new(headers, rows, cx)` makes a table of strings.
`Table::with_columns` sets each column up:

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

A `fixed(true)` column does not sort, though its edge still resizes.
`TableColumn::icon` gives each row, by its model index, a picture of up to
16×16 pixels, drawn from rows of characters. `set_column_comparator` sorts
a column by its values where the text shown, as "2 KB", would sort wrong.
Row indices always refer to the model's rows, sorted or not.
`.focus_ring(false)` leaves out the ring for a table that fills a window.

A click chooses a row, Shift extends the range, and Command or Control
adds or removes one row. `TableEvent::Selected` reports the current row,
`SelectionChanged` the set chosen, and `Activated` an activation. Watch
`SelectionChanged` for actions on several rows: the current row can stay
while the set changes.

`set_selection` and `set_selected_rows` change the selection without an
event, to keep several views of one model in step. Do not write one row
back to a table in answer to its own `SelectionChanged`.

### Icons

`IconItem::new(title, IconKind)` uses the library's icons;
`IconItem::from_image` and `.image` take an `ImageSource`, drawn in a
32-pixel square. A 32×32 `RenderImage` of pixels each opaque or clear is
drawn pixel for pixel and, when chosen, darkened by half, as the original
does; `.selected_image` gives a chosen look of its own. `IconView` scrolls,
chooses, opens and drags items. `.frame(true)` draws the sunken outline of
a view standing alone, `.always_scrollbar(true)` keeps its scroll bar, and
`.scroll_bar_room` leaves room under the bar for a window's resize corner. `.plain_when_inactive(true)` leaves the
names plain while the window is in the background.

### Navigation and layout

`DocumentTabs` reports `DocumentTabEvent::Selected` and `Closed`.
`PathPopup`, `Breadcrumbs` and `Pagination` report the path or page
chosen, and `Ruler` its indents.

`Split` reports its divider's position: the first pane's width in pixels,
or, with `.percent()`, the share of the room it takes, so both panes grow
and shrink with the split. `.vertical()` stacks the panes. A split too
small for the first pane's width shrinks that pane, keeping the second's
least, and gives the width back as it grows. `ScrollView` draws GPUI's
scrolling in the content frame.

## Menus, windows and dialogs

`PopupMenu` reports `MenuEvent(index)`. `.caption("Sort by:")` puts text
before the button; `.bevel(icon)` draws it as a bevel button with an icon
and the pop-down arrow, as a file dialog's folder.

`MenuItem` has a title, a check mark, a disabled state, a keyboard
equivalent and an optional submenu, or is a separator. `MenuBar` and
`ContextMenu` report the menu and item chosen as a `MenuChoice`.
`Menu::icon` gives a menu an icon for a title, and `.icon_title(true)`
shows its text beside it; the text also names the menu for screen
readers. The window's key handler passes keys to
`MenuBar::dispatch_shortcut`. The menu bar is drawn in the GPUI window; it
does not install the system's menu bar.

`PlatinumWindow` is a frame that moves and resizes **inside** a GPUI
window; it does not open a window of the system. Its owner raises it and
handles its `WindowEvent`s: activation, closing, moving, resizing and the
path menu's choice. `set_active(false, cx)` draws it and its content as a
window in the background. `WindowState` holds its position, size and
whether it is rolled up. `PopupWindow` is a window docked at the bottom
edge, as a tab.

`Alert`, `Dialog`, `ProgressDialog`, `FindReplace` and `PageSetup` are
entities: keep them, call `show` and `close`, and render them with their
owner. A dialog keeps Tab inside itself, gives the focus back when it
closes, and answers Return and Escape with its default and cancel
buttons. `ColorWell` opens its color picker as such a dialog, titled with
the well's label. `FontPicker` reports a `FontChoice` and draws its sample
in the style chosen; the application decides what to do with it.

`OpenDialog` and `SaveDialog` browse an `Entity<models::FileTree>` and
report `FileDialogEvent::Opened` or `Saved`; the owner opens or saves the
document. `.new_folders(true)` lets New Folder make a folder; it is dimmed
otherwise. For the system's own file picker, use GPUI's platform APIs.

`Banner` and `Toasts` show notices. `Balloon` gives an element help that
shows after a delay. `card()` is a raised panel, a column filled with
`.child`; `empty_state(text)` centers one line of gray text in the room its
parent gives it. `status_bar`, `placard`, `badge`, `tag` and the other
displays are elements laid out like any other.

## Focus, keyboard and accessibility

Bind Tab and Shift-Tab to `window.focus_next(cx)` and
`window.focus_prev(cx)`, as the examples do. Entities implement
`Focusable`. A choice group moves with the arrow keys inside one tab stop;
a sheet moves through its cells and lets Tab out at its edges. A scroll
bar inside an editor or a list takes `.tab_stop(false)` to leave the focus
there; one standing alone keeps its tab stop.

Controls give GPUI's accessibility tree their roles, names and states;
tables and sheets give their cells too. A name says what the control does
or holds. Check an application with the screen reader of each platform it
targets.

## Themes and languages

`Theme` holds the background and an `Accent`; `theme::ACCENTS` holds the
six accents. Controls take a theme with `.theme(…)`, and entities change
it with `set_theme`; the application passes it on.

`Language` is `English`, `SimplifiedChinese`, `TraditionalChinese`,
`Japanese` or `Korean`. `.language(…)` chooses a control's words and the
forms of its East Asian glyphs, and `set_language` changes them. Text the
application passes in stays as it is. `tr(language, key)` reads the
controls' catalog and `locale::desk` the desktop's; both are compiled in.
Calendars, date pickers and time fields write dates as the language does;
`first_weekday`, `day_first` and `hour_cycle` set the regional formats.

## Drawing

`paint::Pen` draws fills and text on a grid of logical pixels, and
`drawing(width, height, |pen, window, cx| …)` makes an element of it.
`rect`, `frame`, `bevel` and `dot` fill; `translucent` blends a fill over
what is under it and `drop_shadow` draws a menu's or an alert's shadow;
`text` draws a line and `text_styled` a line in a `TextStyle`.
`Font::measure` and `Font::measure_styled` measure what they draw, and
`Font::truncate` and `truncate_in` shorten text with an ellipsis to fit a
width.

The `draw_…` functions — `draw_button`, `draw_checkbox`, `draw_tab_pane`,
`draw_window_frame`, `draw_list_box` and the rest — draw a control's look
for a control of your own or a test. They make no focus, input or
accessibility node; a control of your own adds those.

The Latin faces and the four regional `cjk12` faces are compiled in. After
`init`, characters no face draws fall back to the system's font.

## Diagrams

The `diagram` module, the crate's default feature `diagram`, draws Mermaid
diagrams in the look. [limu-rs](https://github.com/firede/limu-rs) lays a
diagram out as a scene, and the module draws the scene: a node's box as a
button, a group's as a group box, every other outline, line and arrowhead
a pixel at a time, and text in `Font::Small`. It re-exports `limu` and
`limu_gpui`, so an application need not depend on them itself.

`layout_options(language)` gives limu's layout options for the look: text
measured as `Font::Small` draws it in the language, edges in runs across
and down, and everything snapped to whole pixels. A scene laid out with
other options draws, but not on whole pixels.

```rust
use gpui_platinum::{Language, diagram::{self, limu}};
use std::sync::Arc;

let options = diagram::layout_options(Language::English);
let scene = Arc::new(limu::layout("sequenceDiagram\n  A->>B: Hello", &options)?);
let view = diagram::view("greeting", scene.clone())
    .language(Language::English)
    .on_hover(move |item, _, _| {
        if let Some(i) = item {
            println!("over {}", scene.items[i].role);
        }
    });
```

`view(id, scene)` is an element the size of the scene. `.theme(…)` draws it
in a Platinum theme's colors and `.look(…)` in a look of limu's, such as
`Look::new(diagram::theme(&theme))` with overrides of roles. `.language(…)`
draws its East Asian text in the language it was measured in. `.on_hover`
reports the index of the item under the pointer, or none, and `.on_click`
the item a click was on. An item's tooltip shows in a help balloon, and the
pointer is a hand over an item with a link, which the application follows.
`.icons(…)` and `.images(…)` draw pictures as limu-gpui draws them; without
them, pictures are left out. Assistive technology reads the diagram as an
image named by the scene's title, or `.label(…)`.

`paint(scene, origin, options, window, cx)` paints a scene in an element's
paint phase, and `draw` is the hook it paints each item with, for
`limu_gpui::paint_with` when an application draws some items its own way.

## Examples

| Example | What it is |
|---|---|
| `gallery` | Every control, on five pages |
| `controls` | Every control, live, by the name of its function, with its dialogs |
| `desktop` | The classic desktop: windows, menus, a disk and small applications |
| `diagrams` | Every type of Mermaid diagram limu reads, in the look, with its text to edit |

Run them with `cargo run --example <name>`. `--png <file>` renders a
picture without a window, at `--scale <n>`; the gallery takes `--page <n>`,
the desktop `--lang en`, `zh-Hans`, `zh-Hant`, `ja` or `ko`, and the
diagrams `--type <title>`, `--example <n>`, `--src <file>` and, for the
drawing alone in a window of the look, `--frame <title>`. Pictures need
GPUI's offscreen renderer, which gpui-fast v0.1.12 has on macOS and Linux,
not on Windows. `./scripts/screenshots.sh` renders the pictures
in `docs`.

## From the Go library

mygo-platinum binds values by pointer and reports changes as an
immediate-mode view is built again. This port uses GPUI's entities and
typed events, and Rust's naming; the
[README](../README.md#what-is-in-it) maps the names. It is not a drop-in
match: in particular, the Go library's window options and its catalogs an
application can extend have no Rust equivalent. The design specification
describes the look both share.
