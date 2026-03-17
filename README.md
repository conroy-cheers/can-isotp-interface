# can-isotp-interface

`can-isotp-interface` defines dependency-light Rust traits for addressed ISO-TP transports used by UDS-style communication.

The crate provides shared sync and async endpoint traits for sending payloads to a peer address, sending functional requests, and receiving payloads together with reply metadata. It also includes the common status and error types needed around those operations, plus an optional trait for backends that support runtime receive-side flow-control configuration.

It is intended as a small interface crate that can be implemented by different backends while presenting the same transport surface to higher-level diagnostic code. The crate is `no_std` and is designed to work as the common boundary between user-space and kernel-backed ISO-TP implementations.
