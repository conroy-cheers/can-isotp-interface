# Changelog

This file lists the notable changes to `can-isotp-interface`. The crate is
pre-1.0, so under semver a minor version bump is a breaking change. The
[thincan migration guide](https://github.com/conroy-cheers/thincan/blob/master/docs/migration-0.2.md)
covers the upgrade of the whole stack.

## 0.2.0 (2026-11-12)

### Breaking

- `IsoTpAsyncEndpoint`, `IsoTpAsyncEndpointRecvInto`, `IsoTpAsyncEndpointMeta`
  and `IsoTpAsyncEndpointMetaRecvInto` are removed. Two independent
  capabilities replace them:
  - `IsoTpAsyncRead::recv_into(out)` waits indefinitely for the next payload.
    It returns a `ReceivedPayload` (metadata and length) or a `ReceiveError`.
  - `IsoTpAsyncWrite::send(destination, payload, deadline)` takes a
    `Destination` and one absolute deadline on the writer's `Deadline` clock.
    It fails with `TransmitError`.
- `ReceiveError` and `TransmitError` are `#[non_exhaustive]`. Matches on them
  outside this crate need a wildcard arm.
- The minimum supported Rust version is 1.88.

### Added

- `IsoTpAsyncReadSend` and `IsoTpAsyncWriteSend`, which promise `Send` futures
  for threaded executors. Blanket implementations provide the local traits.
- `ReceiveError` (`BufferTooSmall`, `Rejected`, `Fatal`, `Closed`) and
  `TransmitError` (`Timeout`, `Rejected`, `Fatal`, `Closed`). Both keep the
  backend's own error. `TransmitError` has `map`, `Display` and `Error`.
- `Destination` (`Physical`, `Functional`) and `ReceivedPayload`.
- The `Deadline` trait (`remaining`, `wait`, `after`), the `earlier` and
  `until` helpers, and `TimedOut`.
- The `tokio` and `embassy` features, which implement `Deadline` for
  `tokio::time::Instant` and `embassy_time::Instant`. Default features need no
  runtime and stay `no_std`.

### Unchanged

- The blocking API: `IsoTpEndpoint`, `IsoTpEndpointMeta`,
  `IsoTpRxFlowControlConfig` and their types.

## 0.1.0

- First release.
