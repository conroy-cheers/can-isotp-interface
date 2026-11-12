use can_isotp_interface::{
    Deadline, Destination, IsoTpAsyncRead, IsoTpAsyncReadSend, IsoTpAsyncWrite,
    IsoTpAsyncWriteSend, ReceiveError, ReceivedPayload, RecvMeta, TransmitError,
};
use std::{
    cell::Cell,
    future::Future,
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

fn ready<F: Future>(future: F) -> F::Output {
    match pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("test backend unexpectedly waited"),
    }
}

struct LocalReader(Rc<Cell<u8>>);
impl IsoTpAsyncRead for LocalReader {
    type Error = ();
    async fn recv_into(&mut self, out: &mut [u8]) -> Result<ReceivedPayload, ReceiveError<()>> {
        std::future::ready(()).await;
        out[0] = self.0.get();
        Ok(ReceivedPayload {
            meta: RecvMeta { reply_to: 1 },
            len: 1,
        })
    }
}

#[test]
fn local_capability_accepts_a_backend_with_a_non_send_future() {
    let mut reader = LocalReader(Rc::new(Cell::new(7)));
    let mut out = [0; 8];
    let payload = ready(IsoTpAsyncRead::recv_into(&mut reader, &mut out)).unwrap();
    assert_eq!(payload.meta.reply_to, 1);
    assert_eq!(&out[..payload.len], &[7]);
}

struct SendNode(u8);
impl IsoTpAsyncReadSend for SendNode {
    type Error = ();
    async fn recv_into(&mut self, out: &mut [u8]) -> Result<ReceivedPayload, ReceiveError<()>> {
        out[0] = self.0;
        Ok(ReceivedPayload {
            meta: RecvMeta { reply_to: 2 },
            len: 1,
        })
    }
}
#[derive(Clone, Copy)]
struct Budget;
impl Deadline for Budget {
    type Wait = core::future::Pending<()>;
    fn remaining(&self) -> Duration {
        Duration::from_secs(1)
    }
    fn wait(&self) -> Self::Wait {
        core::future::pending()
    }
    fn after(_: Duration) -> Option<Self> {
        Some(Budget)
    }
}
impl IsoTpAsyncWriteSend for SendNode {
    type Error = ();
    type Deadline = Budget;
    async fn send(
        &mut self,
        _: Destination,
        payload: &[u8],
        _: Budget,
    ) -> Result<(), TransmitError<()>> {
        self.0 = payload[0];
        Ok(())
    }
}

// Returning + Send here checks generic futures, rather than just concrete ones.
#[allow(clippy::manual_async_fn)]
fn receive<R: IsoTpAsyncReadSend<Error = ()>>(mut reader: R) -> impl Future<Output = u8> + Send {
    async move {
        let mut out = [0; 8];
        let received = IsoTpAsyncRead::recv_into(&mut reader, &mut out)
            .await
            .unwrap();
        assert_eq!(received.len, 1);
        out[0]
    }
}
#[allow(clippy::manual_async_fn)]
fn send<W: IsoTpAsyncWriteSend<Error = (), Deadline = Budget>>(
    mut writer: W,
) -> impl Future<Output = W> + Send {
    async move {
        IsoTpAsyncWrite::send(&mut writer, Destination::Physical(1), &[3], Budget)
            .await
            .unwrap();
        IsoTpAsyncWrite::send(&mut writer, Destination::Functional(2), &[4], Budget)
            .await
            .unwrap();
        writer
    }
}

#[test]
fn send_capabilities_forward_to_local_traits_and_preserve_generic_send_futures() {
    let node = ready(send(SendNode(0)));
    assert_eq!(ready(receive(node)), 4);
}

#[derive(Clone)]
struct ManualDeadline(Rc<Cell<bool>>);
impl Deadline for ManualDeadline {
    type Wait = core::future::Pending<()>;
    fn remaining(&self) -> Duration {
        if self.0.get() {
            Duration::ZERO
        } else {
            Duration::from_secs(1)
        }
    }
    fn wait(&self) -> Self::Wait {
        core::future::pending()
    }
    fn after(duration: Duration) -> Option<Self> {
        Some(Self(Rc::new(Cell::new(duration.is_zero()))))
    }
}
#[test]
fn earlier_picks_the_deadline_that_expires_first() {
    let expired = ManualDeadline(Rc::new(Cell::new(true)));
    let live = ManualDeadline(Rc::new(Cell::new(false)));
    assert!(
        can_isotp_interface::earlier(live.clone(), expired.clone())
            .remaining()
            .is_zero()
    );
    assert!(
        can_isotp_interface::earlier(expired, live)
            .remaining()
            .is_zero()
    );
}
#[test]
fn expired_deadline_wins_before_a_ready_operation_is_polled() {
    let expired = Rc::new(Cell::new(true));
    let called = Cell::new(false);
    let result = ready(can_isotp_interface::until(ManualDeadline(expired), async {
        called.set(true);
        7
    }));
    assert_eq!(result, Err(can_isotp_interface::TimedOut));
    assert!(!called.get());
}
#[test]
fn deadline_clones_keep_original_expiry_across_suspension() {
    let expired = Rc::new(Cell::new(false));
    let deadline = ManualDeadline(expired.clone());
    let calls = Cell::new(0);
    let operation = std::future::poll_fn(|_| {
        calls.set(calls.get() + 1);
        Poll::<()>::Pending
    });
    let mut wait = pin!(can_isotp_interface::until(deadline.clone(), operation));
    let mut context = Context::from_waker(Waker::noop());
    assert!(wait.as_mut().poll(&mut context).is_pending());
    expired.set(true);
    assert_eq!(deadline.remaining(), Duration::ZERO);
    assert_eq!(
        wait.as_mut().poll(&mut context),
        Poll::Ready(Err(can_isotp_interface::TimedOut))
    );
    assert_eq!(calls.get(), 1);
}
