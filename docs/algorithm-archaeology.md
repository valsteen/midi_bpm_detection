# Algorithm Archaeology

This document records the original reasoning behind the BPM detector. It is not a formal proof of the algorithm. It is
the design story that explains why the project grew from a small interval experiment into a realtime, visual,
parameterized tool.

## Starting Idea

The original intuition was simple: if a human taps note-on events with a tempo in mind, the intervals between taps should
eventually reveal that tempo. The hard part is that humans are not exact. A tap is not a perfect timestamp for the beat;
it is a noisy observation around the beat.

That led to the first major modeling choice: do not treat each interval as one exact value. Treat it as a plausible
region around the measured interval. The project models that uncertainty with a normal distribution, borrowing the
familiar shape of measurement error from physics.

## Interval-Domain Scoring

The core detector works in the interval domain:

```text
note-on event pairs
  -> measured interval between timestamps
  -> interval candidates around that measured duration
  -> score accumulated into an interval histogram
  -> winning interval converted to BPM
```

For each pair of note-on events, the detector measures the elapsed duration between them. Around that duration, it walks
through a precomputed normal distribution at the configured BPM histogram sample rate. Each sampled offset contributes
weight to nearby interval bins.

The histogram is therefore not a direct "events per BPM" counter. It is a weighted accumulation of plausible beat
intervals.

## Why The Histogram Exists

The detector ultimately returns a single estimated BPM, but the single number hides too much. During experiments, a
second or third candidate tempo may be nearly as plausible as the winner. Without the histogram, a jump from one tempo
to another looks mysterious. With the histogram, it is visible as competing peaks.

This is why visualization became central to the project. The UI is not only presentation: it is the feedback loop for
tuning the model and deciding whether a set of weights produces credible tempo estimates.

The interpolation and color choices in the GUI are separate from the calculation. They make changes easier to read and
nicer to look at, but they are not part of the BPM model.

## Why More Weights Appeared

Raw intervals are ambiguous:

- An observed interval may represent one beat, several beats, or a subdivision of a beat.
- A plausible BPM range is needed, otherwise the same interval can imply too many related tempos.
- Recent events should matter, but older events should not disappear immediately because tempo needs a short history to
  stabilize.
- Event properties such as velocity, pitch distance, and octave distance can help decide whether a pair of events should
  influence the tempo strongly or weakly.

The current scoring function combines these criteria into a weight. The implementation uses logarithmic scaling so the
parameters can be combined and tuned without every factor exploding independently.

The important practical point is that these weights exist because the algorithm is experimental. They expose enough
surface area to test whether a musical intuition actually helps tempo detection.

## Static And Dynamic Parameters

The project separates parameters by when they affect the model:

- Static BPM parameters change the shape of the model. They can alter buffer sizes or rebuild precomputed normal
  distribution data.
- Dynamic BPM parameters affect scoring. They can usually be changed while reusing the existing buffers and precomputed
  data.

This distinction is mostly technical, but it matters for realtime use. Static changes are more expensive and need to be
handled away from constrained processing paths.

## Interval Histogram Versus BPM Display

The core histogram is indexed by interval duration, not by BPM. The GUI converts each interval bin to a BPM label using:

```text
BPM = 60 / interval_duration_seconds
```

That conversion is nonlinear. Equal steps in interval duration are not equal steps in BPM. The same timing uncertainty in
milliseconds becomes visually warped when shown on a BPM axis.

This is why the normal distribution can appear to change shape in the BPM visualization even though the detector applies
the same interval-domain normal distribution everywhere. The uncertainty model did not change; the coordinate system did.

Another way to say it: `BPM = 60 / duration`, so `|dBPM / dduration| = 60 / duration^2`. At short intervals, small
duration changes correspond to larger BPM changes; at long intervals, the same duration change corresponds to smaller
BPM changes. Conversely, equal BPM spacing corresponds to different amounts of time depending on where you are on the
axis. This is a display artifact caused by translating duration to BPM.

The exact perceived direction of the artifact should be validated against the current GUI, because the bars are sampled
in duration space and then plotted at BPM positions.

## Future Direction: Tempo Distribution Space

The current detector already scores interval durations, then exposes a winning BPM and a histogram that the GUI remaps
to BPM labels. A future model could make that boundary more explicit: store and exchange tempo estimates as a
distribution over beat duration, and treat BPM as one possible projection for display or host integration.

