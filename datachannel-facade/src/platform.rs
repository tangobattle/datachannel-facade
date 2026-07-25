//! Options one backend has and the other cannot.
//!
//! Kept out of the cross-platform API on purpose: a knob that silently
//! does nothing on half your targets is worse than one you have to ask
//! for by name. Reaching into [`native`] is also the compile error that
//! tells you a transport built on these is native-only.

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
