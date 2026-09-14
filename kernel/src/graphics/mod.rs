//! kernel graphics.
//!
//! everything draws into the linear framebuffer limine hands us. no
//! acceleration, no double buffering yet, just honest memory writes.

pub mod console;
pub mod font;
pub mod framebuffer;
