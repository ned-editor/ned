Welcome to the mound! **Term**it **i**mplements **T**UI - yet another terminal UI.

**Tl;DR:**

```rust no_run
//! This is a basic hello world termit example.
//!
//! Just add a loop and some input handling...
use std::io;
use termit::prelude::*;
fn main() -> io::Result<()> {
    futures_lite::future::block_on(run())
}

async fn run() -> io::Result<()> {
    // The Termit facilitator:
    // * Use the Terminal to set up your terminal experience.
    // * Initialize Termit.
    let mut termit = Terminal::try_system_default()?.into_termit::<NoAppEvent>();

    // The TUI tree:
    // * Canvas sets the default style for the contained widget and fills space.
    // * "Hello World!" - &str/String - is a widget.
    // * We just place the text in the middle with some placement constraints.
    let mut ui = Canvas::new(
        "Hello World!"
            .width(6)
            .height(2)
    ).back(Color::blue(false));

    // Render and update and print... rinse and repeat?
    termit.step(&mut (), &mut ui).await?;
    //                 \      |
    //   the mutable    |     |
    //   application    |     |
    //   model goes here      \- The UI to show off

    Ok(())
}
```

You can also use termit in bare form, just to render some once of content

```rust no_run
//! This is a minimal hello world termit example.
//!
//! There is no input processing, no async, no looping. Just print some nice screen.
use std::io;
use termit::prelude::*;
fn main() -> io::Result<()> {
    // The Termit facilitator:
    // * Use the Terminal to set up your terminal experience.
    // * Initialize Termit.
    let mut termit = Terminal::try_system_default()?.into_termit::<NoAppEvent>();

    // The TUI tree:
    // * Canvas sets the default style for the contained widget and fills space.
    // * "Hello World!" - &str/String - is a widget.
    // * We just place the text in the middle with some placement constraints.
    let mut ui = Canvas::new(
        "Hello World!"
            .width(6)
            .height(2)
    ).back(Color::blue(false));

    // render and update:
    termit.update(&mut (), Event::Refresh(0), &mut ui);
    //                  |   |                      |
    //   the mutable    |   |                       \- The UI to show off
    //   application    |   |
    // model goes here -/   |
    //                       \- this is an event,
    //                           here a refresh
    //
    // print       ,- the reduced scope (rectangular window)
    //            |     None => all of the screen
    termit.print(None)
}
```

