# Plugin Flow

This document describes the CLAP/VST3 plugin path. This is the production mode and the strictest runtime boundary.

## Buffer-Oriented Processing

Plugin MIDI processing is not the same shape as a small program reading one MIDI event at a time. The host calls the
plugin with an audio buffer representing a short span of time. MIDI events are delivered with timing offsets inside that
same span.

In this project, `process()` uses:

- `buffer.samples()` to advance the plugin's absolute sample clock after each processed block;
- `context.next_event()` to iterate host events for the current block;
- `event.timing()` to place each MIDI event inside the block;
- `sample_to_duration(sample_rate, current_sample + event.timing())` to create the timestamp used by `BPMDetection`.

That buffer-oriented shape matters because plugin code needs exact timing relative to the audio timeline. Treating MIDI
as an event stream detached from audio buffers would make the result sloppier and less representative of how the host
actually schedules audio and MIDI.

## Realtime Callback Boundary

The plugin `process()` callback should stay small and predictable. It runs in the host's realtime processing context, so
it should avoid blocking work, unbounded allocation, and heavyweight BPM computation.

The callback does only the immediate block-local work:

```text
host process block
  -> inspect transport tempo
  -> iterate MIDI events in this block
  -> map note-on messages to core note-on events with absolute timestamps
  -> push compact events into a fixed ring buffer
  -> schedule background BPM work when needed
```

The ring buffer is the handoff boundary. The callback uses `try_push`; if the buffer is full, it logs and keeps the
realtime path moving instead of blocking.

## Background Task Boundary

`TaskExecutor` runs outside the realtime callback. It drains the ring buffer, updates `BPMDetection`, and handles side
effects that do not belong in `process()`:

- forwarding DAW BPM to the GUI;
- computing BPM and histogram data;
- sending optional BPM feedback to the Bitwig controller bridge;
- pushing GUI updates through `GuiRemote`;
- applying delayed static/dynamic parameter changes.

The key distinction is that `process()` captures block-accurate facts, while background tasks do the expensive or
externally visible work.

## Config Timing

DAW/plugin parameter changes can arrive through the host while audio is processing. The plugin delays static and dynamic
config application by a short sample-based window before scheduling background tasks.

- Static BPM config can rebuild detection buffers or precomputed data.
- Dynamic BPM config changes scoring values and can reuse the existing model shape.

The delay helps group related parameter changes and avoids doing model work directly from the realtime callback.

## Tempo Feedback

The plugin cannot act as a system MIDI device or MIDI clock provider. It is loaded by the host as a plugin. For tempo
feedback, it optionally writes detected BPM to a localhost controller bridge. That bridge is separate from the realtime
MIDI/audio callback and is handled by the background task executor.

This differs from the desktop/native MIDI path, where the app can own a virtual MIDI output and emit MIDI clock as an
experimental integration route.
