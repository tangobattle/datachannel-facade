//! The native backend: [`libdatachannel`].

pub use libdatachannel::{
    DataChannel, DataChannelOptions, Description, Error, GatheringState, PeerConnection, Reliability, SdpType, State,
    TransportPolicy,
};

/// Native-only configuration, carried on [`crate::Configuration`] and
/// set through [`crate::platform::native::ConfigurationExt`].
///
/// These are all things a browser has no way to express, which is
/// exactly why they aren't on the cross-platform type.
#[derive(Debug, Default, Clone)]
pub struct ConfigExtras {
    pub bind_address: Option<std::net::IpAddr>,
    pub port_range_begin: u16,
    pub port_range_end: u16,
    pub enable_ice_tcp: bool,
    pub enable_ice_udp_mux: bool,
    pub disable_auto_negotiation: bool,
    pub disable_fingerprint_verification: bool,
}

pub fn new_peer_connection(config: crate::Configuration) -> Result<PeerConnection, Error> {
    let sys = config.sys;
    PeerConnection::new(libdatachannel::Configuration {
        ice_servers: config.ice_servers,
        ice_transport_policy: config.ice_transport_policy,
        bind_address: sys.bind_address,
        port_range_begin: sys.port_range_begin,
        port_range_end: sys.port_range_end,
        enable_ice_tcp: sys.enable_ice_tcp,
        enable_ice_udp_mux: sys.enable_ice_udp_mux,
        disable_auto_negotiation: sys.disable_auto_negotiation,
        disable_fingerprint_verification: sys.disable_fingerprint_verification,
        ..Default::default()
    })
}

/// libdatachannel takes per-description options; the cross-platform call
/// passes none. `platform::native` is where the other form lives.
pub fn set_local_description(pc: &PeerConnection, type_: Option<SdpType>) -> Result<(), Error> {
    pc.set_local_description(type_, None)
}
