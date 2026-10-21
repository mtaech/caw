# gpui-kit 0.7 API notes (verified against vendored sources)

This file records **verified** API facts for the exact versions Caw resolves to. It exists so
the team does not invent APIs. When something here is missing or looks wrong, read the vendored
source instead of guessing:

```text
~/.cargo/registry/src/mirrors.tuna.tsinghua.edu.cn-4dc01642fd091eda/
├── gpui-kit-0.7.0/src/lib.rs          # facade: pub use ::gpui::*; base/component/assets/platform
├── gpui-base-0.7.0/src/               # unstyled behavior: styled.rs, root.rs, slider.rs, scroll...
├── gpui-component-0.7.0/src/          # styled components (button/, input/, sidebar/, theme/, ...)
└── gpui-pre-0.3.7/src/                # GPUI itself (div, Window, Context, App, executors)
```

Docs: https://gpui-kit.com/llms.txt and https://gpui-kit.com/component/{name}.md.

## Imports: the root is GPUI, layers are named

`gpui_kit::*` re-exports **GPUI only** (`pub use ::gpui::*`). Layout helpers and the styled
extensions are **not** in the root; they live in `gpui_kit::base`:

```rust
use gpui_kit::*;                                   // div, px, Entity, Context, Window, img, ...
use gpui_kit::base::{h_flex, v_flex, StyledExt};   // free functions + .h_flex()/.v_flex() + font_*
use gpui_kit::FluentBuilder as _;                  // .when(), .when_some(), .when_none()
use gpui_kit::StatefulInteractiveElement as _;     // .overflow_y_scroll(), .track_scroll(), .id()
use gpui_kit::component::{
    ActiveTheme, Sizable, Root, TitleBar, Icon, IconName, WindowExt,
};
```

- font_bold / font_semibold / font_medium come from `gpui_kit::base::StyledExt`, not from GPUI.
- `StatefulInteractiveElement` is needed for `.overflow_y_scroll()` / `.overflow_x_scroll()`
  on `Div` (they are not on `Styled`). Scrollbars also need
  `gpui_kit::component::scroll::ScrollableElement`.
- `cx.theme()` requires `ActiveTheme` in scope.
- Use `gpui_kit::FluentBuilder as _` for `.when_some(...)`; the old when_some error was a
  missing trait import.

## Theme

```rust
use gpui_kit::component::ActiveTheme;
let theme = cx.theme();          // &Theme; Deref<Target = ThemeColor>
theme.background                  // tokens live on ThemeColor through Deref
```

`ThemeColor` fields (0.7): accent, accent_foreground, background, border, button*, caret,
danger*, foreground, info*, input, link*, list*, muted, muted_foreground, popover,
popover_foreground, primary*, ring, scrollbar*, secondary*, selection, sidebar*,
skeleton, slider*, success*, switch*, tab*, table*, title_bar*, status_bar*, warning*,
overlay, window_border, red/green/blue/yellow/magenta/cyan (+ *_light).

**There is no `card` token.** For a card/dialog surface use `popover` (or `background` + border).
Use `secondary` for subtle section backgrounds (the existing views do).

Theme control (verified `gpui-component/src/theme/mod.rs`):

```rust
use gpui_kit::component::Theme;
Theme::change(ThemeMode::Dark, None, cx);                 // Light | Dark (no System variant)
Theme::global_mut(cx).font_family = "Noto Sans CJK SC".into();
Theme::global_mut(cx).mono_font_family = "DejaVu Sans Mono".into();
Theme::update(cx, |theme, _window| { /* edit tokens */ });
```

- `ThemeMode` is only Light / Dark; "follow system" must be implemented in app code by reading
  the OS appearance and resolving to one of the two.
- Global app font belongs on `Theme.font_family`, not on each `div()`.
- `Theme::change` signature: `(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App)`.

## Layout, scrolling, text

```rust
h_flex().gap_2().items_center().child(a).child(b)     // row, cross-axis centered
v_flex().gap_3().w_full().child(...)                   // column, stretch
div().flex_1().size_full().overflow_y_scroll()         // REQUIRES StatefulInteractiveElement
div().text_ellipsis()                                  // gpui Styled has text_ellipsis()
div().truncate()                                       // gpui Styled
```

- `h_flex()` centers on the cross axis; `v_flex()` does not stretch. Give full-height children
  `h_full()` or set `items_start` when a header/footer/scroll region must resolve against the
  parent height.
- Raw hex/rgb is discouraged; use theme tokens. If a literal is unavoidable, use
  `gpui::hsla(...)` / `rgba()`, never a string hex.

## Components (verified signatures)

**Sidebar** (`gpui_kit::component::sidebar::*`):

```rust
Sidebar::new("id").w(px(220.)).header(SidebarHeader::new().child(..)).footer(..)
    .child(SidebarGroup::new("Label").child(SidebarMenu::new().child(item)))
SidebarMenuItem::new("label").icon(CawIcon::Music).active(bool).on_click(|_, _, cx| {})
    .suffix(|window, cx| Button::new("add").ghost().xsmall().icon(CawIcon::Plus).into_any_element())
```

