//! `Send`/`Sync` bounds that evaporate on wasm.
//!
//! The backends disagree about threads, and honestly so. libdatachannel
//! fires callbacks from its own network threads, so every callback it
//! takes must be `Send + Sync`. Browser objects are thread-affine —
//! their handlers only ever run on the thread that registered them — so
//! `web-datachannel` asks for nothing.
//!
//! Writing the callback setters twice under `#[cfg]` would mean two
//! copies of every signature; asserting `unsafe impl Send` on the wasm
//! side to make one signature fit would be worse, because it is a claim
//! about the *build* (`-Ctarget-feature=+atomics` gives wasm threads),
//! not about the target. These blanket markers say it once instead: they
//! alias the real bound off wasm and mean nothing on it.
//!
//! The consequence is worth stating plainly: [`crate::PeerConnection`]
//! and [`crate::DataChannel`] are `Send + Sync` natively and neither on
//! wasm, so wasm callers keep them — and anything holding them — on one
//! thread.

/// `Send`, except on wasm32 where it is no constraint at all.
#[cfg(not(target_arch = "wasm32"))]
pub trait WasmNotSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + ?Sized> WasmNotSend for T {}

#[cfg(target_arch = "wasm32")]
pub trait WasmNotSend {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> WasmNotSend for T {}

/// `Sync`, except on wasm32 where it is no constraint at all.
#[cfg(not(target_arch = "wasm32"))]
pub trait WasmNotSync: Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Sync + ?Sized> WasmNotSync for T {}

#[cfg(target_arch = "wasm32")]
pub trait WasmNotSync {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> WasmNotSync for T {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The markers are only useful if the native backend's real
    /// `Send + Sync` requirement still reaches a callback bounded by
    /// them, which relies on supertrait elaboration.
    #[test]
    fn markers_elaborate_off_wasm() {
        fn assert_send_sync<T: WasmNotSend + WasmNotSync>() {
            fn inner<T: Send + Sync>() {}
            inner::<T>();
        }
        assert_send_sync::<crate::PeerConnection>();
        assert_send_sync::<crate::DataChannel>();
    }
}