This would preserve the uncertainty model at the same scale where it is computed. If human tapping imprecision is
assumed to be roughly constant in time, for example `+/- 20 ms`, then the equivalent uncertainty in BPM is not constant:

```text
BPM = 60 / duration_seconds
absolute BPM error ~= (BPM^2 / 60) * duration_error_seconds
relative BPM error ~= (BPM / 60) * duration_error_seconds
```

Under that assumption, higher tempos are inherently harder to estimate accurately in BPM terms. The beat duration is
shorter, so the same absolute tap jitter occupies a larger tempo span after conversion to BPM.

An implementation direction:

- keep the detector's canonical histogram in beat-duration space;
- introduce a named output type such as `TempoEstimateDistribution` instead of passing raw histogram slices;
- include the duration axis metadata with the distribution: lowest duration, highest duration, sample rate/resolution,
  and normalization policy;
- derive the current single BPM estimate from the maximum-scoring duration bin, preserving today's host-facing behavior;
- let the GUI choose its projection: duration axis, nonlinear BPM axis, or possibly `log2(BPM)` / `log2(duration)`;
- keep interpolation/color effects in the GUI layer so they cannot be mistaken for scoring data.

A logarithmic tempo axis may be especially interesting because doubling and halving tempo become equal visual distances.
That matches the musical ambiguity already present in range folding, where an observed interval may be multiplied or
divided until it lands in the plausible beat-duration range.

Questions to settle before implementing:

- whether the canonical distribution should remain linearly sampled in duration, or move to a logarithmic duration axis;
- whether normal distribution parameters should stay expressed in absolute time, or support relative/musical units;
- how much of the distribution shape should be exposed to plugin hosts versus only to diagnostics/visualization;
- how to compare old and new estimates when the axis or normalization policy changes.

## Terminology Still Worth Refining

The current code uses names such as `multiplier` and `subdivision` for interval correction. The intent is that an
observed interval may need to be divided or multiplied until it lands in the plausible beat-duration range. That turns a
raw interval into an interval candidate.

Those names are serviceable but easy to confuse with musical subdivision terminology. If this area is refactored, useful
terms may be:

- observed interval: the direct duration between two note-on events;
- interval candidate: the duration being scored as a possible beat duration;
- range folding: the act of multiplying or dividing an observed interval until it lands in the configured BPM range;
- scoring criteria: recency, velocity, pitch relation, octave relation, range fit, high-tempo bias, and normal
  distribution weight.

## Why The Project Grew

The first feedback loops were simpler: generate output, inspect a result, tweak parameters, repeat. That was not enough
to understand whether real tapping was being captured correctly or why the estimate jumped.

The desktop mode came first because it was the fastest way to experiment. Compile, run, tap, inspect, tweak. It avoided
the packaging and host-integration work required by a DAW plugin while the core idea was still unstable.

The plugin mode is the real production target, but reaching a workable plugin state required extra architecture:
packaging, DAW parameter integration, plugin editor integration, realtime callback constraints, host MIDI timing, and
reload behavior. Once the plugin is built and loaded, the DAW feedback loop can be acceptable: in Bitwig, for example,
the plugin can be unloaded and reloaded without restarting the whole DAW. The hard part is getting to that workable
state. After that point, iteration can focus again on the core functionality while still testing it in the environment
where it is meant to be used.

The WASM mode was more of a showcase and learning target. It makes the detector demoable by opening a page, without
asking someone to build a Rust workspace or install a plugin. It also forced the shared code to stay honest across target
boundaries: browser tasks instead of native threads, no native MIDI service dependencies, and a clean enough surface for
components to be swapped per runtime.

This is where the project became partly about architecture itself. The useful product can be summarized as "tap and
estimate BPM", but the project also became a learning ground for realtime processing, multi-target Rust builds, reusable
UI, dependency isolation by crate, and efficient visualization. Some work, especially the polished histogram rendering,
is therefore intentionally a bit beyond the minimum product need: it helps explain, debug, and showcase the system.

The architecture is therefore partly the result of the algorithm's uncertainty. The model needed experimentation, and the
experimentation needed realtime visualization and careful runtime boundaries.
