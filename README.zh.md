# gpui-platinum

[English](README.md) | 简体中文

[![verify](https://github.com/firede/gpui-platinum/actions/workflows/verify.yml/badge.svg?branch=main)](https://github.com/firede/gpui-platinum/actions/workflows/verify.yml)

一套为 [GPUI](https://github.com/longbridge/gpui-fast) 编写的控件，风格取自
上世纪 90 年代末灰色、带斜面立体感的 "Platinum" 桌面外观。所有控件都通过
GPUI 的渲染器逐像素绘制。仓库里还附带一个"经典桌面"示例，把这些控件实际用了
起来。它是 [mygo-platinum](https://github.com/firede/mygo-platinum) 的 Rust
移植版。可以在 [gpui-platinum.firede.com](https://gpui-platinum.firede.com)
看到由 GPUI 绘制的桌面。

![控件一览](docs/gallery@2x.png)

更多：[第 2 页](docs/gallery-2@2x.png)（斜面按钮、工具栏、输入框、状态类控件、
富文本、弹出窗口）、[第 3 页](docs/gallery-3@2x.png)（列表、表格、布局）、
[第 4 页](docs/gallery-4@2x.png)（日期、颜色、表格计算、标尺）、
[第 5 页](docs/gallery-5@2x.png)（文档与导航）。

> **仅为风格上的致敬。** 本项目与 Apple Inc. 没有任何关联，也未获其认可或
> 支持。项目没有使用 Apple 的任何字体、图标、图片或商标：西文点阵字体、所有
> 图标和图片都是为 mygo-platinum 原创绘制的，东亚文字字形来自开源的缝合像素
> 字体，设计规范（[docs/spec.md](docs/spec.md)）也是从零写成的。

> [!WARNING]
> **这是一个早期的个人项目。** 它为作者自用而写，按原样分享，不承诺提供支持。
> 0.x 版本之间 API 随时可能变化；它依赖的 gpui-fast v0.1.12 和 limu-rs v0.1.0
> 本身也还年轻、仍在变化。请固定版本使用，并做好随之调整的准备。

## 包含哪些控件

| 控件 | 函数 |
|---|---|
| 按钮、默认按钮与取消按钮 | `button` |
| 复选框（支持混合状态） | `checkbox` |
| 单选按钮与单选组 | `radio`、`ChoiceGroup::radio` |
| 滑块（支持刻度） | `Slider` |
| 进度条，确定与不确定两种 | `progress_bar`、`indeterminate_progress` |
| 展开三角 | `disclosure` |
| 分组框与分隔线 | `group_box`、`separator` |
| 标签页控件 | `tab`、`ChoiceGroup::tabs` |
| 弹出菜单 | `PopupMenu` |
| 文本框、密码框、多行文本区 | `TextField` |
| 列表框（支持图标、灰显行） | `ListBox` |
| 滚动条 | `ScrollBar` |
| 窗口框架：条纹标题栏，关闭、缩放与折叠框，缩放角，路径菜单 | `PlatinumWindow` |
| 菜单栏、菜单、子菜单、快捷键 | `MenuBar`、`Menu`、`MenuItem` |
| 警告框、可移动的模态对话框 | `Alert`、`Dialog` |
| 用点阵字体绘制的文字 | `label`、`text_in`、`paragraph` |
| 斜面按钮、图标按钮、开关、分段控件 | `bevel_button`、`icon_button`、`ChoiceGroup::segments` |
| 带溢出菜单的工具栏 | `Toolbar` |
| 小箭头、数字框、搜索框 | `Stepper`、`NumberField`、`SearchField` |
| 旋转箭头、标牌、窗口头部、徽标、标签、图像槽 | `chasing_arrows`、`placard`、`window_header`、`badge`、`tag`、`image_well` |
| 帮助气球、右键菜单 | `Balloon`、`ContextMenu` |
| 树视图、可排序表格、图标视图 | `TreeView`、`Table`、`IconView` |
| 分栏、滚动视图、工具窗口 | `Split`、`ScrollView`、带 `utility` 的 `PlatinumWindow` |
| 日历、日期选择器、时间输入框 | `CalendarView`、`DatePicker`、`TimeField` |
| 组合框、颜色槽与取色器 | `Combobox`、`ColorWell` |
| 带公式的电子表格、公式栏 | `SheetView` |
| 查找替换对话框、标尺 | `FindReplace`、`find_next`、`replace_all`、`Ruler` |
| 分栏视图、文档标签页、字体选择器、页面设置 | `ColumnView`、`DocumentTabs`、`FontPicker`、`PageSetup` |
| 进度对话框、横幅、提示消息（toast）、空状态 | `ProgressDialog`、`Banner`、`Toasts`、`empty_state` |
| 路径弹出菜单、面包屑、分页、卡片、状态栏 | `PathPopup`、`Breadcrumbs`、`Pagination`、`card`、`status_bar` |
| 富文本：样式、四种字号、段落对齐；格式栏 | `RichTextArea`、`SizeMenu` |
| 停靠在底边的弹出窗口 | `PopupWindow` |
| 打开与保存对话框 | `OpenDialog`、`SaveDialog` |
| 窗口打开时的缩放矩形动画 | `zoom_rects` |
| Mermaid 图表，由 [limu](https://limu.firede.com) 排版 | `diagram::view` |

每个控件都具备完整的状态（按下、禁用、聚焦、默认、选中、选定，以及位于后台
窗口时的非活动状态），可以用键盘操作，并通过 GPUI 的无障碍支持向屏幕阅读器
描述自己。有独特外观的控件还导出了对应的 `draw_…` 函数，方便你画出外观一致的
自定义控件。

`diagram` 模块以这套外观绘制 Mermaid 图表。limu-rs 用库里的点阵字体测量文字，
把图表排在整像素上；模块像绘制控件那样绘制其中的框，逐像素绘制线条。它会报告
指针下的元素和被点击的元素，并在帮助气球里显示元素的提示文字。

```rust
use gpui_platinum::{Language, diagram::{self, limu}};

let options = diagram::layout_options(Language::English);
let scene = limu::layout("flowchart LR; a --> b", &options)?;
diagram::view("flow", scene).on_click(|item, _, _| println!("clicked item {item}"))
```

它是 crate 的默认 feature `diagram`。只需要控件时，用 `default-features = false`
去掉它。

控件名沿用 mygo-platinum，按 Rust 的命名习惯书写。两者不同之处：

| mygo-platinum | 本库 |
|---|---|
| `Button`、`Checkbox`、`Disclosure` | `button`、`checkbox`、`disclosure` |
| `RadioGroup`、`TabControl`、`Segmented` | `ChoiceGroup::radio`、`::tabs`、`::segments` |
| `TextArea` | 带 `.multiline(true)` 的 `TextField` |
| `SplitPercent`、`SplitVertical` | 带 `.percent()`、`.vertical()` 的 `Split` |
| `Calendar`、`Sheet` | `CalendarView`、`SheetView` |
| `FormulaBar`、`FormatBar` | 内置于 `SheetView` 与 `RichTextArea` |
| `Window` | `PlatinumWindow` |
| `Find`、`ReplaceAll` | `find_next`、`replace_all` |
| `SpringOpen`、`ZoomRects` | `PopupWindow::spring_open`、`zoom_rects` |
| `Card`、`EmptyState` | `card`、`empty_state` |
| `diagram.Layout`、`diagram.View` | `diagram::layout_options`、`diagram::view` |

## 使用

需要较新的稳定版 Rust（gpui-fast 使用 1.98 构建），以及 crates.io 上的
gpui-fast v0.1.12，与 limu-gpui 相同。应用自己的 GPUI 依赖也必须是这个 crate：
不同 GPUI 的类型（例如来自 gpui-fast 仓库的那份）不能混用。

```toml
[dependencies]
gpui-platinum = { git = "https://github.com/firede/gpui-platinum", tag = "v0.2.0" }
gpui = { package = "gpui-fast", version = "0.1.12" }
gpui_platform = { package = "gpui-fast-platform", version = "0.1.12", features = ["font-kit", "runtime_shaders", "wayland", "x11"] }
```

```rust
use gpui::{prelude::*, *};
use gpui_platinum::{CheckState, MenuEvent, PopupMenu, Slider, button, button_scope, checkbox};

struct Settings {
    sound: bool,
    volume: f64,
    sort: Entity<PopupMenu>,
    sort_by: usize,
    _sort_changed: Subscription,
}

impl Settings {
    fn new(cx: &mut Context<Self>) -> Self {
        let sort = cx.new(|cx| PopupMenu::new(["Name", "Date", "Size"], cx).caption("Sort by:"));
        let _sort_changed = cx.subscribe(&sort, |this, _, MenuEvent(index), cx| {
            this.sort_by = *index;
            cx.notify();
        });
        Self { sound: true, volume: 50., sort, sort_by: 0, _sort_changed }
    }

    fn apply(&mut self) { /* … */ }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let set_volume = cx.listener(|this, volume: &f64, _, cx| {
            this.volume = *volume;
            cx.notify();
        });
        let sound = if self.sound { CheckState::On } else { CheckState::Off };
        button_scope(
            "settings",
            div()
                .flex()
                .flex_col()
                .p(px(12.))
                .gap(px(8.))
                .child(checkbox("sound", "Play alert sounds", sound).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.sound = !this.sound;
                        cx.notify();
                    },
                )))
                .child(
                    Slider::new("volume", self.volume, 0., 100.)
                        .label("Volume")
                        .on_change(move |volume, window, cx| set_volume(&volume, window, cx)),
                )
                .child(self.sort.clone())
                .child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .child(button("cancel", "Cancel").cancel_button())
                        .child(button("ok", "OK").default_button().on_click(cx.listener(
                            |this, _, _, _| this.apply(),
                        ))),
                ),
        )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        gpui_platinum::init(cx); // 在打开第一个窗口之前调用一次
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(Settings::new))
            .expect("open a window");
        cx.activate(true);
    });
}
```

控件分两类。轻量控件（`button`、`checkbox`、`Slider`、`ChoiceGroup` 以及
展示类控件）是每次渲染时根据应用状态重新构建的元素：给每个控件一个稳定的
ID，在回调里更新状态并调用 `cx.notify()`。自带状态的控件（`TextField`、
`PopupMenu`、`ListBox`、`Table`、各种对话框）是实体：用 `cx.new` 创建一次，
保留句柄，并订阅它们的类型化事件。`button_scope` 让 Return 和 Escape 落到其中
的默认按钮和取消按钮上。[API 指南](docs/api.md)（英文）介绍这两类控件、焦点、
语言与绘制；`cargo doc --no-deps --open` 打开 API 参考。

内置六种强调色：Lavender、Graphite、Harbor、Moss、Ember 和 Orchid，见
`theme::ACCENTS`，通过 `Theme::accent` 设置。

## 经典桌面

![桌面示例](docs/desktop-zh-hans@2x.png)

`examples/desktop.rs` 在一个窗口里实现了一整套桌面：

- 带时钟和应用菜单的菜单栏；
- 可以打开、也可以拖进废纸篓的桌面图标；
- 可以移动、缩放、卷起、最大化，以及停靠到底边成为弹出窗口的窗口；
- 一个浏览自带小磁盘的文件浏览器：支持图标、按钮、列表、分栏四种视图，以及
  弹簧文件夹、按住 Option 拖动复制、显示简介、复制、放回原处、标题栏路径菜单，
  还有"查找文件"；
- 用这些控件搭出来的应用：计算器、便笺本、写字板（富文本编辑器，带样式、字号、
  标尺、查找替换、页面设置和打开/存储对话框）、账簿（电子表格）、拼图、闹钟、
  控件（按函数名列出库里的每个控件，都可以直接操作），以及外观、日期与时间、
  语言、声音、鼠标五个控制面板。

磁盘只存在于内存中，打开/存储对话框浏览的是它，而不是电脑上的文件。

桌面支持英文、简体中文、繁体中文、日文和韩文，跟随系统语言，也可以在"语言"
控制面板中选择。文字、日期与时间，以及东亚文字的地区字形都会随之切换。消息
目录在 `assets/locales/desktop`。

桌面里所有的名称和图片都是原创的。网站展示的是 GPUI 绘制的桌面图片；桌面本身
是原生应用，不在浏览器中运行。

## 运行示例

```sh
cargo run --example gallery                    # 分五页展示所有控件
cargo run --example gallery -- --png gallery.png --page 2   # 不开窗口，把第 2 页渲染成图片
cargo run --example controls                   # 按函数名列出所有控件
cargo run --example desktop                    # 经典桌面
cargo run --example desktop -- --png desk.png --lang ja     # 不开窗口渲染成图片，日文界面
cargo run --example diagrams                   # 以这套外观展示每种图表
cargo run --example diagrams -- --png sequence.png --type Sequence --example 2
cargo run --example diagrams -- --src flow.mmd --png flow.png --frame Flow  # 只渲染图表，放在一个窗口里
```

`--scale 2` 以两倍尺寸渲染图片。图片由 GPUI 的离屏渲染器绘制，gpui-fast v0.1.12
在 macOS 和 Linux 上提供它，Windows 上没有。`./scripts/screenshots.sh` 生成
`docs` 里的配图；`./scripts/bundle.sh --gallery`（或 `--desktop`）把示例打包成
macOS 应用。

`./scripts/site.sh` 把网页连同桌面绘制的图片（`desktop --site <dir>`）构建到
`site/dist`；`./scripts/deploy.sh` 按正式网址构建，并按 `wrangler.jsonc` 的
配置部署到 Cloudflare。在 GitHub 上发布 Release 时也会自动部署。网页文字改动后，
运行 `./scripts/site-fonts.sh` 按新的文字重新裁剪网页字体。

## 测试

```sh
./scripts/verify.sh
```

这个脚本依次运行 `cargo fmt --check`、所有目标的全部测试、文档测试，以及禁止
警告的 Clippy；GitHub 会在 macOS、Windows 和 Linux 上运行它。额外的参数会
传给 Cargo，例如 `--offline`。测试包括：

- 每个控件、gallery 的各页、桌面和每种图表的场景，由 GPUI 的离屏渲染器绘制，与
  `tests/reference/snapshots` 中 mygo-platinum 的 golden 图逐像素比对。它们在
  由 Metal 绘制的 macOS 上运行，其他平台上跳过：

  ```sh
  cargo test --test snapshots             # 所有场景
  cargo test --test snapshots -- tabs     # 名称含 "tabs" 的场景
  ```

  不一致的场景会把绘制结果和用红色标出差异像素的图留在 `target/snapshots`；
  标记为 pending 的场景只报告差异，不算失败；
- 控件的绘制与 `tests/reference` 中 mygo-platinum 的图片比对；
- 控件的键盘操作、菜单与取值范围，以及随时间的行为（闪烁、光标闪动、重复、
  延时），使用测试执行器的时钟；
- `Font::measure_styled` 与 `Pen::text_styled` 的绘制结果是否一致；
- diagrams 示例的每个例子都能排版和绘制，以及图表视图报告的指针位置；
- 文本缓冲、富文本、日期、公式、树、图表像素和桌面各应用的单元测试。

有意修改了控件外观后，运行 `./scripts/screenshots.sh`，并查看 `docs` 里的
新图。

## 说明

- 像素即 GPUI 的逻辑像素：控件在任意整数倍缩放的显示器上都很清晰，在 2 倍屏上
  以 2×2 的像素块绘制。
- 文字使用 [platinum-fonts](https://github.com/firede/platinum-fonts) 的点阵
  字体绘制，打包在 `assets/fonts`：Platinum Sans Bold（`Font::Bold`）与
  Platinum Sans Small（`Font::Small`），另有 `Font::Sans` 和
  `Font::SmallBold`。它们覆盖 ASCII、常用标点、Latin-1 的全部字母（带重音的
  字母，以及 Æ、Ð、Ø、Þ、Œ、ß）和最常用的中欧字母（Ł、Ś、Č、Ő、Ą…）。
- 东亚文字使用 platinum-fonts 中 `cjk12` 的 12px 字体绘制，打包了四种地区
  字形，由控件的 `.language(…)` 选择。没有字体绘制的字符（希腊文、其他带重音
  的字母）在调用 `init` 之后会用系统字体在同一基线上绘制，因此任何文字都可以
  输入和显示。
- 控件自身的文字（按钮、对话框、标签和屏幕阅读器读出的名称）提供英文、简体
  中文、繁体中文、日文和韩文，来自 `assets/locales/controls`，跟随每个控件的
  `Language`。日历、日期选择器和时间输入框按语言的习惯书写日期；
  `first_weekday`、`day_first` 和 `hour_cycle` 用来按用户的区域格式设置。
- 文本行会为东亚字形留出空间，与 mygo-platinum 设置了东亚字体后的行为一致。
  `text_input::set_east_asian_line_room(false)` 让英文文本行只按西文字体排列。
- 图表中旋转 90° 整数倍的标签逐像素旋转，与 mygo-platinum 一致；旋转其他角度的
  标签逐像素取其来源像素，而 mygo-platinum 会对它重新采样。limu-rs 把图表网格和
  坐标轴的每个点都对齐到像素上，Go 版 limu 则整体平移，所以甘特图的网格和 XY
  图的线条与 mygo-platinum 相差一个像素。

## 许可

MIT，详见 [LICENSE](LICENSE)。字体来自
[platinum-fonts](https://github.com/firede/platinum-fonts)，其字形数据采用
SIL Open Font License 1.1，见 [assets/fonts](assets/fonts)。
