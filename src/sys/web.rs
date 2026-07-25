//! The wasm backend: [`web_datachannel`].

pub use web_datachannel::{
    DataChannel, DataChannelOptions, Description, Error, GatheringState, PeerConnection, Reliability, SdpType, State,
    TransportPolicy,
};

/// Nothing. Every native-only option is native-only because no browser
/// exposes it, so there is nothing to carry here — and
/// [`crate::platform::native`] isn't compiled for wasm, so nothing can
/// try to set one.
#[derive(Debug, Default, Clone)]
pub struct ConfigExtras;

pub fn new_peer_connection(config: crate::Configuration) -> Result<PeerConnection, Error> {
    PeerConnection::new(web_datachannel::Configuration {
        ice_servers: config
            .ice_servers
            .iter()
            .map(|url| {
                // libdatachannel takes credentials inline in the URL;
                // the browser wants them as separate fields.
                let parts = crate::ice::parse_ice_server(url);
                web_datachannel::IceServer {
                    urls: parts.urls,
                    username: parts.username,
                    credential: parts.credential,
                }
            })
            .collect(),
        ice_transport_policy: config.ice_transport_policy,
    })
}

pub fn set_local_description(pc: &PeerConnection, type_: Option<SdpType>) -> Result<(), Error> {
    pc.set_local_description(type_)
}
