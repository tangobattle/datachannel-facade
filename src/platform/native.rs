//! Native-only options and operations.
//!
//! Everything here is native-only because no browser exposes it — not
//! because the web backend hasn't got round to it. Pinning local ICE
//! credentials, skipping DTLS fingerprint verification, and fixing a UDP
//! port range are all deliberately unavailable to web content, and a
//! transport that needs them is a native transport. Reaching for one of
//! these is the compile error that says so.

pub use libdatachannel::LocalDescriptionInit;

/// Native-only knobs on [`crate::Configuration`].
pub trait ConfigurationExt {
    /// Bind to a specific address, and/or pin the local UDP port range.
    ///
    /// A fixed range is how a peer can be reached without signaling:
    /// the other side already knows where to look. `(port, port)` pins a
    /// single port.
    fn set_bind(
        &mut self,
        addr: Option<std::net::IpAddr>,
        port_range_begin: u16,
        port_range_end: u16,
    );

    /// Multiplex connections onto one UDP port.
    fn set_enable_ice_udp_mux(&mut self, value: bool);

    /// Gather TCP ICE candidates as well as UDP.
    fn set_enable_ice_tcp(&mut self, value: bool);

    /// Don't auto-generate an offer when a data channel is created.
    ///
    /// Needed whenever the local description has to be set explicitly —
    /// with pinned ICE credentials, or after several channels are
    /// created, so one offer covers all of them instead of a partial
    /// offer racing out on the first.
    fn set_disable_auto_negotiation(&mut self, value: bool);

    /// Skip the DTLS fingerprint check.
    ///
    /// Only meaningful when the remote description was fabricated
    /// locally rather than received, so its fingerprint cannot match —
    /// which is the case for a signaling-free transport where both sides
    /// synthesize each other's SDP from pinned credentials. Turning this
    /// on otherwise gives up the guarantee that you are talking to the
    /// peer whose SDP you were handed.
    fn set_disable_fingerprint_verification(&mut self, value: bool);
}

impl ConfigurationExt for crate::Configuration {
    fn set_bind(
        &mut self,
        addr: Option<std::net::IpAddr>,
        port_range_begin: u16,
        port_range_end: u16,
    ) {
        self.sys.bind_address = addr;
        self.sys.port_range_begin = port_range_begin;
        self.sys.port_range_end = port_range_end;
    }

    fn set_enable_ice_udp_mux(&mut self, value: bool) {
        self.sys.enable_ice_udp_mux = value;
    }

    fn set_enable_ice_tcp(&mut self, value: bool) {
        self.sys.enable_ice_tcp = value;
    }

    fn set_disable_auto_negotiation(&mut self, value: bool) {
        self.sys.disable_auto_negotiation = value;
    }

    fn set_disable_fingerprint_verification(&mut self, value: bool) {
        self.sys.disable_fingerprint_verification = value;
    }
}

/// Native-only operations on [`crate::PeerConnection`].
pub trait PeerConnectionExt {
    /// Set the local description with pinned ICE credentials.
    ///
    /// With a `LocalDescriptionInit` naming the local ufrag and pwd, a
    /// peer that already knows them can reconstruct this side's SDP
    /// without any exchange — which is what a signaling-free transport
    /// is built on. The browser has no equivalent: web content cannot
    /// choose its own ICE credentials.
    fn set_local_description_ex(
        &self,
        type_: Option<crate::SdpType>,
        init: Option<&LocalDescriptionInit>,
    ) -> Result<(), crate::Error>;

    /// The ICE candidate pair actually in use, as raw candidate strings
    /// `(local, remote)`. Errors until the agent has chosen one.
    ///
    /// Useful for telling a relayed route from a direct one (a relayed
    /// candidate reads `typ relay`). A browser can answer the same
    /// question only through `getStats()`, asynchronously, so it isn't
    /// offered cross-platform.
    fn selected_candidate_pair(&self) -> Result<(String, String), crate::Error>;
}

impl PeerConnectionExt for crate::PeerConnection {
    fn set_local_description_ex(
        &self,
        type_: Option<crate::SdpType>,
        init: Option<&LocalDescriptionInit>,
    ) -> Result<(), crate::Error> {
        self.native().set_local_description(type_, init)
    }

    fn selected_candidate_pair(&self) -> Result<(String, String), crate::Error> {
        self.native().selected_candidate_pair()
    }
}
