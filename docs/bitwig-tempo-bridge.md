# Bitwig Tempo Bridge

This document describes the narrow bridge between the Rust plugin and the Bitwig controller extension. It is not a
general Bitwig remote-control protocol.

## Purpose

The CLAP/VST3 plugin can estimate BPM from incoming MIDI inside Bitwig, but the plugin cannot directly own Bitwig's host
transport tempo. The companion controller extension provides that host-facing capability.

The production Bitwig tempo-control path is:

```text
Rust plugin -> localhost RemoteSocket client -> Bitwig controller extension -> Bitwig transport tempo
```

## Install And Use

1. Build/install the Bitwig controller extension from `extension/`.
2. Load the extension in Bitwig.
3. Drop the CLAP/VST3 plugin somewhere that receives note input.
4. Select the plugin device in Bitwig.
5. Enable the plugin's `Send tempo` parameter when the plugin should drive Bitwig tempo.

The extension follows Bitwig's current selected device. When the selected device exposes the plugin's `DAW Port`
parameter, the extension pins its cursor track and cursor device to that selection. The pin keeps the extension focused on
the recognized plugin while it writes the rendezvous port.

## Rendezvous

The extension chooses the localhost port. The plugin learns it through a host-exposed plugin parameter.

1. On initialization, the extension creates a Bitwig remote connection with
   `ControllerHost.createRemoteConnection("BPM Receiver", port)`.
2. The extension starts with a random dynamic port and then uses `connection.port` if Bitwig reports a different bound
   port.
3. The extension watches direct parameter names on the selected device.
4. When it sees a parameter named `DAW Port`, it stores that direct parameter id, pins the cursor track/device, and writes
   the chosen port into the parameter.
5. The plugin's `DAW Port` parameter callback stores non-zero values as the pending tempo-controller port.
6. The plugin background task takes that pending port and opens a TCP connection to `127.0.0.1:<port>`.
7. Once connected, BPM updates flow from plugin to extension over that socket.

The `DAW Port` parameter is not a user-facing tempo value. It is the rendezvous slot that lets the Bitwig extension tell
the plugin which localhost port to connect to.

## Socket Frame

The Rust side writes one length-prefixed payload per BPM update:

```text
uint32_be payload_length
byte[payload_length] payload
```

For the current bridge:

```text
payload_length = 4
payload = f32_be bpm
```

The Rust code writes the full 8-byte frame. The Bitwig extension receive callback is written against the payload bytes it
receives from Bitwig's remote socket API, and decodes that payload with `ByteBuffer.wrap(payload).float`.

Keep this distinction explicit:

- external clients write a length-prefixed frame to Bitwig's remote socket;
- extension code handles the callback payload as one big-endian 32-bit floating-point BPM value.

## Runtime Ownership

- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs` owns the `DAW Port` host parameter.
- `rust/crates/midi-bpm-detector-plugin/src/task_executor.rs` owns the TCP connection and writes BPM frames outside the
  realtime callback.
- `extension/extensions/beat-detection-controller/src/main/kotlin/beatdetection/BeatDetectionExtension.kt` owns Bitwig
  device following, cursor pinning, remote connection setup, payload reception, and transport tempo writes.
- `extension/extensions/beat-detection-controller/src/main/kotlin/beatdetection/TempoControllerFrame.kt` owns payload
  decoding on the Kotlin side.

The realtime plugin callback never performs TCP writes. It only captures MIDI/audio-block facts and schedules background
work. Socket connection and writes belong to `TaskExecutor`.

## Failure Behavior

- If Bitwig cannot bind the initially requested port, the extension uses the actual bound port reported by Bitwig.
- If the selected device no longer exposes the remembered `DAW Port` parameter, the extension unpins the track and
  device.
- If the plugin is not selected, the extension cannot discover the parameter and no port is written.
- If the plugin has no non-zero `DAW Port`, it has no tempo-controller socket target.
- If the plugin cannot connect to the tempo controller within the short timeout, it logs the failure and continues.
- If a BPM write fails, the plugin drops the socket and waits for another rendezvous port.
- If no plugin connects, the extension stays loaded and waits for a client connection.

## Bridge Boundaries

- The bridge carries one BPM payload shape today: `f32_be bpm` inside Bitwig's length-prefixed remote socket frame.
- The bridge is scoped to tempo feedback, not general Bitwig remote control.
- Bitwig controller API calls stay in the Kotlin extension.
- Socket connection management and writes stay outside the plugin audio/realtime callback.
