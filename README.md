# datachannel-facade

One WebRTC data-channel API over [libdatachannel] natively and the
browser on wasm.

[libdatachannel]: https://github.com/tangobattle/libdatachannel-rs

Two crates:

* **`datachannel-facade`** — the API. Switches backend on
  `target_arch`, and carries the native-only options in a separate
  `platform::native` module rather than pretending they work everywhere.
* **`web-datachannel`** — the wasm backend: browser WebRTC, deliberately
  shaped like libdatachannel's Rust API. Usable on its own, but it exists
  to make the facade's switch thin.

## The trick that makes it thin

Most of the facade's types are re-exported straight from whichever
backend is compiled in, not redefined and mapped. That works because
`web-datachannel` was written to mirror libdatachannel — same type names,
same fields, same signatures. `SdpType::Offer` is a different type on each
target and means the same thing on both.

The interesting case is `set_local_description`. In the browser it
returns a Promise; in libdatachannel it returns immediately and the SDP
you actually wanted arrives through an `on_local_description` callback.
Since the return value was never the useful part, `web-datachannel` drives
the Promise on the microtask queue and reports through the same callback —
so the facade keeps **one synchronous signature on both targets**, with
the same observable behaviour. Callers don't branch, and `async` doesn't
leak into everything upstream.

A rejected Promise has nowhere to be returned to, so it is logged and the
connection is driven to `State::Failed`, which is what a failed
description exchange amounts to anyway.

## What the browser cannot do

These live in `platform::native` and are native-only *by nature*, not by
omission — web content is not permitted any of them, and won't be:

| option | why not on web |
| --- | --- |
| `set_local_description_ex` with pinned ICE credentials | a page cannot choose its own ufrag/pwd |
| `set_disable_fingerprint_verification` | a page cannot opt out of DTLS identity checks |
| `set_bind` (address / UDP port range) | a page cannot pick its ports |
| `selected_candidate_pair` | only via async `getStats()` |

Reaching for one of these is a compile error on wasm, which is the point:
a transport built on them is a native transport, and it should say so at
build time rather than fail at runtime.

## ICE servers

The facade takes ICE servers in libdatachannel's form — one string per
server, credentials inline (`turn:user:pass@host:port`) — because that is
what callers already write. The web backend splits them into the
browser's `{ urls, username, credential }`. The password may itself
contain `@`, so the host is taken from the last one.

## License

MPL-2.0