[![asciicast](https://asciinema.org/a/AQ2WCTk9otfIagUh6gJ39MUGK.svg)](https://asciinema.org/a/AQ2WCTk9otfIagUh6gJ39MUGK)

For a little more sophisticated example with input event loop and custom widgets, [`cargo run --example color`](examples/color.rs). A simplistic [Delta.Chat](https://delta.chat) client inspired by [the dreamer project](https://github.com/dignifiedquire/dreamer) is called [dech](https://gitlab.com/BrightOpen/dech). There are more [examples](examples/).

Here's a replay of the [editor example](examples/editor.rs):

[![asciicast](https://asciinema.org/a/0wMK1pyqEp8I9uSoSS2snLvu4.svg)](https://asciinema.org/a/0wMK1pyqEp8I9uSoSS2snLvu4)

# Why?

Back to basics, less line drawing, more content and elegance. Async?

# Usage

In your `Cargo.toml`:

```toml
[dependencies]
termit = "*"
```

Please note that the API is still evolving. Feedback is most welcome. Ergonomic static UI initialization and dynamic UI updates are somewhat at odds with each other.

# What's in the box?

On the system side:

- [x] Asynchronous input parsing into a neat input model stolen from crossterm.
- [x] Synchronous output with escaping and style.
- [x] TTY terminal ioctl / Windows console WinRT set up and configuration.
- [x] Plug in your own IO and control.
- [x] Explicit use of IOs = there is no single system-wide tty/console or global settings.
- [x] Internal screen buffer trying hard to reduce the amount of IO.
- [x] Separation of concerns, there is the UI, there is the model and there is the app.

On the UI side:

- [x] A composable widget ecosystem encouraging you to make your own.
- [x] Focus on making a fancy UI, forget the terminal and IO and OS.
- [x] Colors! Do not worry about which color suits which terminal,
  we're compatible. Express it in RGB if you will and we take
  care that it shows on 8 color terminals, too,
  well in 8 colors approximated.
- [x] Styles! With a convenient API to make things `Pretty`, literaly.
- [ ] Styles on windows

On the app side:

- [x] `Termit` helps you to juggle app loop, input, printing...
- [x] It is opinionated, but you can do some things your way, like handling input, printing.
- [x] Async first - except for output which complicates things a lot (how do you run an async terminal cleanup from `drop()`?).
- [x] Usable without async.
- [x] Reduce dependencies by disabling default features: `--no-default-features`
  still renders a UI, dropping windows-sys, rustix and the input parser.
  We try hard to degrade gracefully.
- [ ] Integration with other TUI libs through `TerminalEmulator`: let them print
  their ANSI-fu to temu and show it like a termit widget. Incomplete (resizing
  in particular) and behind the opt-in `temu` feature. If you want it finished,
  [say so](https://gitlab.com/BrightOpen/BackYard/termit/-/issues).
- [x] Reusable components - take just the input parser, screen buffer or terminal initialization...

On the curiosity side:

- [x] The screen buffer lives in an anonymous memory mapped area avoiding frequent allocation. `MmapVec` is as big as it's contents.
- [x] We've got some Windows Console API calls neatly wrapped in rust style. It is calling new `windows-sys` crate and WinRT.
- [x] We use `rustix` for all *nix ioctl things. You may be able to compile without `libc`, not tested.
- [x] Some support for terminal emulation - we can run other terminal apps and display them in a window.
- [x] Constant time refresh for FPS sensitive apps - games.
- [ ] Virtual terminal emulation is still incomplete.

Development

- [x] Docs and examples
- [x] Test coverage
- [x] Automated building and testing

And the real world applications:

- [x] A terminal delta.chat client - `dech` - still in the making, but already chatting...

Non-goals:

- Full coverage of tty ioctl / windows console features. We'll implement as much as is necessary to render UI and handle input.
- Big library of ready made widgets - please make your own crate and ping me for linking.

# Concepts

Terminal access is termit's own cross platform implementation. There is no
backend abstraction and no backend dependency.

Termit takes care of the most common tasks around running a terminal app and building a TUI - a text based user interface.
It could be your one stop shop for all things 'terminal' if you're not looking for a comprehensive coverage of all Terminal / Console features.

## Async

Async to blend in with the rest of the efficient async app. Termit is useful without async as well.
For optimal fit, you'd design your backend as an event stream that consumes commands.
Simple apps can be designed with a simple static state and in-loop processing, though.

## Widgets maintain their own state

Unlike in some other TUI libs, widgets are not ephemeral unless you want to. You can implement a stateful `Widget` easily:

```rust
use termit::prelude::*;
struct UpdateCounter {
    state: usize
}
impl<M, A: AppEvent> Widget<M, A> for UpdateCounter {
    fn update(
        &mut self,
        _model: &mut M,
        _input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        self.state +=1;
        painter.paint(&format!("{}", self.state), screen, 0, false)
    }
}
```

This is usefult if your widget tracks multiple internal state values which are irrelevant to the application as a whole. For instance, a text box editor doesn't need to polute application state with cursor position.

You may also recreate the whole or part of the UI tree in each update cycle if you want to. Then your widgets must keep their state in the model.

## Widgets update the app model directly

Widgets can manipulate the application state model directly.

However, these changes should be near instant. More intensive work (io, network) should be sent to the application as a command and processed independently and ideally asynchronously. This is done in the dech example.

```rust
use termit::prelude::*;
struct AppState {
    state: usize
}
struct UpdateCounter;
impl Widget<AppState, ()> for UpdateCounter {
    fn update(
        &mut self,
        model: &mut AppState,
        _input: &Event<()>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        model.state +=1;
        painter.paint(&format!("{}", model.state), screen, 0, false)
    }
}
```

The widget generally accesses the whole app state mutably. It should be possible, though, to create a widget adapter that will focus on a subset of the model or another model alltogether.

# Simple widgets, composition and decoration

Avoid creating complex widgets. Instead, add features by wrapping other widgets in decorators or by composing a complex subtree from other widgets.

Look at the [`crate::widget::WidgetBinder`] for instance which allows us to load and save any widget, any property of the widget or even replcace it altogether on update. It is a decorator implemented for all `impl Widget`'s with `.bind_with()`:

```rust
# use termit::prelude::*;
# fn sample () {
struct AppState {
    banner: String
}
let mut bound_widget = Canvas::new(String::new()).back(Color::blue(false))
        .bind_with(
            |w,m:&AppState| *w.content_mut() =  m.banner.clone(),
            |w,m:&mut AppState| m.banner =  w.content().clone()
        );
# bound_widget.update(&mut AppState{banner:"I".to_string()}, &Event::<()>::Refresh(0), &mut Screen::default(), &Painter::default());
# }
```

The same with [`crate::widget::WidgetPropertyBinder`]:

```rust
# use termit::prelude::*;
# fn sample () {
struct AppState {
    banner: String
}
let mut bound_widget = Canvas::new(String::new()).back(Color::blue(false))
        .bind_property(|w| w.content_mut(), |m: &mut AppState| &mut m.banner);
# bound_widget.update(&mut AppState{banner:"I".to_string()}, &Event::<()>::Refresh(0), &mut Screen::default(), &Painter::default());
# }
```

# Company

The terminal UI space has grown considerably since termit started. These are
the libraries worth knowing about, and where termit sits among them.

* [ratatui](https://crates.io/crates/ratatui) - the mainstream choice, with by
  far the largest widget ecosystem. Immediate mode: the frame is redrawn each
  tick and widgets are values built per frame, so state lives in the
  application rather than in the widget tree.
* [cursive](https://crates.io/crates/cursive) - closest to termit in spirit. A
  retained tree of stateful views driven by callbacks, over several backends.
* [iocraft](https://crates.io/crates/iocraft) - declarative components with
  hooks, in the React idiom.
* [termwiz](https://crates.io/crates/termwiz) - the terminal layer behind
  WezTerm: parsing, cells and surfaces. Lower level than a widget toolkit.
* [bracket-lib](https://crates.io/crates/bracket-lib) - formerly rltk.
  CP437 and roguelike oriented, aimed at games.
* [ncurses](https://crates.io/crates/ncurses) - bindings to the C library, for
  when you want the original.
* [tui](https://crates.io/crates/tui) - termit's own early inspiration. It has
  been unmaintained since 2022; ratatui continues it.

Where termit takes a different line:

* **Widgets own their state.** A widget is a long-lived value in a retained
  tree, not a description rebuilt on every frame.
* **Widgets update the model directly.** The widget is handed `&mut Model`
  during `update`, with no messages, reducers or callbacks in between.
* **Async first**, and perfectly usable without async.
* **No backend dependency at all.** termit talks to the terminal through its
  own cross platform implementation rather than wrapping crossterm or
  ncurses, and `--no-default-features` still renders a UI.
* **A terminal emulator as a widget.** `TerminalEmulator` renders another
  program's ANSI output inside your layout. Opt in with the `temu` feature;
  it is still incomplete.

Take ratatui for the widest selection of widgets and the most company. Take
termit if the points above describe how you would rather write a TUI.
