//! Splitting libdatachannel-style ICE server URLs.
//!
//! The facade takes ICE servers in libdatachannel's form — one string
//! per server, with any credentials inline — because that is what
//! callers already write. The browser wants them as separate fields, so
//! the web backend splits them.
//!
//! The split lives here, compiled on every target, rather than in
//! `sys::web`: it is pure string handling, and behind a `wasm32` cfg its
//! tests would never run.

/// A server URL taken apart into the browser's shape.
///
/// Only the web backend has any use for this, but the module is compiled
/// everywhere so its tests run — hence the allow off wasm.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IceServerParts {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// Split `turn:username:password@host:port` into its parts.
///
/// Anything without inline credentials passes through as a bare URL,
/// which covers every `stun:` server and any `turn:` server whose
/// credentials are supplied out of band.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn parse_ice_server(url: &str) -> IceServerParts {
    let bare = || IceServerParts {
        urls: vec![url.to_owned()],
        username: None,
        credential: None,
    };

    // Credentials can only be in the userinfo, which is after the scheme
    // and before an `@`.
    let Some((scheme, rest)) = url.split_once(':') else {
        return bare();
    };
    // A password may itself contain `@`, so the host starts after the
    // *last* one.
    let Some((userinfo, host)) = rest.rsplit_once('@') else {
        return bare();
    };
    // No colon in the userinfo means there is no password to lift out.
    // Guessing would mangle the URL, so leave it alone.
    let Some((username, credential)) = userinfo.split_once(':') else {
        return bare();
    };

    IceServerParts {
        urls: vec![format!("{scheme}:{host}")],
        username: Some(username.to_owned()),
        credential: Some(credential.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_ice_server;

    #[test]
    fn a_stun_url_is_passed_through() {
        let s = parse_ice_server("stun:stun.example.com:3478");
        assert_eq!(s.urls, vec!["stun:stun.example.com:3478"]);
        assert_eq!(s.username, None);
        assert_eq!(s.credential, None);
    }

    #[test]
    fn inline_turn_credentials_are_split_out() {
        let s = parse_ice_server("turn:alice:s3cret@turn.example.com:3478");
        assert_eq!(s.urls, vec!["turn:turn.example.com:3478"]);
        assert_eq!(s.username.as_deref(), Some("alice"));
        assert_eq!(s.credential.as_deref(), Some("s3cret"));
    }

    /// A password may contain `@`, so the host comes from the last one.
    #[test]
    fn the_host_comes_from_the_last_at_sign() {
        let s = parse_ice_server("turn:alice:p@ss@turn.example.com:3478");
        assert_eq!(s.urls, vec!["turn:turn.example.com:3478"]);
        assert_eq!(s.username.as_deref(), Some("alice"));
        assert_eq!(s.credential.as_deref(), Some("p@ss"));
    }

    /// No colon in the userinfo means no password to lift out.
    #[test]
    fn a_credential_free_turn_url_is_left_alone() {
        let s = parse_ice_server("turn:turn.example.com:3478");
        assert_eq!(s.urls, vec!["turn:turn.example.com:3478"]);
        assert_eq!(s.username, None);
    }

    /// A bare host with no scheme has nowhere to hide credentials.
    #[test]
    fn a_schemeless_url_is_passed_through() {
        let s = parse_ice_server("turn.example.com");
        assert_eq!(s.urls, vec!["turn.example.com"]);
        assert_eq!(s.username, None);
    }
}
