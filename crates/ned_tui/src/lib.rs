#![allow(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub(crate) mod log;
pub mod unicode;
pub mod geometry;
pub mod input;
pub mod output;
mod painter;
mod screen;
pub(crate) mod sys;

mod terminal;
mod termit;
pub mod widget;

pub use self::painter::*;
pub use self::screen::*;
pub use self::sys::Control;
pub use self::terminal::*;
pub use self::termit::*;

pub mod prelude {
    //! All the stuff you'll probably need when making Termit TUIs and/or widgets
    pub use crate::geometry::*;
    pub use crate::input::*;
    pub use crate::output::color::*;
    pub use crate::output::style::*;
    pub use crate::widget::*;
    pub use crate::*;
}
