//! Shared UDS-over-ISO-TP transport traits.
//!
//! This crate defines dependency-light interfaces for 29-bit UDS-style ISO-TP transports where
//! messages are always addressed to a peer (`u8` source/target addresses).

#![no_std]
#![allow(async_fn_in_trait)]

use core::time::Duration;

mod asynchronous;
mod deadline;
pub use asynchronous::{
    Destination, IsoTpAsyncRead, IsoTpAsyncReadSend, IsoTpAsyncWrite, IsoTpAsyncWriteSend,
    ReceiveError, ReceivedPayload, TransmitError,
};
pub use deadline::{Deadline, TimedOut, earlier, until};

/// Receive-side ISO-TP flow-control parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RxFlowControl {
    /// Block size (BS) to advertise (0 = unlimited).
    pub block_size: u8,
    /// Minimum separation time (STmin) to advertise between consecutive frames.
    pub st_min: Duration,
}

/// Optional extension trait: runtime-configurable RX flow-control.
pub trait IsoTpRxFlowControlConfig {
    /// Backend-specific error type.
    type Error;

    /// Update receive-side FlowControl parameters.
    fn set_rx_flow_control(&mut self, fc: RxFlowControl) -> Result<(), Self::Error>;
}

/// Result of a receive attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvStatus {
    /// No payload arrived before the timeout elapsed.
    TimedOut,
    /// One payload was delivered to the callback.
    DeliveredOne,
}

/// Control signal for receive callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvControl {
    /// Continue processing future payloads.
    Continue,
    /// Stop processing after the current payload.
    Stop,
}

/// Metadata accompanying a received payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecvMeta {
    /// Address to reply to.
    pub reply_to: u8,
}

/// Error for a send attempt.
#[derive(Debug)]
pub enum SendError<E> {
    /// The send did not complete before the timeout elapsed.
    Timeout,
    /// Backend-specific error.
    Backend(E),
}

/// Error for a receive attempt.
#[derive(Debug)]
pub enum RecvError<E> {
    /// Caller-provided buffer was too small to hold the received payload.
    BufferTooSmall {
        /// Needed payload length in bytes.
        needed: usize,
        /// Provided buffer length in bytes.
        got: usize,
    },
    /// Backend-specific error.
    Backend(E),
}

/// Result of a receive-into attempt (multi-peer with reply-to metadata).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvMetaIntoStatus {
    /// No payload arrived before the timeout elapsed.
    TimedOut,
    /// One payload was delivered into the provided buffer.
    DeliveredOne {
        /// Reply-to metadata (sender identity).
        meta: RecvMeta,
        /// Number of payload bytes written into the output buffer.
        len: usize,
    },
}

/// UDS-aware ISO-TP endpoint.
///
/// This trait is addressed: sends target a peer address and receives report `reply_to` metadata.
pub trait IsoTpEndpoint {
    /// Backend-specific error type.
    type Error;

    /// Send a payload to a specific peer address.
    fn send_to(
        &mut self,
        to: u8,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<(), SendError<Self::Error>>;

    /// Send a functional-addressing payload to a target functional address.
    fn send_functional_to(
        &mut self,
        functional_to: u8,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<(), SendError<Self::Error>>;

    /// Receive at most one payload and deliver it with reply-to metadata.
    fn recv_one<Cb>(
        &mut self,
        timeout: Duration,
        on_payload: Cb,
    ) -> Result<RecvStatus, RecvError<Self::Error>>
    where
        Cb: FnMut(RecvMeta, &[u8]) -> Result<RecvControl, Self::Error>;
}

/// Backward-compatible alias trait name for addressed sync endpoints.
pub trait IsoTpEndpointMeta: IsoTpEndpoint {}

impl<T> IsoTpEndpointMeta for T where T: IsoTpEndpoint {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sync_alias_remains_available() {
        fn accepts<T: IsoTpEndpointMeta>() {}
        struct Endpoint;
        impl IsoTpEndpoint for Endpoint {
            type Error = ();
            fn send_to(&mut self, _: u8, _: &[u8], _: Duration) -> Result<(), SendError<()>> {
                Ok(())
            }
            fn send_functional_to(
                &mut self,
                _: u8,
                _: &[u8],
                _: Duration,
            ) -> Result<(), SendError<()>> {
                Ok(())
            }
            fn recv_one<Cb>(&mut self, _: Duration, _: Cb) -> Result<RecvStatus, RecvError<()>>
            where
                Cb: FnMut(RecvMeta, &[u8]) -> Result<RecvControl, ()>,
            {
                Ok(RecvStatus::TimedOut)
            }
        }
        accepts::<Endpoint>();
    }
}
