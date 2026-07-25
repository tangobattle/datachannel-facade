//! One WebRTC data-channel API over [`libdatachannel`] natively and the
//! browser on wasm.
//!
//! [`libdatachannel`]: https://github.com/tangobattle/libdatachannel-rs
//!
//! # How thin this is, and why
//!
//! Most of the types below are re-exported straight from whichever
//! backend is compiled in, rather than being redefined here and mapped.
//! That is affordable because `web-datachannel` was written to mirror
//! libdatachannel's shape deliberately — same type names, same fields,
//! same signatures — so there is nothing to translate. `SdpType::Offer`
//! resolves to a different type on each target and means the same thing
//! on both.
//!
//! Two things do need real wrapping:
//!
//! * [`Configuration`], because libdatachannel's carries options no
//!   browser can honour (bind address, port range, DTLS certificate
//!   files, fingerprint verification). Offering those cross-platform and
//!   ignoring them on web would be a lie, so they live in
//!   [`platform::native::ConfigurationExt`] instead and this type only
//!   holds what both can do.
//! * [`PeerConnection`], because it takes a `Configuration`, and because
//!   its `on_data_channel` hands back a [`DataChannel`].
//!
//! # What the browser cannot do
//!
//! Everything a browser refuses is native-only *by nature*, not by
//! omission — see [`platform::native`]. Pinning local ICE credentials,
//! skipping DTLS fingerprint verification, and fixing a UDP port range
//! have no web equivalent and are not going to get one; a transport
//! built on them is a native transport.
//!
//! Thread-safety is on that list too. A libdatachannel connection is
//! `Send + Sync` and fires from its own threads; a browser one is
//! thread-affine and is neither. Rather than assert the native shape
//! onto JS values, callbacks are bounded with [`marker::WasmNotSend`] /
//! [`marker::WasmNotSync`] — `Send + Sync` off wasm, nothing on it — so
//! the bound is written once and holding a connection across threads is
//! native-only in the same honest way.

mod ice;
pub mod marker;
pub mod platform;
mod sys;

use marker::{WasmNotSend, WasmNotSync};

pub use sys::{DataChannelOptions, Description, Error, GatheringState, Reliability, SdpType, State, TransportPolicy};

/// Peer connection configuration — the part both backends honour.
///
/// Native-only options are reached through
/// [`platform::native::ConfigurationExt`], which keeps them out of code
/// that has to compile for both targets.
#[derive(Debug, Default, Clone)]
pub struct Configuration {
    /// ICE servers as URL strings, in libdatachannel's format:
    /// `stun:host:port`, or `turn:username:password@host:port` with the
    /// credentials inline. The web backend splits them back apart, since
    /// the browser wants them as separate fields.
    pub ice_servers: Vec<String>,

    pub ice_transport_policy: TransportPolicy,

    /// Backend-specific extras, set through the platform extension
    /// traits rather than directly. Empty on wasm — every extra is
    /// native-only, which is the whole point — hence the allow.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) sys: sys::ConfigExtras,
}

impl Configuration {
    /// An empty configuration.
    ///
    /// Set the public fields on the result, and reach native-only options
    /// through [`platform::native::ConfigurationExt`]. A constructor
    /// rather than `..Default::default()` because the backend extras
    /// field is private, which makes functional-record-update
    /// unavailable to other crates.
    pub fn new() -> Self {
        Self::default()
    }
}

/// A WebRTC peer connection.
pub struct PeerConnection {
    inner: sys::PeerConnection,
}

impl PeerConnection {
    pub fn new(config: Configuration) -> Result<Self, Error> {
        Ok(Self {
            inner: sys::new_peer_connection(config)?,
        })
    }

    pub fn close(&self) -> Result<(), Error> {
        self.inner.close()
    }

    /// Set the local description, generating it as needed.
    ///
    /// Returns as soon as the work is started; the resulting SDP arrives
    /// through [`Self::set_on_local_description`]. That is
    /// libdatachannel's contract, and the web backend matches it by
    /// driving the browser's Promise and reporting through the same
    /// callback — so this signature is synchronous on both targets.
    pub fn set_local_description(&self, type_: Option<SdpType>) -> Result<(), Error> {
        // The one call whose signature differs between backends:
        // libdatachannel takes per-description options that only it has.
        // See `platform::native::PeerConnectionExt` for the form that
        // uses them.
        sys::set_local_description(&self.inner, type_)
    }

    pub fn set_remote_description(&self, desc: &Description) -> Result<(), Error> {
        self.inner.set_remote_description(desc)
    }

    /// Add a remote ICE candidate. An empty string means
    /// end-of-candidates.
    pub fn add_remote_candidate(&self, cand: &str) -> Result<(), Error> {
        self.inner.add_remote_candidate(cand)
    }

    pub fn local_description(&self) -> Result<Description, Error> {
        self.inner.local_description()
    }

    pub fn remote_description(&self) -> Result<Description, Error> {
        self.inner.remote_description()
    }

    pub fn create_data_channel(&self, label: &str, options: DataChannelOptions) -> Result<DataChannel, Error> {
        Ok(DataChannel {
            inner: self.inner.create_data_channel(label, options)?,
        })
    }

    pub fn set_on_local_description(
        &mut self,
        cb: Option<impl Fn(&str, SdpType) + WasmNotSend + WasmNotSync + 'static>,
    ) {
        self.inner.set_on_local_description(cb);
    }

    /// Called with each gathered local candidate, and with an empty
    /// string once gathering is done.
    pub fn set_on_local_candidate(&mut self, cb: Option<impl Fn(&str) + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_local_candidate(cb);
    }

    pub fn set_on_state_change(&mut self, cb: Option<impl Fn(State) + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_state_change(cb);
    }

    pub fn set_on_gathering_state_change(
        &mut self,
        cb: Option<impl Fn(GatheringState) + WasmNotSend + WasmNotSync + 'static>,
    ) {
        self.inner.set_on_gathering_state_change(cb);
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(DataChannel) + WasmNotSend + WasmNotSync + 'static>) {
        self.inner
            .set_on_data_channel(cb.map(|cb| move |inner| cb(DataChannel { inner })));
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl PeerConnection {
    /// The backend connection, for [`platform::native`] to reach the
    /// operations that only exist there.
    pub(crate) fn native(&self) -> &sys::PeerConnection {
        &self.inner
    }
}

/// An open (or opening) data channel.
pub struct DataChannel {
    inner: sys::DataChannel,
}

impl DataChannel {
    pub fn send(&self, buf: &[u8]) -> Result<(), Error> {
        self.inner.send(buf)
    }

    pub fn close(&self) -> Result<(), Error> {
        self.inner.close()
    }

    pub fn is_open(&self) -> bool {
        self.inner.is_open()
    }

    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    pub fn buffered_amount(&self) -> Result<usize, Error> {
        self.inner.buffered_amount()
    }

    pub fn set_buffered_amount_low_threshold(&self, amount: usize) -> Result<(), Error> {
        self.inner.set_buffered_amount_low_threshold(amount)
    }

    pub fn set_on_open(&mut self, cb: Option<impl Fn() + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_open(cb);
    }

    pub fn set_on_closed(&mut self, cb: Option<impl Fn() + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_closed(cb);
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(&str) + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_error(cb);
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_message(cb);
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + WasmNotSend + WasmNotSync + 'static>) {
        self.inner.set_on_buffered_amount_low(cb);
    }
}