- `SidebarGroup` has **no** `.header()` in 0.7, only `new(label)`, `child`, `children`, `collapsed`.
  To put a "+" next to a group, either use `SidebarMenuItem::suffix` on an item, or compose a
  custom `v_flex` section instead of `SidebarGroup`.
- `SidebarMenuItem::child(...)` exists for nested items; `suffix` takes
  `Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>`, so return `.into_any_element()`.
- `SidebarMenuItem` must be a child of `SidebarMenu`/`SidebarGroup`; do not wrap it in
  `div().child(item)` (that produced the `From<Div> for SidebarMenuItem` error).

**Button** (`gpui_kit::component::button`):

```rust
Button::new("id").label("文本").icon(CawIcon::Play).on_click(|_, window, cx| {})
    .primary() / .secondary() / .danger() / .warning() / .success() / .info()
    .ghost() / .outline() / .link() / .text()
    .xsmall() / .small() / .medium() / .large()
    .toggled(bool)           // selected state for on/off buttons (there is no .selected())
```

- Import `ButtonVariants` and `Sizable` traits for the builder methods.

**Input** (`gpui_kit::component::input`):

```rust
let state: Entity<InputState> = cx.new(|cx| InputState::new(window, cx)
    .placeholder("搜索...").clean_on_escape());
Input::new(&state).cleanable(true)
// read text:  state.read(cx).value(cx) -> SharedString   (NOT .text())
// focus:      state.read(cx).focus_handle(cx)
// events:     InputEvent::Change (subscribe via cx.subscribe)
```

**Slider** (`gpui_kit::component::slider`, state re-exported from gpui-base):

```rust
let s = cx.new(|_| SliderState::new().min(0.0).max(100.0).step(0.1).default_value(0.0));
Slider::new(&s)
// events SliderEvent::Change(SliderValue) / Release(SliderValue), .start()
// programmatic: s.update(cx, |s, cx| s.set_value(v, window, cx));   // 3 args
```

**Root / window / overlays**:

```rust
Root::new(view_entity, window, cx)                       // view: impl Into<AnyView>
TitleBar::window_options()                               // WindowOptions for custom title bar
window.open_dialog(cx, |dialog, window, cx| dialog.title("...").child(..));
window.open_alert_dialog(cx, |alert, _, _| alert.warning().title("删除？").description("..."));
window.push_notification(notification, cx);              // toasts
```

- `WindowExt` is implemented for `Window`; import the trait.
- Dialogs/sheets/notifications are rendered by `Root`; keep the app root wrapped in `Root`.

**Icons**:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum CawIcon { Music, Play, /* ... */ }
impl IconNamed for CawIcon { fn path(self) -> SharedString { "icons/music.svg".into() } }
Icon::new(CawIcon::Music).size_4().text_color(cx.theme().primary)
```

- `IconNamed` comes from `gpui_kit::component`; `Sizable` provides size_*.
- Custom SVG paths are served by `AssetSource` (`src/assets.rs`), so `main.rs` must call
  `.with_assets(CawAssets)` on the application builder.
- `IconName` variants are limited (Lucide set). The old `IconName::Music` does not exist; use a
  custom `CawIcon` for anything not in `IconName`.

## Async, timers, executors

```rust
// Periodic tick (position polling): gpui executor timer, not smol's Timer iterator
cx.spawn(async move |entity, cx| loop {
    cx.background_executor().timer(Duration::from_millis(250)).await;
    entity.update(cx, |this, cx| this.update_position(cx)).ok();
}).detach();

// CPU/IO work off the UI thread
let result = cx.background_executor().spawn(async move { scan_directories(&dirs) }).await;

