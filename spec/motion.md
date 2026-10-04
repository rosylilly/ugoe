# ugoe Motion

**ugoe Specification 0.1 — Draft.** See [README.md](README.md) for the status, conventions and
license of this specification.

This document defines how parameter values and part opacities change over time: playback of
[motions](model.md#motions) (curves, fades, looping, events), cross-fading between motions,
[expressions](model.md#expressions), the automatic [effects](model.md#effects) (eye blink, breathing,
lip sync), the [pose](model.md#pose), and the order in which all of these, tracking and
[physics](physics.md) are applied within one frame before the model is [evaluated](evaluation.md).

## Contents

- [Overview](#overview)
- [Curves](#curves)
- [Motion playback](#motion-playback)
- [Playing several motions](#playing-several-motions)
- [Expressions](#expressions)
- [Effects](#effects)
- [Pose](#pose)
- [Part opacity](#part-opacity)
- [Order of operations in a frame](#order-of-operations-in-a-frame)
- [Unknown values](#unknown-values)
- [Determinism](#determinism)
- [Conformance](#conformance)

## Overview

A runtime advances a model by a time step `dt` (seconds, supplied by the host) once per frame. Each
stage below reads and writes the current parameter values; every write clamps the value to the
parameter's range ([evaluation.md](evaluation.md#parameter-values)). Stages that refer to a parameter
or part that does not exist in the model skip it.

Notation: `lerp(a, b, t) = a + (b − a) × t`; `clamp(x, lo, hi)`; the **easing** function

```
easing(x) = 0.5 − 0.5 × cos(π × clamp(x, 0, 1))
```

rises smoothly from 0 at `x = 0` to 1 at `x = 1`.

## Curves

A curve ([model.md](model.md#motions)) is a sequence of segments. The first segment is `start` with
the point `p`; each further segment runs from the end point of the previous segment (`p0`) to its
own point `to`. The value of a curve at time `t`:

1. If the curve has no segments, contains a segment of an unknown type, or its first segment is not
   `start`, the curve has **no value** (it is ignored).
2. If `t ≤ p.t` of the `start` segment, the value is `p.v`.
3. Otherwise, find the first segment with `t ≤ to.t`. Let `u = (t − p0.t) / (to.t − p0.t)`, or
   `u = 1` if `to.t ≤ p0.t`. The value is:

   | Segment | Value |
   | --- | --- |
   | `linear` | `p0.v + (to.v − p0.v) × u` |
   | `bezier` | see below |
   | `stepped` | `p0.v` if `u < 1`, `to.v` if `u = 1` |
   | `inverse_stepped` | `to.v` if `u > 0`, `p0.v` if `u = 0` |

4. If `t` is after the last point, the value is the last point's `v`.

A curve with a `start` segment after the first one breaks the rule of
[model.md](model.md#motions) that writers MUST follow. This document does not define its value: a
runtime MAY ignore the whole curve (give it no value at every time), and [Conformance](#conformance)
does not cover it. *Informative:* the reference runtime evaluates the segments before that `start` as
above and gives the curve no value after the last of them; for `start (0, 0)`, `linear (1, 1)`,
`start (2, 5)`, … the value is 0.5 at `t = 0.5` and there is no value for `t > 1`.

**Bézier segments.** A `bezier` segment with control points `c1`, `c2` is the cubic Bézier curve
`B(s) = (1−s)³ p0 + 3(1−s)² s c1 + 3(1−s) s² c2 + s³ to` for `s ∈ [0, 1]`, in both time and value.
Its value at time `t` is `B_v(s)` for the `s` with `B_t(s) = t`:

- If `to.t ≤ p0.t`, the value is `to.v`.
- Times are normalized to the segment: `x1 = clamp((c1.t − p0.t) / span, 0, 1)`,
  `x2 = clamp((c2.t − p0.t) / span, 0, 1)` and `x = clamp((t − p0.t) / span, 0, 1)`, with
  `span = to.t − p0.t`. Clamping the control points' times into the segment makes `B_t` monotonic, so
  `s` is unique. (The data itself is not changed.)
- Solve `b(s) = x` for `b(s) = 3(1−s)² s x1 + 3(1−s) s² x2 + s³`. `x ≤ 0` gives `s = 0` and `x ≥ 1`
  gives `s = 1`. Implementations MUST find `s` with `|b(s) − x| ≤ 1e-6` or to the precision of single
  precision arithmetic.
- The value is `(1−s)³ p0.v + 3(1−s)² s c1.v + 3(1−s) s² c2.v + s³ to.v`. The control points' values
  are not clamped.

*Informative:* the reference implementation solves for `s` with Newton's method safeguarded by
bisection: starting at `s = x` within `[0, 1]`, it keeps an interval containing the root, takes the
Newton step when it stays inside the interval and the derivative exceeds the single-precision
epsilon, bisects otherwise, and stops when `|b(s) − x| ≤ 1e-6`, the interval is narrower than
`1e-6`, or after 40 iterations.

## Motion playback

A playing motion has two clocks:

- `elapsed`: time since the motion started, used for fades; it never wraps.
- `time`: the position within the motion, used to evaluate curves and events; it wraps when the
  motion loops.

It also has an **end** time on the `elapsed` clock, or no end: a non-looping motion ends at
`max(duration, 0)`; a looping motion has no end until it is stopped.

**Starting** sets `elapsed = 0`, `time = 0`.

**Stopping** sets `end = min(end, elapsed + max(fade_out, 0))` (just `elapsed + max(fade_out, 0)` if
there was no end). A motion is **finished** when it has an end and `elapsed ≥ end`. A finished motion
applies nothing and fires no events.

**Advancing** by `dt` (nothing happens if the motion is finished, or if `dt` is not a finite number
greater than 0, such as NaN or infinity):

```
dt = min(dt, end − elapsed)                   if there is an end
elapsed += dt
if not looping, or not duration > 0 (NaN included):
    to = min(time + dt, max(duration, 0))
    fire the events in (time, to];  time = to
else:
    remaining = dt
    loop:
        if time + remaining < duration:
            fire the events in (time, time + remaining];  time += remaining;  stop
        fire the events in (time, duration]
        remaining −= duration − time;  time = 0
        if remaining ≤ 0: stop
```

**Events.** The events of a range are fired in order of their time `t`. The range is `(from, to]`,
except that it is `[from, to]` for the first range after the motion starts and for the first range
after each wrap, so that an event at time 0 fires at the start of every loop. When one advance spans
several loops, the events of every loop are fired. Events with the same time fire in array order.
Fired events are delivered to the host with their `value`.

A runtime MAY limit the number of loops one advance handles, so that a huge `dt` or a tiny `duration`
cannot stall it. What an advance does beyond the limit is not specified, except that `time` stays in
`[0, duration)`.

**Fade weight.** For a fade-in length `fi` and fade-out length `fo`:

```
w_in  = easing(elapsed / fi)             if fi > 0;  1 otherwise
w_out = 1                                if there is no end
      = easing((end − elapsed) / fo)     if fo > 0
      = 1 if end − elapsed > 0, else 0   otherwise
w     = w_in × w_out
```

A non-looping motion therefore starts fading out `fade_out` seconds before its end, and a stopped
motion fades out over `fade_out` seconds. Every curve uses its own `fade_in` and `fade_out` when they
are present and the motion's otherwise; the end time is always the motion's.

**Applying.** After advancing, for each curve (in array order) that has a value `c` at `time`, with
its fade weight `w` (times an external weight, normally 1):

| Target `kind` | Effect |
| --- | --- |
| `parameter` | `value = lerp(value, c, w)` (clamped) |
| `part_opacity` | A part opacity request `(c, w)` ([Part opacity](#part-opacity)) |
| `effect_group` | Sets the strength of an effect group ([Effect groups](#effect-groups)) |

A motion of length 0 that does not loop is finished as soon as it starts and applies nothing.

In the frame in which a motion reaches its end (`elapsed ≥ end` after advancing), it is finished: it
applies nothing in that frame, although the events up to the end fire. The parameters therefore keep
the values the motion applied in the previous frame (carried over as base values,
[Order of operations in a frame](#order-of-operations-in-a-frame)), not the curves' values at the
end time.

## Playing several motions

*This section defines the behavior of the reference motion manager. Runtimes that cross-fade between
motions SHOULD behave the same way.*

- The runtime keeps a list of playing motions in the order they were started, including motions
  that are fading out.
- Every start request has a **priority**: `idle` < `normal` < `force`. The **current motion** is the
  most recently started motion, as long as it has not been stopped and has not finished. A request
  with a lower priority than the current motion's is refused; a request with the same or a higher
  priority is accepted.
- Starting a motion stops every playing motion (they fade out over their own `fade_out`) and appends
  the new one (it fades in over its own `fade_in`). Stopping all motions stops every playing motion
  and clears the current motion.
- Every frame, each playing motion, in the order they were started, is advanced by `dt` and applied.
  Because each motion blends with `lerp` over the result of the previous ones, overlapping motions
  cross-fade from the old to the new. When the most recently started motion has finished, there is
  no current motion. Finished motions are removed.
- Events are collected per motion, in the order the motions were started.
- *Informative:* a looping idle motion never finishes; start it with `idle` so that `normal` requests
  can replace it.

### Effect groups

The strength `s` of an effect group (`EyeBlink` or `LipSync`) starts at 1. For each playing motion
in the order they were started, and each of its `effect_group` curves for that group with value `c`
and fade weight `w`: `s = lerp(s, c, w)`. Without such curves, `s = 1`; as a motion fades out, `s`
returns to 1. Curves naming other groups are ignored.

## Expressions

An expression is applied over the parameter values that motions and tracking produced. Each item
`(parameter, value, op)` of an expression with fade weight `w` changes the parameter's current value
`v` to:

| `op` | New value |
| --- | --- |
| `add` | `v + value × w` |
| `multiply` | `v × (1 + (value − 1) × w)` |
| `overwrite` | `v + (value − v) × w` |

The result is clamped to the parameter's range. Items with an unknown `op` are ignored.

- The fade weight uses the formula of [Motion playback](#motion-playback) with the expression's
  `fade_in` and `fade_out`, on a clock `elapsed` that starts at 0. An expression has no duration: it
  has no end until it is stopped. Advancing adds `dt` to `elapsed`, but not beyond the end; a `dt`
  that is not a finite number greater than 0 changes nothing.
- An expression is **stopped** and **finished** as a motion is ([Motion playback](#motion-playback)):
  stopping sets `end = min(end, elapsed + max(fade_out, 0))`, so it fades out over its `fade_out` from
  now, or stops having an effect at once if `fade_out = 0`; stopping it again keeps the earlier end.
- Several expressions may be active. They are advanced and applied in the order they were started.
  The host operations are:

  | Operation | Effect |
  | --- | --- |
  | Add an expression | Starts it on top of the active ones. |
  | Stop all expressions | Stops every active expression. |
  | Stop an expression by name | Stops every active expression with that `name`. |
  | Set an expression | Stops all expressions, then adds the new one. |

  Finished expressions are removed.
- Because the frame order restores the base values every frame
  ([Order of operations in a frame](#order-of-operations-in-a-frame)), `add` does not accumulate.

## Effects

### Eye blink

The eye blink cycles through four phases: `open` (waiting), `closing`, `closed` and `opening`.
The lengths of `closing`, `closed` and `opening` are the fields of the same names; the length of
each `open` phase is a random **interval**. The *openness* is:

| Phase (`t` = time in the phase) | Openness |
| --- | --- |
| `open` | 1 |
| `closing` | `1 − min(t / closing, 1)` (1 if `closing` is 0) |
| `closed` | 0 |
| `opening` | `min(t / opening, 1)` (1 if `opening` is 0) |

**Random intervals.** The generator is SplitMix64 with a 64-bit state initialized to `seed`. Each
draw:

```
state = state + 0x9E3779B97F4A7C15             (wrapping, mod 2⁶⁴)
z = state
z = (z xor (z >> 30)) × 0xBF58476D1CE4E5B9     (wrapping)
z = (z xor (z >> 27)) × 0x94D049BB133111EB     (wrapping)
z = z xor (z >> 31)
u = (z >> 40) / 2²⁴                            (a single-precision number in [0, 1))
interval = interval_setting × (0.5 + u)
```

The first interval is drawn when the runtime is created for the model (before the first frame) and
again whenever the eye blink settings are replaced; a new one is drawn each time the phase returns to
`open`. The intervals are therefore uniformly distributed in `[0.5, 1.5) × interval` and the same
seed always gives the same blinks.

**Advancing** by a finite `dt > 0` (any other `dt` changes nothing; if the eye blink `interval` is
not greater than 0, NaN included, the eye blink never advances):

```
loop:
    len = length of the current phase
    if t + dt < len: t += dt; stop
    dt −= max(len − t, 0);  t = 0;  go to the next phase (drawing a new interval when entering open)
```

Phases of length 0 are passed through. One advance may pass several phases. A runtime MAY limit the
number of phases one advance passes, so that a huge `dt` or tiny lengths cannot stall it; what an
advance does beyond the limit is not specified.

**Applying** with effect-group strength `s` ([Effect groups](#effect-groups)): each target parameter
is set to `v × (1 + (openness − 1) × s)`. The blink **multiplies** the current value, so eyes that a
motion or expression half-closed close further, and values above 1 keep their ratio.

### Breathing

Each breath entry keeps a phase `φ ∈ [0, cycle)`, initially 0. Advancing by a finite `dt > 0` (any other `dt` changes nothing) sets
`φ = (φ + dt) mod cycle` (Euclidean remainder), or `φ = 0` if `cycle ≤ 0`. Applying adds

```
weight × (offset + peak × sin(2π × φ / cycle))          (the sine term is 0 if cycle ≤ 0)
```

to the target parameter's current value. Breathing has no effect group.

### Lip sync

The host supplies a **level** between 0 and 1 (values outside are clamped; NaN counts as no level),
or no level. With no level, lip sync writes nothing. With a level `ℓ` and the strength `s` of the
`LipSync` group, each target parameter is set to `lerp(v, ℓ, s)`. Lip sync analyzes no audio itself.

### Replacing the effect settings

A host MAY replace the effect settings (`eye_blink`, `breath` and `lip_sync` together) while the
model runs. Replacing them, even with equal settings, restarts the effects from the new settings:

- The eye blink starts again: the generator's state is set to the new `seed`, the phase is `open`
  with `t = 0`, and the first interval is drawn at once ([Eye blink](#eye-blink)).
- The phase `φ` of every breath entry is 0.
- The lip sync level supplied by the host is kept; the new `lip_sync` parameters receive it from
  the next frame, and the old ones are no longer written.

The replacement takes effect in the next frame. It does not change the parameter values: a parameter
that the old settings wrote and the new ones do not returns to its base value in the next frame
([Order of operations in a frame](#order-of-operations-in-a-frame)).

## Pose

Each pose group keeps a **shown entry** and an opacity per entry. Initially the first entry is shown,
with opacity 1; the others have opacity 0.

Every frame, given the part opacity requests of motions for the parts of pose entries
([Part opacity](#part-opacity)) and `dt`:

1. In each group, among the entries whose part has a request with a value greater than 0, the entry
   with the largest value becomes the shown entry (the earlier entry on a tie). Without such requests
   the shown entry does not change (it may have been chosen by the host, below).
2. The shown entry's opacity increases by `max(dt, 0) / fade_duration` (to 1 at once if
   `fade_duration ≤ 0`), up to 1. Every other entry's opacity is limited to at most
   `1 − (the shown entry's opacity)`.
3. Each entry's opacity is written as a part opacity override with weight 1 to its `part` and to
   every part in its `links`.

**Showing an entry.** A host MAY choose the shown entry directly by naming the `part` of an entry
(not a part of its `links`). Between frames, that entry becomes the shown entry of its group at once;
the opacities do not change until the next frame, which fades towards it with step 2. A part that is
not the `part` of any entry changes nothing. In a frame in which motions request an entry of the
group with a value greater than 0 (step 1), the requests decide the shown entry, so while a motion
requests pose parts the host's choice has no lasting effect. While the pose stage is off, showing an entry still changes the
shown entry, but the opacities neither advance nor are written; when the stage is turned on again,
the fade continues from the opacities the pose kept.

While switching, the old and the new entry's opacities sum to 1. During a cross-fade between two
motions, the old motion's request loses weight while the new one's gains it, so the shown entry
changes when the new request becomes larger.

## Part opacity

Motion curves with the target `part_opacity` override the part's own opacity.

1. For each part, the requests of all playing motions are combined in the order the motions were
   started into one override `(v, W)`, starting at `(0, 0)`. For each request `(c, w)`:

   ```
   W' = 1 − (1 − W) × (1 − w)
   v  = (v × W × (1 − w) + c × w) / W'        if W' > 0
   W  = W'
   ```

   Overriding with `(v, W)` equals overriding with each request in turn:
   `lerp(lerp(o, v_old, W_old), c, w)`.
2. If the part is the `part` of a pose entry and the pose stage is on, the combined override is not
   applied; instead `v × W` is the pose request for that part ([Pose](#pose)).
3. Otherwise the override `(v, W)` is given to evaluation, which replaces the part's own opacity `o`
   by `lerp(o, v, W)` ([evaluation.md](evaluation.md#parts)).
4. If the pose stage is on, the pose then writes its overrides (weight 1) for the parts and links of
   all its entries, every frame (also in groups with a single entry). A pose without groups writes
   nothing.

Overrides are rebuilt every frame; when the curves end, parts return to their own opacity.

## Order of operations in a frame

A runtime that implements motions, effects or physics advances one frame by `dt` in this order:

1. **Restore base values.** Every parameter is set to its *base value*. If a parameter's current
   value differs (bit for bit) from the value the previous frame left in it, the host changed it
   between frames; that value becomes the new base value. (In the first frame, the base values are
   the parameters' current values.)
2. **Motions** ([Playing several motions](#playing-several-motions)).
3. **Tracking**, when the host supplies tracking input: tracked parameters are overwritten with the
   values computed by the avatar profile's tracking mapping
   ([avatar-profile.md](avatar-profile.md#computing-the-parameter-values)).
4. **Save base values:** the current values become the base values.
5. **Expressions.**
6. **Eye blink.**
7. **Breathing.**
8. **Physics** ([physics.md](physics.md#advancing-time)). With dynamic physics, the first stage of
   evaluation happens here ([evaluation.md](evaluation.md#dynamic-physics-offsets)).
9. **Lip sync.**
10. Remember the current parameter values as "the values this frame left" (for step 1 of the next
    frame).
11. **Part opacity** overrides from motion curves and the pose ([Part opacity](#part-opacity)).
12. **Evaluation** of the model ([evaluation.md](evaluation.md#order-of-evaluation)); with dynamic
    physics, this is the second stage.

Consequences:

- Stages 5 to 9 are applied on top of the base values every frame, so their changes (`add` of
  expressions, breathing, eye blink, additive physics) never accumulate across frames.
- The base value includes motions and tracking. When a motion ends, the last value it applied stays,
  and when the host stops supplying tracking input, the last tracked values stay. Tracking replaces
  the value of the parameters it drives, motion values included, and each frame drives only the
  parameters that receive a value ([avatar-profile.md](avatar-profile.md#computing-the-parameter-values)).
- A value set by the host between frames becomes the base value. It is overridden by a playing
  motion that targets the parameter (according to the motion's weight) and by tracking; expressions,
  eye blink and breathing are layered on it; physics reads it as an input or overwrites it as an
  output; lip sync overrides it while a level is supplied.
- Physics comes after expressions, eye blink and breathing because it reads the parameters they
  produce (the head angle, for example).
- A host MAY ask that motions take precedence over tracking. Then, while tracking input is supplied,
  the tracked values are written *before* step 2 (so a playing motion is applied on top of them,
  according to its weight), and step 3 writes only the tracked parameters the host chose to keep
  under tracking (none by default). Both writes happen before step 4, so the values are part of the
  base values as usual. Without this request, the order above applies unchanged.
- A host MAY turn off individual stages (physics, eye blink, breathing, lip sync, pose). A stage that
  is off does not touch parameters or part opacities; the eye blink and breathing clocks do not
  advance while they are off. When physics is turned off, its offsets are cleared; when it is turned
  on again it restarts from the rest pose (a reset). When the pose is off, part opacity requests for
  pose parts are applied as ordinary overrides.

## Unknown values

| Case | Behavior |
| --- | --- |
| Curve with an unknown target `kind` | Ignored. |
| Curve containing an unknown segment type | The whole curve is ignored. |
| `effect_group` curve with an unknown group name | Ignored. |
| Expression item with an unknown `op` | Ignored. |
| Curve or expression item whose parameter or part does not exist | Ignored. |

## Determinism

Everything in this document is deterministic: the same model, the same sequence of host actions
(starting and stopping motions, setting, adding and stopping expressions, levels, tracking input,
parameter changes, showing pose entries, replacing effect settings) and the same sequence of `dt` values produce the same parameter values and part
opacities. Randomness comes only
from the seeded eye blink generator; the clock is never read.

## Conformance

A runtime conforms when, fed the same sequence of host actions and `dt` values, the parameter values
and part opacity overrides it passes to evaluation differ from the values defined here by at most
`1e-4 × max(1, max − min)` for a parameter (with its range `min`, `max`) and `1e-4` for an opacity,
in every frame (curves that break the rule of [model.md](model.md#motions) excepted, see
[Curves](#curves)). The eye blink's random intervals are computed with integer arithmetic and MUST be
reproduced exactly (the single-precision `u` included).
