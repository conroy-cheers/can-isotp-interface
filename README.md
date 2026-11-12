# can-isotp-interface

Dependency-free `no_std` transport contracts for addressed ISO-TP communication.
Blocking endpoint and flow-control APIs remain available. Version 0.2 replaces
the old combined async traits with independent capabilities.

`IsoTpAsyncRead::recv_into` waits indefinitely. Cancelling pending receive
preserves the next payload and reassembly. Oversized payloads are consumed and
reported as `BufferTooSmall`, including with an empty destination buffer.
`ReceiveError::{Rejected, Fatal, Closed}` distinguish consumed transaction failure
from session termination and retain backend causes.

`IsoTpAsyncWrite::send(Destination, payload, deadline)` takes a physical or
functional address and the writer's associated absolute `Deadline`. The deadline
bounds transport acceptance, not delivery. Abandoned unstarted work is discarded;
accepted work can complete after cancellation. `TransmitError::{Timeout,
Rejected, Fatal, Closed}` describe the operation's result without a separate
backend-wide error classifier.

`Deadline` supplies remaining time and an independently registered wait for the
same expiry. `Deadline::after(duration)` builds a deadline relative to now on
the same clock, and `earlier` picks whichever of two expires first, so generic
code can cap a relative timer by an absolute deadline without a runtime timer.
`until` checks expiry before polling the operation. Optional `tokio`
and `embassy` features implement it for native monotonic instants. Default
features require no runtime and remain `no_std`.

Implement local traits for local futures. Implement `IsoTpAsyncReadSend` and
`IsoTpAsyncWriteSend` to promise `Send` futures; blanket implementations provide
the local capabilities. Import the needed capability at the call site to avoid
ambiguous method names. Helpers that themselves await a generic deadline need
`Deadline::Wait: Send` when they promise a `Send` future.