// Clone everything a closure needs before spawning (PathBuf, Arc<Track>, Entity handles);
// moving a value into background_executor().spawn and then using it in entity.update
// is a borrow-after-move error (current app.rs bug).
```

- `cx.spawn` closure receives a WeakEntity<T> (or AsyncApp in App context); `entity.update`
  returns Result<_>, so use `let _ = ...` or `.ok()`.
- `smol::Timer::interval(...).next()` needs StreamExt; avoid it in UI code.

## Render functions: borrow model

Render functions take `app: &PlayerApp, window: &mut Window, cx: &mut Context<PlayerApp>`.
Inside:

- read state through the `&PlayerApp` argument; never `self.app.read(cx)` while also needing `cx`.
- clone `cx.entity()` into handlers and call `entity.update(cx, |app, cx| app.method(cx))`.
- Do **not** pass `&mut cx.to_async()` to functions expecting `&mut Context<PlayerApp>`
  (current main.rs error: expected &mut Context<PlayerApp>, found &mut AsyncApp).
- Root view: `impl Render for PlayerApp` in `src/root_view.rs`; `self` is `&mut PlayerApp`,
  so `let app = &*self;` then pass `app` and `cx`.

## Common failure modes seen in the current tree

| Error | Real cause | Fix |
|-------|------------|-----|
| cannot find function h_flex | not in gpui_kit::* | use gpui_kit::base::h_flex |
| no method font_bold | StyledExt not imported | use gpui_kit::base::StyledExt |
| no method overflow_y_scroll | trait not imported | use gpui_kit::StatefulInteractiveElement as _ |
| no method when_some | FluentBuilder not imported | use gpui_kit::FluentBuilder as _ |
| no field card on Theme | no card token | use popover or secondary |
| SidebarGroup::header missing | API removed | SidebarMenuItem::suffix / custom section |
| From<Div> for SidebarMenuItem | wrapped item in div | put item directly under SidebarMenu |
| Button::selected missing | no such method | .toggled(bool) or Toggle |
| AsyncApp vs Context<PlayerApp> | cx.to_async() misuse | pass cx directly; root view owns Render |
| borrow of moved value: path | clone before spawn | clone path before background spawn |
| smol::Timer ... next | missing StreamExt | cx.background_executor().timer(..).await |
| IconName::Music missing | not in Lucide set | custom CawIcon |

## Rules

1. Never invent a method. Grep the vendored source for `pub fn` before using it.
2. Read the component doc page for a non-trivial component (`.../component/{name}.md`).
3. Do not reach for raw colors, absolute layouts, or manual overlays when a token/component exists.
4. All app state lives in `PlayerApp` (Entity); UI state (inputs/sliders) lives in `Entity<...State>`.
5. Keep unrelated files compiling; only touch files in your write scope.

## Corrections / discoveries (2026-10-21, foundation pass)

- `IconName` for components: import it explicitly: `use gpui_kit::component::{IconName, IconNamed};`
  It is the compatibility enum generated from the default Lucide list (Play, Pause, Plus, Delete,
  Search, Settings, Folder, ArrowLeft, Close, User, RefreshCw, Loader, ...). There is no Music/Disc/
  ListMusic/Shuffle/Repeat/Skip/Volume variant -- use custom `CawIcon` paths for those.
- `WindowBounds::centered(size, cx)` takes 2 args in this GPUI version (there is also a
  `centered(display_id, size, cx)`-style constructor elsewhere; the one used by WindowOptions
  accepts `size` + `cx`).
- `Entity::update` returns `R` (for `Entity<T>`), but `WeakEntity::update` returns `Result<R>`.
  For a timer loop that must stop when the window closes, use `let weak = view.downgrade();`
  and `if weak.update(cx, ...).is_err() { break; }`.
- `Context::theme()` does not exist. `ActiveTheme` is implemented for `App`/`Window`; inside
  `cx.new(|cx| ...)` capture the color before the closure: `let bg = cx.theme().background;`.
- `InputState::focus_handle(cx)` is reached through the `Focusable` trait and takes `&App`:
  `state.read(cx).focus_handle(cx).is_focused(window)`. `state.read(cx).value()` takes no `cx`.
- `Entity::update` + `Context` borrow rules: in `Render for PlayerApp`, `self` is `&mut PlayerApp`;
  use `let app: &PlayerApp = self;` for the read-only part and `cx` for handlers.
- `Root::new(...)` lives in `gpui_kit::component::Root` (re-export of `gpui_base::Root`), and
  `.bg(...)` works through `Styled`.

## Roadmap reminder for P2

- AppConfig persistence lives in src/config.rs (core-audio). `main.rs` calls
  `config::AppConfig::load().apply_theme(cx)` after `gpui_kit::init(cx)`.
- Focus handle: `PlayerApp { pub focus_handle: FocusHandle }` created with `cx.focus_handle()`
  and exposed through `impl Focusable for PlayerApp`.

## Corrections from content-views P1 (verified)

- `FluentBuilder` is at `gpui_kit::prelude::FluentBuilder`, not the crate root.
  `gpui_kit::*` does include `StatefulInteractiveElement`.
- `.overflow_y_scroll()` / `.overflow_x_scroll()` / `.track_scroll()` need `.id(...)` first: the
  trait is implemented for `Stateful<E>` (and `Img`), not for a bare `Div`. Pattern:
  `div().id("content-scroll").flex_1().size_full().overflow_y_scroll()`.
- There is no `font_mono()` in 0.7. Use `.font_family(cx.theme().mono_font_family.clone())`.
- `find_music_dirs` lives in `crate::audio::library` (not re-exported through app/ui).
- `img(path)`: pass a `PathBuf` for filesystem covers (`img(PathBuf)` -> Resource::Path). Passing a
  `String`/`&str` makes GPUI look it up in the AssetSource, so `~/.config/caw/covers/*.jpg` would
  silently fail to load. Use PathBuf for all runtime cover paths.
- `InputState::value()` takes no `cx`; `InputState::focus_handle(cx)` comes from `Focusable`.
