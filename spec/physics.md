# ugoe Physics

**ugoe Specification 0.1 — Draft.** See [README.md](README.md) for the status, conventions and
license of this specification.

This document defines the simulation of [physics settings](model.md#physics-settings): pendulum
chains driven by parameters, their outputs to parameters, the order of rigs and relays between them,
and *dynamic physics* — outputs that move deformers and vertices directly, colliders, and gravity
frames. The data is defined in [model.md](model.md#physics-settings).

## Contents

- [Overview](#overview)
- [Coordinates and units](#coordinates-and-units)
- [State](#state)
- [Advancing time](#advancing-time)
- [Inputs](#inputs)
- [Pendulum step](#pendulum-step)
- [Rest pose](#rest-pose)
- [Outputs](#outputs)
- [Rig order and relays](#rig-order-and-relays)
- [Dynamic physics](#dynamic-physics)
- [Evaluation with dynamic physics](#evaluation-with-dynamic-physics)
- [Unknown values and invalid references](#unknown-values-and-invalid-references)
- [Determinism](#determinism)
- [Conformance](#conformance)

## Overview

Each rig works in three stages:

1. **Inputs:** parameter values (for example the head angle) are normalized and summed into a
   *root pose*: a translation and an angle of the chain's root.
2. **Pendulum chain:** a chain of particles joined by distance constraints is simulated under
   gravity, wind and the motion of the root, in fixed time steps.
3. **Outputs:** positions of particles, or angles between successive segments, are written to
   parameters (for example hair sway) or, with dynamic physics, added to deformers and vertices.

*Informative:* the coefficients were chosen to give the behavior that 2D rigging artists expect from
pendulum physics ("mobility", "delay", "acceleration", "radius"); they are defined by the equations
below, not by any other system.

## Coordinates and units

- The physics space is two-dimensional with `y` pointing **down**, like the canvas. For rigs that do
  not use dynamic physics, lengths have no fixed unit: they are the units of the normalization range
  `normalization.position` and of the particles' `position` and `radius`. For dynamic physics the
  physics space is the canvas and lengths are canvas pixels ([Dynamic physics](#dynamic-physics)).
- Angles are in degrees, clockwise positive (as seen on screen). `rotate(v, θ)` rotates vector `v` by
  `θ` degrees: `rotate(v, θ) = R(θ) · v` with `R` as in [evaluation.md](evaluation.md#overview).
- The coefficients `mobility` and `delay` are defined per **reference frame** of 1/30 second, so
  that changing the simulation frequency `fps` hardly changes the motion.
- `normalize(v) = v / |v|`. Where this document says "or `d` if zero", the result is `d` when `|v|`
  is zero (or not finite).
- `signed_angle(a, b)` is the angle from vector `a` to vector `b` in degrees, clockwise positive:
  `atan2(a.x × b.y − a.y × b.x, a.x × b.x + a.y × b.y)` converted to degrees; 0 if `a` or `b` is the
  zero vector.

## State

A physics runtime keeps, for the physics settings of one model instance:

- `accumulator`: simulated time not yet consumed by steps (seconds), initially 0;
- `initialized`: whether the particles have been placed, initially `false`;

and for every rig:

- for every particle `i`: `position_i`, `last_i` (position at the start of the last step) and
  `velocity_i`;
- `pose`: the root pose used by the last step (including relay inputs);
- `external`: the part of `pose` computed from the frame's parameters (without relay inputs);
- `previous` and `current`: the output values of the last two steps, one per output.

**Reset** (for example after loading another model or a teleport) sets `accumulator = 0` and
`initialized = false` and clears the rig states. The next update places the particles again.

## Advancing time

The simulation advances in fixed steps of `step = 1 / fps` seconds. If `fps` is not a finite
positive number, 60 is used. Let `τ = step × 30`, the step length in reference frames.

An **update** by `dt` seconds with the current parameter values does the following:

```
1. For each rig r (in array order), compute its target pose:
       target_r = root_pose(r, parameters, excluding relay inputs)       (Inputs)
       target_r.translation += anchor_r                                   (Dynamic physics; 0 otherwise)
       fa = frame_angle_r                                                 (Dynamic physics; 0 otherwise)
       if initialized: fa = pose_r.frame_angle + wrap(fa − pose_r.frame_angle)
       target_r.angle += fa
       target_r.frame_angle = fa
   wrap(x) maps x to (−180, 180] degrees.

2. If not initialized:
       initialized = true; accumulator = 0
       for each rig r in array order:
           pose = target_r + relay inputs of r                           (Rig order and relays)
           place the chain in its rest pose for `pose`                    (Rest pose)
           pose_r = pose; external_r = target_r
           current_r = previous_r = outputs of r                          (Outputs)

3. If dt is finite and positive: accumulator += dt.
   whole = floor(accumulator / step)
   accumulator −= whole × step
   n = min(whole, 8)                                                     (the rest of the time is dropped)

4. Let start_r = external_r for every rig. For k = 1 … n:
       for each rig r in array order:
           external = lerp(start_r, target_r, k / n)                     (translation, angle and frame_angle)
           pose = external + relay inputs of r
           advance the chain one step with root pose `pose`              (Pendulum step)
             and rotation_delta = pose.angle − pose_r.angle
           pose_r = pose; external_r = external
           previous_r = current_r; current_r = outputs of r

5. α = clamp(accumulator / step, 0, 1)
   Write every output: value = lerp(previous, current, α)                 (Outputs)
```

- The input is interpolated across the steps of one update, so that a large change of the input in
  one frame is spread over the steps.
- The outputs are interpolated between the last two steps by the time left in the accumulator. This
  makes the motion smooth when the display frame rate and `fps` differ, at the cost of a delay of up
  to one step.
- In the first update (and the first after a reset), the particles are placed in the rest pose for
  the current input, so a model loaded with its head tilted does not start swinging.
- The limit of 8 steps per update avoids long stalls after a pause; the excess time is dropped.
- The outputs are written in every update, even when no step was taken.

## Inputs

`root_pose(r, parameters, filter)` sums the rig's inputs that pass `filter`:

```
pose = { translation: (0, 0), angle: 0 }
for each input (in array order):
    skip the input if its parameter does not exist or its source is unknown
    range = normalization.position   if source is x or y
          = normalization.angle      if source is angle
    n = normalize_input(value of the parameter, the parameter's min, default, max, range)
    if reflect: n = −n
    v = n × weight / 100
    source x:     pose.translation.x += v
    source y:     pose.translation.y += v
    source angle: pose.angle         += v           (degrees)
```

`normalize_input` maps the two sides of the parameter's default separately:

```
if value > default and max > default:
    t = min((value − default) / (max − default), 1);   n = lerp(range.default, range.max, t)
else if value < default and default > min:
    t = min((default − value) / (default − min), 1);   n = lerp(range.default, range.min, t)
else:
    n = range.default
```

Input weights are not clamped.

## Pendulum step

Particle 0 is the root. It is not simulated: it follows the root pose. Particles `i ≥ 1` are
simulated from the root outwards; each uses the already updated position of particle `i − 1` as its
parent.

For a step with root pose `pose` and `rotation_delta` (the change of the root angle since the last
step):

```
gravity' = rotate(settings.gravity, pose.angle)
wind'    = rotate(settings.wind, pose.frame_angle)          (frame_angle is 0 without dynamic physics)
Q        = R(rotation_delta × 0.2)

last_0 = position_0
position_0 = pose.translation + particles[0].position

for i in 1 … count − 1:
    parent      = position_(i−1)
    parent_last = last_(i−1)
    from_dir = normalize(position_i − parent_last), or rest_dir(i) if zero
    last = position_i
    h = τ × delay_i
    q = parent + Q · (position_i − parent)                  // dragged by the root's rotation
    q = q + velocity_i × h + (gravity' × acceleration_i + wind') × h²
    q = parent + normalize(q − parent) × radius_i           // distance constraint; rest_dir(i) if zero
    q = collide(q, parent, from = parent + from_dir × radius_i)   // dynamic physics only (Colliders)
    velocity_i = (q − last) / h × max(mobility_i, 0)^τ      if h > 0;  (0, 0) otherwise
    last_i = last
    position_i = q
```

`rest_dir(i) = normalize(particles[i].position − particles[i−1].position)`, or `(0, 1)` if zero: the
direction of the particle from its parent in the initial layout.

Meaning of the coefficients (per particle):

| Field | Meaning |
| --- | --- |
| `radius` | Distance to the previous particle (the length of the pendulum segment). |
| `mobility` | Fraction of the velocity kept per reference frame (0 to 1). 1 keeps the swing forever; smaller values stop it sooner. Values above 1 make the swing grow without limit. |
| `delay` | Time scale. 1 runs at normal speed; larger values react faster and swing faster; smaller values swing slowly. |
| `acceleration` | Gravity multiplier. Larger values pull the particle back to its rest direction more strongly. It does not multiply the wind. |

The root angle acts in two ways: gravity is rotated by it (so at rest the chain hangs at the root
angle), and a change of the angle drags each particle around its parent by 0.2 times the change.

## Rest pose

The **rest pose** for a root pose places every particle at rest (velocity 0, `last = position`)
hanging in the direction of its force:

```
rest_segment(i) = normalize(gravity' × acceleration_i + wind') × radius_i,
                  or rest_dir(i) × radius_i if zero
position_0 = pose.translation + particles[0].position
for i in 1 … count − 1:
    from = position_(i−1) + rest_dir(i) × radius_i
    position_i = position_(i−1) + rest_segment(i)
    position_i = collide(position_i, position_(i−1), from)          // dynamic physics only
```

`rest_offset(i) = Σ_{k=1..i} rest_segment(k)` (and `rest_offset(0) = (0, 0)`) is the position of
particle `i` relative to the root in the rest pose **without** colliders. It is used by additive
outputs and displacement outputs, with the `gravity'` and `wind'` of the step in which the outputs
are computed.

## Outputs

### Output values

After placing the particles and after every step, each output computes a value. An output whose
`particle` is 0 or not less than the number of particles has no value and no effect.

For outputs to `parameter` and `deformer_angle` targets:

```
segment = position_i − position_(i−1)
parent  = position_(i−1) − position_(i−2)     if i ≥ 2
        = reference                            if i = 1
source x:     s = segment.x
source y:     s = segment.y
source angle: s = signed_angle(parent, segment)
```

`reference` is `settings.gravity` (not rotated by the root angle), except for `deformer_angle`
outputs with dynamic physics ([Angle outputs](#angle-outputs)).

For an output to a parameter with `blend = add`, the same quantity computed on the rest pose (with
`rest_offset(i) − rest_offset(i−1)` as the segment, `rest_offset(i−1) − rest_offset(i−2)` or
`reference` as the parent) is subtracted from `s`: the output is the deviation from rest.

For outputs to `deformer_offset` and `vertices` targets, see
[Displacement outputs](#displacement-outputs).

Finally `value = s × scale`, negated if `reflect` is `true`.

### Writing to parameters

In step 5 of [Advancing time](#advancing-time), for each rig in array order and each of its outputs
in array order, with `value = lerp(previous, current, α)` and `w = clamp(weight / 100, 0, 1)`:

| `blend` | New parameter value |
| --- | --- |
| `overwrite` | `clamp(p + (clamp(value, min, max) − p) × w, min, max)` |
| `add` | `clamp(p + value × w, min, max)` |

where `p` is the parameter's current value and `min`, `max` its range. Several outputs to the same
parameter are applied one after another.

- `overwrite` replaces the parameter with the physics value (at weight 100). It is meant for
  parameters driven only by physics (hair sway).
- `add` adds the physics motion's deviation from rest to whatever the parameter holds, so physics
  sway can be layered on values written by tracking, motions or the user. At rest it adds nothing.
  Because the frame order of [motion.md](motion.md#order-of-operations-in-a-frame) restores parameters
  to their base values every frame, the additions do not accumulate across frames. A host that runs
  physics on its own MUST rewrite such parameters every frame before the update for the same reason.

## Rig order and relays

Rigs are advanced in the order of the `rigs` array, one after another within each step. A later rig
can read, as an input, a parameter that an earlier rig outputs; the two rigs are then chained
**within the same step**.

- An input of rig `r` is a **relay input** when its parameter is the target of a `parameter` output
  of a rig **before** `r` in the array. Which inputs are relay inputs depends only on the settings.
- When any rig has a relay input, the runtime keeps a **relay copy** of the parameter values, made
  at the start of each update from the current values:
  - At the start of every step (and before placing the particles), every parameter that is the
    target of a `parameter` output of any rig is reset in the copy to its value at the start of the
    update.
  - Right after a rig is placed or stepped, its current (not interpolated) output values are written
    to the copy with the rules of [Writing to parameters](#writing-to-parameters) (at `α` = 1).
- Relay inputs are read from the relay copy: `root_pose(r, relay copy, relay inputs only)` is added
  to the rig's pose (translation and angle; not the frame angle). They are not interpolated across
  the steps. All other inputs are read from the parameter values at the start of the update and are
  interpolated as described in [Advancing time](#advancing-time). The `rotation_delta` of a step
  includes the change due to relay inputs.
- The rest pose of the first update is also computed in array order, so a later rig rests at the
  rest outputs of earlier rigs.
- The values written to the real parameters are the interpolated outputs, as for any output.
- An input that reads the output of the same rig or of a later rig is not a relay input: it reads the
  value at the start of the update. In the frame order of
  [motion.md](motion.md#order-of-operations-in-a-frame), parameters are restored to their base values
  before physics runs, so this is the base value, not the output written in the previous update
  (physics outputs do not remain in the base values). A host that runs physics on its own, without
  restoring the parameters, reads the previous output instead.
- Without relay inputs, the order of the rigs does not affect the results.

*Informative:* parameters used only for relays should be marked `internal`
([model.md](model.md#parameters)).

## Dynamic physics

A rig uses **dynamic physics** when it has a `frame`, a non-empty `colliders`, or an output whose
target is not `parameter` (and not unknown). When any rig of the settings uses dynamic physics, the
runtime evaluates the model in two stages ([Evaluation with dynamic physics](#evaluation-with-dynamic-physics))
and computes, from the first-stage world transforms of the deformers, the following per rig and per
update. A runtime that does not support dynamic physics (feature `physics.dynamic`,
[bundle.md](bundle.md#baseline-features)) ignores the outputs to targets other than `parameter`, the
colliders and `frame`.

In dynamic physics, the physics space is the canvas: particle positions, `radius`,
`collision_radius`, collider radii and the position normalization range are in canvas pixels.

### Target resolution

- `deformer_angle` and `deformer_offset` targets MUST refer to rotation deformers, `vertices` with a
  `warp` owner to a warp deformer, and `vertices` with a `mesh` owner to a mesh. Outputs whose target
  does not exist or has the wrong kind are ignored.
- The rig's `colliders` entries that name no collider are ignored. Colliders whose `attach` names no
  deformer are ignored.

### Root anchor

The rig's root moves with an **anchor deformer** `A`:

1. the deformer of the first output (in array order) whose target resolved to a `deformer_angle` or
   `deformer_offset`; otherwise
2. the `frame` deformer, if it exists; otherwise
3. none (the canvas).

With `p0 = particles[0].position` and `F_A` the first-stage world transform of `A` (the mapping from
`A`'s local coordinates to the canvas), `anchor = F_A(p0) − p0` (0 without an anchor). The root is
therefore placed at `F_A(p0) + input translation`: with `p0 = (0, 0)` it sits at the anchor
deformer's origin and moves with it.

### Gravity frame

When `frame` names a deformer `D`, gravity and wind are interpreted in `D`'s coordinate system:

```
ĝ           = normalize(settings.gravity), or (0, 1) if zero
J           = Jacobian of F_D at (0.5, 0.5) if D is a warp deformer, at (0, 0) otherwise
frame_angle = signed_angle(ĝ, J · ĝ)
```

Gravity is rotated by `frame_angle` plus the input angle; wind by `frame_angle` only
([Pendulum step](#pendulum-step)). Changes of `frame_angle` drag the chain like changes of the input
angle, and `frame_angle` is unwrapped against the previous step so that it does not jump at ±180
degrees ([Advancing time](#advancing-time)). Without `frame`, `frame_angle = 0`.

The Jacobian of a world transform is the matrix of its partial derivatives (columns: derivatives in
local `x` and `y`): `M` for an affine transform, the identity for the identity, and for a warp the
derivative of the cell's bilinear mapping inside the lattice (the cell being chosen as in
[evaluation.md](evaluation.md#warp-deformers), including the edge cell at `u = 1` or `v = 1`) and
`A_u` / `A_v` for a direction in which the point lies outside the lattice.

### Angle outputs

A `deformer_angle` output adds `value × w` degrees (`w = clamp(weight / 100, 0, 1)`) to the
rotation deformer's angle ([evaluation.md](evaluation.md#dynamic-physics-offsets)). Its value is
computed like a parameter output, except:

- For particle 1, `reference = J · settings.gravity`, where `J` is the Jacobian of the first-stage
  world transform of the target deformer's **parent** at the target deformer's interpolated `origin`
  (including blend-shape deltas, without physics offsets). The angle is thus measured from the
  direction of gravity as seen in the deformer's own parent frame.
- If `det J < 0` (the parent frame is mirrored, for example by `reflect_x`), the value is negated.
  This applies to outputs of every particle: `J` depends only on the target deformer. The particle
  changes only the reference against which the angle is measured.

*Informative:* with a head rotated by 30 degrees and no `frame`, a hair deformer under the head
receives −30 degrees and the hair keeps hanging straight down. With `frame` set to the head, gravity
turns with the head and the hair tilts with it. Mapping chain segments one to one to a chain of
rotation deformers (`Hair1` → `Hair2` → `Hair3`) makes the deformers follow the segments like bones.

### Displacement outputs

`deformer_offset` and `vertices` outputs use the **displacement** of particle `i`: its position
relative to the root minus its rest position relative to the root (without colliders):

```
d = (position_i − position_0) − rest_offset(i)
source x:     value = (d.x, 0)
source y:     value = (0, d.y)
source angle: no value (the output is ignored)
value = value × scale, negated if reflect
```

The displacement is 0 when the whole chain merely follows the root; only the lagging swing is
output. To move both `x` and `y`, use two outputs.

- `deformer_offset`: the rotation deformer's world origin is moved by `value × w` canvas pixels.
- `vertices`: every control point `v` of the warp (after mapping to the canvas) or every vertex `v`
  of the mesh (after mapping to the canvas, before glues) is moved by `value × w × weights[v]`.
  Vertices without a weight (when `weights` is shorter) do not move.

Displacements are added in canvas coordinates after the world transform, because the physics space
is the canvas.

### Accumulating offsets

The offsets are rebuilt from zero in every update: for each rig in array order and each output in
array order, the output's contribution is added to the offset of its target. Several outputs to the
same target add up. The values used are the interpolated values `lerp(previous, current, α)`.

### Colliders

A collider's shape is placed with the first-stage world transform `F` of its `attach` deformer
(identity without `attach`): `center`, `a` and `b` are mapped through `F`; the radius is **not**
scaled (it stays in canvas pixels, even under a warp). Collider positions stay fixed during all the
steps of one update.

Let a collider's **core** be its center (a circle) or its segment `a`–`b` (a capsule), and `r` its
radius. For a particle `i` with margin `μ = max(collision_radius_i, 0)`, a candidate position `q`
with parent `P` is **blocked** by the collider when

```
dist(q, core) < r + μ                                     (the particle is inside the enlarged collider)
or ( dist(P, core) ≥ r  and  dist(segment P–q, core) < r )  (the segment crosses the collider itself)
```

where `dist` is the Euclidean distance between point or segment and core.

`collide(q, parent P, from)` keeps the segment length and places the segment outside the rig's
colliders:

```
L = |q − P|
near = colliders with dist(P, core) < L + r + μ           (colliders out of reach are skipped)
if near is empty: return q
if q = P: push q out of every near collider; return q
target = normalize(q − P);  base = normalize(from − P), or target if zero
at(θ) = P + R(θ) · base × L                               (θ in radians here, clockwise positive)
blocked(θ) = at(θ) is blocked by any near collider
step = min(max(min radius of near colliders, 0.5) / L, 10 degrees)
end  = signed angle from base to target, in radians, in (−π, π]
start = 0
if blocked(0):
    start = nearest free angle to 0 (see below); if none: push q out of every near collider; return q
span = end − start, wrapped to (−π, π]
samples = max(ceil(|span| / step), 1)
free = start
for k in 1 … samples:
    θ = start + span × k / samples
    if blocked(θ): return at(bisect(free, θ))
    free = θ
return q                                                  (not blocked: q is unchanged)
```

- `bisect(free, hit)` halves the interval 16 times, keeping `free` not blocked and `hit` blocked, and
  returns the final `free`.
- The nearest free angle is searched at `±min(step × k, π)` for `k = 1 … ceil(π / step)`, trying the
  positive (clockwise) angle before the negative one; the first free angle `θ` found is refined by
  `bisect(θ, sign(θ) × step × (k − 1))`.
- To **push `q` out** of a collider: if `dist(q, core) < r + μ`, move `q` to `c + dir × (r + μ)`,
  where `c` is the closest point of the core to `q` and `dir = normalize(q − c)`; if `q` lies on the
  core, `dir` is `(0, −1)` for a circle and the segment's direction rotated by 90 degrees
  counter-clockwise on screen (`(b − a).y, −(b − a).x`, normalized) for a capsule. Pushing out may
  change the segment's length. This happens only when the parent is inside a collider so large that
  no direction is free.

The segment is thus swept around its parent from its direction at the start of the step towards its
new direction, and stops just before the first collider it would hit: a fast swing does not tunnel
through colliders, and the segment length is kept. Only the turn around the parent is swept: the
start direction is taken relative to the parent's previous position (`from_dir`) and placed at the
parent's new position, so when the parent itself moves (for example with the root) the segment is
carried along without a check on the way and may pass through a collider. Because the velocity is
computed from the constrained position, a particle that hits a collider loses its speed.

*Informative:* when the rest pose of a chain intersects a collider, the chain is placed along the
collider in the first update rather than hanging straight. Authoring tools should warn about this.

## Evaluation with dynamic physics

When the settings use dynamic physics, one physics update with a model instance is:

1. Evaluate the first stage of the model ([evaluation.md](evaluation.md#dynamic-physics-offsets)) with
   the current parameter values.
2. From it, compute per rig the anchor, the frame angle, the references and mirroring of angle
   outputs, and the colliders' world shapes.
3. Advance time ([Advancing time](#advancing-time)).
4. Write the outputs: parameter outputs to the parameters, other outputs to the offsets
   ([Accumulating offsets](#accumulating-offsets)).

The second stage of evaluation then uses the final parameter values and the offsets.

## Unknown values and invalid references

| Case | Behavior |
| --- | --- |
| Input whose parameter does not exist, or whose `source` is unknown | Ignored. |
| Output whose `source` is unknown | Ignored. |
| Output whose `target` is unknown | Ignored. |
| Output whose target does not exist or has the wrong kind | Ignored. |
| Output whose `particle` is 0 or out of range | Ignored. |
| Unknown `blend` | `overwrite`. |
| Collider with an unknown `shape`, or whose `attach` does not exist | Ignored. |

## Determinism

Physics MUST be deterministic: the same settings, model, sequence of parameter values and sequence of
`dt` values MUST produce the same outputs. Physics MUST NOT use random numbers or the clock; time
enters only through `dt`.

## Conformance

A physics implementation conforms when, fed the same sequence of parameter values and `dt` values
from a reset, its outputs stay within these tolerances of the outputs defined here, in every update:

| Output | Tolerance |
| --- | --- |
| Parameter values written by physics | `|Δ| ≤ 1e-3 × (max − min)` of the parameter |
| Offsets of rotation deformer angles | `|Δ| ≤ 0.05` degrees |
| Offsets of rotation deformer origins, control points and vertices (each coordinate) | `|Δ| ≤ 0.05` canvas pixels |

- The tolerances assume `dt` values for which the number of steps per update is unambiguous in
  single precision (for example `dt = 1/fps` with the same value for both). Conformance examples use
  such sequences.
- Collisions and the sweep are discontinuous: when a candidate position lies within rounding
  distance of a collider's boundary, implementations may disagree. Conformance examples avoid such
  configurations.
