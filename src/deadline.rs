//! A portable absolute deadline and expiry-first wait.
use core::{
    future::{Future, poll_fn},
    pin::pin,
    task::Poll,
    time::Duration,
};

/// Clones preserve the clock and expiry; every wait has independent registration.
pub trait Deadline: Clone {
    type Wait: Future<Output = ()>;
    /// Saturates at zero after expiry.
    fn remaining(&self) -> Duration;
    fn wait(&self) -> Self::Wait;
    /// The deadline `duration` from now on this clock, or `None` if it is not
    /// representable. Lets generic code build relative timers on the caller's clock.
    fn after(duration: Duration) -> Option<Self>;
}

/// Whichever of two deadlines on the same clock expires first.
pub fn earlier<D: Deadline>(a: D, b: D) -> D {
    if b.remaining() < a.remaining() { b } else { a }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedOut;

/// Check expiry before polling an operation, including when both are ready.
/// An operation already accepted remains potentially delivered if observing its
/// result races expiry. This helper cannot undo transport acceptance.
pub async fn until<D: Deadline, F: Future>(
    deadline: D,
    operation: F,
) -> Result<F::Output, TimedOut> {
    let mut alarm = pin!(deadline.wait());
    let mut operation = pin!(operation);
    poll_fn(move |cx| {
        if deadline.remaining().is_zero() || alarm.as_mut().poll(cx).is_ready() {
            Poll::Ready(Err(TimedOut))
        } else {
            operation.as_mut().poll(cx).map(Ok)
        }
    })
    .await
}

#[cfg(feature = "tokio")]
impl Deadline for tokio::time::Instant {
    type Wait = tokio::time::Sleep;
    fn remaining(&self) -> Duration {
        self.saturating_duration_since(Self::now())
    }
    fn wait(&self) -> Self::Wait {
        tokio::time::sleep_until(*self)
    }
    fn after(duration: Duration) -> Option<Self> {
        Self::now().checked_add(duration)
    }
}

#[cfg(feature = "embassy")]
impl Deadline for embassy_time::Instant {
    type Wait = embassy_time::Timer;
    fn remaining(&self) -> Duration {
        Duration::from_micros(self.saturating_duration_since(Self::now()).as_micros())
    }
    fn wait(&self) -> Self::Wait {
        embassy_time::Timer::at(*self)
    }
    fn after(duration: Duration) -> Option<Self> {
        let micros = u64::try_from(duration.as_micros()).ok()?;
        Self::now().checked_add(embassy_time::Duration::from_micros(micros))
    }
}
