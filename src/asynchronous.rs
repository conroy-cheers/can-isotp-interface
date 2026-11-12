//! Independently owned addressed async transport capabilities.
use crate::{Deadline, RecvMeta};
use core::future::Future;

/// Physical or functional ISO-TP destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Physical(u8),
    Functional(u8),
}

/// Metadata and length of a complete payload copied into the caller's buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceivedPayload {
    pub meta: RecvMeta,
    /// Never exceeds the output buffer's length.
    pub len: usize,
}

/// A consumed rejection or a terminated transport.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReceiveError<E> {
    /// Consumed and discarded one payload, including for a zero-sized buffer.
    BufferTooSmall { needed: usize, got: usize },
    /// Consumed a failed transaction; subsequent I/O remains usable.
    Rejected(E),
    /// End the owning session and retain the original cause.
    Fatal(E),
    /// The endpoint already ended.
    Closed,
}

/// Result of attempting transport acceptance, not remote acknowledgement.
#[derive(Debug)]
#[non_exhaustive]
pub enum TransmitError<E> {
    Timeout,
    Rejected(E),
    Fatal(E),
    Closed,
}

impl<E> TransmitError<E> {
    /// Transform a cause without changing its lifecycle classification.
    pub fn map<F>(self, map: impl FnOnce(E) -> F) -> TransmitError<F> {
        match self {
            Self::Timeout => TransmitError::Timeout,
            Self::Rejected(error) => TransmitError::Rejected(map(error)),
            Self::Fatal(error) => TransmitError::Fatal(map(error)),
            Self::Closed => TransmitError::Closed,
        }
    }
}

/// Receive indefinitely. Cancellation preserves pending input and reassembly.
/// After consumption, return in the same poll without another suspension.
pub trait IsoTpAsyncRead {
    type Error;
    async fn recv_into(
        &mut self,
        out: &mut [u8],
    ) -> Result<ReceivedPayload, ReceiveError<Self::Error>>;
}

/// Receive capability with futures that can move between executor threads.
pub trait IsoTpAsyncReadSend: Send {
    type Error;
    fn recv_into(
        &mut self,
        out: &mut [u8],
    ) -> impl Future<Output = Result<ReceivedPayload, ReceiveError<Self::Error>>> + Send;
}
impl<T: IsoTpAsyncReadSend> IsoTpAsyncRead for T {
    type Error = T::Error;
    async fn recv_into(
        &mut self,
        out: &mut [u8],
    ) -> Result<ReceivedPayload, ReceiveError<Self::Error>> {
        IsoTpAsyncReadSend::recv_into(self, out).await
    }
}

/// Serialize transport acceptance using the caller's original absolute deadline.
/// Cancellation discards observed unstarted requests; accepted work may still
/// complete. Its payload/protocol state must remain independently owned. Queue
/// wrappers may wait for completion notification after acceptance. No automatic
/// retries are permitted by this contract.
pub trait IsoTpAsyncWrite {
    type Error;
    type Deadline: Deadline;
    async fn send(
        &mut self,
        destination: Destination,
        payload: &[u8],
        deadline: Self::Deadline,
    ) -> Result<(), TransmitError<Self::Error>>;
}

/// Transmit capability with `Send` futures. Generic deadline-waiting helpers
/// additionally require `<Self::Deadline as Deadline>::Wait: Send`.
pub trait IsoTpAsyncWriteSend: Send {
    type Error;
    type Deadline: Deadline + Send;
    fn send(
        &mut self,
        destination: Destination,
        payload: &[u8],
        deadline: Self::Deadline,
    ) -> impl Future<Output = Result<(), TransmitError<Self::Error>>> + Send;
}
impl<T: IsoTpAsyncWriteSend> IsoTpAsyncWrite for T {
    type Error = T::Error;
    type Deadline = T::Deadline;
    async fn send(
        &mut self,
        destination: Destination,
        payload: &[u8],
        deadline: Self::Deadline,
    ) -> Result<(), TransmitError<Self::Error>> {
        IsoTpAsyncWriteSend::send(self, destination, payload, deadline).await
    }
}

impl<E: core::fmt::Display> core::fmt::Display for TransmitError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Timeout => f.write_str("transport acceptance deadline elapsed"),
            Self::Rejected(e) => write!(f, "transport rejected request: {e}"),
            Self::Fatal(e) => write!(f, "transport failed: {e}"),
            Self::Closed => f.write_str("transport closed"),
        }
    }
}
impl<E: core::error::Error + 'static> core::error::Error for TransmitError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Rejected(e) | Self::Fatal(e) => Some(e),
            _ => None,
        }
    }
}
