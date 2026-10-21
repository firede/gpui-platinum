# gpui-platinum

English | [简体中文](README.zh.md)

Platinum desktop controls for Rust and [GPUI](https://github.com/longbridge/gpui-fast): gray faces, pixel-cut corners, beveled edges and bitmap type, in the style of late-1990s desktops. Controls draw through GPUI's native renderer. A gallery and a classic desktop example put them to work.

![The GPUI control gallery](docs/gallery@2x.png)

More: [page 2](docs/gallery-2@2x.png) (buttons, fields and rich text), [page 3](docs/gallery-3@2x.png) (collections and layout), [page 4](docs/gallery-4@2x.png) (dates, colors and sheets), [page 5](docs/gallery-5@2x.png) (documents and navigation).

This is the GPUI counterpart of [mygo-platinum](https://github.com/firede/mygo-platinum). It carries over that project's visual language, fonts and original artwork, with Rust APIs built around GPUI elements, entities and events. It is a tribute in style only, with no affiliation with Apple Inc.

The crate is an early 0.x project. Its API may change, and it currently pins the `longbridge/gpui-fast` fork to revision `b836dd9`. Keep the GPUI dependencies of your application on that same revision. This repository's verification and screenshots do not establish support for every platform supported by GPUI.

## What is in it

| Controls | Rust API |
|---|---|
| Push, default and cancel buttons; check boxes; disclosure triangles | `button`, `checkbox`, `disclosure`, `Control`, `CheckState`, `button_scope` |
| Radio groups, tabs and segmented choices | `radio`, `tab`, `ChoiceGroup`, `ChoiceItem` |
| Sliders, scroll bars and progress | `Slider`, `ScrollBar`, `progress_bar`, `indeterminate_progress` |
| Text, password and multiline fields | `TextField` |
| Rich text, styles, sizes and paragraph alignment | `RichTextArea`, `RichText`, `TextStyle`, `SizeMenu` |
| Lists, trees, tables and icon grids | `ListBox`, `TreeView`, `Table`, `TableColumn`, `IconView` |
| Menus, submenus and context menus | `PopupMenu`, `MenuBar`, `Menu`, `MenuItem`, `ContextMenu` |
| Window frames and bottom-edge pop-up windows | `PlatinumWindow`, `WindowState`, `PopupWindow`, `zoom_rects` |
| Calendar, date, time, number and search fields | `CalendarView`, `DatePicker`, `TimeField`, `NumberField`, `SearchField`, `Stepper` |
| Combo boxes, color and font pickers | `Combobox`, `ColorWell`, `FontPicker` |
| Alerts, dialogs, find and replace, page setup | `Alert`, `Dialog`, `FindReplace`, `PageSetup` |
| Open and save dialogs for an application-owned tree | `OpenDialog`, `SaveDialog`, `models::FileTree` |
| Progress dialogs and notifications | `ProgressDialog`, `Banner`, `Toasts` |
| Navigation and document layout | `ColumnView`, `DocumentTabs`, `PathPopup`, `Breadcrumbs`, `Pagination`, `Ruler`, `Split` (pixels or `.percent()`), `ScrollView` |
| Labels, groups, toolbar and status displays | `label`, `paragraph`, `group_box`, `separator`, `Toolbar`, `placard`, `window_header_parts`, `badge`, `tag`, `status_bar` |
| Supporting displays | `Balloon`, `image_well`, `card`, `empty_state`, `chasing_arrows` |

The [API guide](docs/api.md) explains ownership, events, focus, localization and the differences from the Go API. `cargo doc --no-deps --open` opens the reference generated from the current source.

## Using it

Use a current Rust toolchain with support for edition 2024, plus the native build requirements of the pinned GPUI fork. The examples are native applications and need a graphical session. The [GPUI repository](https://github.com/longbridge/gpui-fast) is the source of platform setup instructions.

For an application checked out beside this repository:

```toml
[dependencies]
gpui-platinum = { path = "../gpui-platinum" }
gpui = { git = "https://github.com/longbridge/gpui-fast", rev = "b836dd9" }
gpui_platform = { git = "https://github.com/longbridge/gpui-fast", rev = "b836dd9", features = ["font-kit", "runtime_shaders", "wayland", "x11"] }
```

Call `gpui_platinum::init(cx)` once, before opening the first window. It registers native text editing, font fallback and modal keyboard bindings. Construct stateful controls with `cx.new`, retain their entities and subscribe to their events. Rebuild lightweight controls from application state in `Render::render`:

```rust
use gpui::{prelude::*, *};
use gpui_platinum::{CheckState, Slider, Theme, checkbox};

struct SoundSettings {
    enabled: bool,
    volume: f64,
}

impl Render for SoundSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let on_volume_change = cx.listener(|this, value: &f64, _, cx| {
            this.volume = *value;
            cx.notify();
        });
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(16.))
            .child(
                checkbox(
                    "sounds",
                    "Play alert sounds",
                    if self.enabled { CheckState::On } else { CheckState::Off },
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.enabled = !this.enabled;
                    cx.notify();
                })),
            )
            .child(
                Slider::new("volume", self.volume, 0., 100.)
                    .theme(Theme::default())
                    .label("Volume")
                    .on_change(move |value, window, cx| {
                        on_volume_change(&value, window, cx);
                    }),
            )
    }
}
```

Give lightweight controls stable IDs, keep their values in application state and call `cx.notify()` after a change. Entity controls, such as `TextField` and `Table`, expose typed events instead. Both patterns are described in the API guide.

Six accents ship with the library: Lavender, Graphite, Harbor, Moss, Ember and Orchid. Set `Theme::accent` from `theme::ACCENTS`; pass the theme to controls and use their `set_theme` methods when it changes.

## The classic desktop

![The GPUI desktop example](docs/desktop@2x.png)

The desktop runs inside one native GPUI window. It includes a menu bar, an in-memory disk with icon, list, button and column views, movable and collapsible window frames, Calculator, Jotter, Scribe, Ledger, Puzzle, Alarm Clock and control panels. Scribe uses the library's rich text editor, ruler, find and replace, and open/save dialogs. The disk and document dialogs operate on the example's own data; they are not a replacement for the operating system's file picker.

Controls and the desktop provide English, Simplified Chinese, Traditional Chinese, Japanese and Korean catalogs. `Language` selects the wording and regional East Asian glyphs. Catalogs live in `assets/locales/controls` and `assets/locales/desktop`.

## Running the examples

```sh
cargo run --example gallery
cargo run --example controls
cargo run --example desktop
cargo run --example desktop -- --lang zh-Hans
```

`./scripts/bundle.sh --gallery` builds a macOS app bundle for the gallery. `./scripts/screenshots.sh` regenerates the seven documentation images from the GPUI renderer; pass `--offline` to use cached dependencies.

The gallery presents the controls in pages. Controls is a browsable catalog. Desktop combines them into applications. See [the API guide](docs/api.md#examples) for their roles and the [design specification](docs/spec.md) for the visual contract.

## Verification

```sh
./scripts/verify.sh
```

The script checks formatting, runs all targets' tests and documentation examples, and runs Clippy with warnings denied. Tests cover the text buffer, rich text formatting, dates, formulas, tree operations, selection and keyboard calculations, with selected pixel-drawing tests compared against the reference images in `tests/reference`.

These tests are not a complete automated audit of every interaction, screen reader or operating system. Runtime checks should include Tab and Shift-Tab, menu and dialog focus, text composition, dragging, resizing and background-window states on the target platform. Screenshots in `docs` are captured from this GPUI implementation; reference images in `tests/reference` are appearance fixtures from the original project.

## Fonts and drawing

- Coordinates are logical pixels. Whole display scale factors preserve the bitmap grid; system-font fallback is rendered by the platform text system.
- Latin text uses the bundled Platinum Sans and Platinum Sans Small faces from [platinum-fonts](https://github.com/firede/platinum-fonts). East Asian text uses the bundled `cjk12` regional faces.
- Unsupported bitmap characters use system-font fallback. Native text composition and clipboard handling are provided through GPUI's input interfaces.
- `paint::Pen` records pixel fills and text. Exported `draw_*` functions allow custom controls to share the library's drawing style.
- Stable element IDs preserve focus and element state. Reuse an entity for each stateful control instead of creating it on every render.

## Documentation and site

- [API guide](docs/api.md): application setup, component families, state, events, input and localization.
- [Design specification](docs/spec.md): colors, metrics and expected control behavior.
- [Web page](site/index.html): the library in five languages, with the desktop as GPUI draws it, styled with pictures the library draws (`cargo run --example desktop -- --site <dir>`). `./scripts/site.sh` builds it into `site/dist`; preview it with `python3 -m http.server -d site/dist`. `./scripts/deploy.sh` builds it and deploys it to Cloudflare, as `wrangler.jsonc` says.

## License

MIT; see [LICENSE](LICENSE). Bitmap font data and its notices are in [assets/fonts](assets/fonts). The website's font files retain their [OFL notice](site/fonts/OFL.txt). The original design and artwork come from [mygo-platinum](https://github.com/firede/mygo-platinum); this project uses no Apple font or icon assets.
