# Bitwig Extension Rendezvous Handover

Use this handover when designing another Rust plugin/client plus Bitwig controller extension pair.

## Pattern

Use the Bitwig controller extension as the host-facing rendezvous owner.

```text
Bitwig extension chooses/binds localhost port
  -> extension writes port into selected plugin/device parameter
  -> Rust side observes parameter and connects to 127.0.0.1:<port>
  -> Rust side sends framed payloads over the socket
  -> extension receives payloads and performs Bitwig host operations
```

This keeps host authority in Bitwig and keeps the Rust side independent from the Bitwig controller API.

## Why This Shape

- A plugin loaded in Bitwig can expose parameters, but it cannot directly own all host-control operations.
- A controller extension can create a Bitwig remote connection and can call host/controller APIs.
- The selected-device parameter is a practical rendezvous surface: it lets the extension tell the selected plugin which
  localhost port to use without global discovery or fixed ports.
- The socket carries runtime data after rendezvous. The parameter only communicates connection setup.

## Current BPM Example

In `midi-bpm-detector`:

- Extension module: `extension/extensions/beat-detection-controller`.
- Rust plugin crate: `rust/crates/midi-bpm-detector-plugin`.
- Rendezvous parameter: `DAW Port`.
- Port owner: the Bitwig extension.
- Socket client: the Rust plugin background task.
- Current message: one BPM update.
- Current wire frame: `uint32_be payload_length` followed by the payload.
- Current payload: `f32_be bpm`.

The extension follows the current selected Bitwig device. When that device exposes `DAW Port`, the extension pins its
cursor track/device and writes the bound port into that parameter. The plugin sees the non-zero port, connects to
localhost, and writes BPM frames outside the realtime callback.

## Guidance For A Broader Project

- Treat the rendezvous parameter as connection setup, not as the data protocol.
- Let the extension choose the port so it can use Bitwig's `createRemoteConnection` result.
- Keep the Rust audio/realtime path out of socket connection and writes. Use a background worker/task.
- Keep message framing explicit from the first multi-message use case.
- Start with the smallest payload that proves the workflow. Add versioning, capability exchange, or a root protocol
  package only when more than one message shape or compatibility level exists.
- Document which side owns host operations, which side owns socket connection attempts, and what happens when the
  selected Bitwig device changes.

## Copyable Prompt For Another Agent

We want to use the same Bitwig extension rendezvous pattern as `midi-bpm-detector`: the Bitwig controller extension
creates a localhost remote connection, writes the chosen port into a known parameter on the selected plugin/device, and
then the Rust side connects to `127.0.0.1:<port>`. The parameter is only the rendezvous slot; runtime data flows over the
socket. Keep Bitwig host operations in the extension, keep Rust audio/realtime paths away from socket writes, and use an
explicit length-prefixed frame once the socket carries runtime messages. Start narrow and do not build a general protocol
until there are multiple real message shapes or compatibility levels.
