# Examples

**ugoe Specification 0.1 — Draft.** See [../README.md](../README.md) for the status, conventions and
license of this specification.

Each directory contains a small model and the results a runtime is expected to produce for it.
**Evaluation examples** give the results at given parameter values ([Files](#files)); **timeline
examples** give the results frame by frame while time advances, with physics, motions, expressions,
effects and the pose ([Timeline examples](#timeline-examples)). The examples illustrate the rules and
let an implementation check itself against them, with the tolerances of
[evaluation.md](../evaluation.md#conformance), [physics.md](../physics.md#conformance) and
[motion.md](../motion.md#conformance). They are **informative**: where an example and the text
disagree, the text is correct (and the example is a bug in the specification).

Evaluation examples:

| Example | Shows |
| --- | --- |
| [single-quad](single-quad/) | Linear interpolation of mesh keyforms; clamping of parameter values to their range. |
| [warp-grid](warp-grid/) | A warp deformer: bilinear mapping inside the lattice and extrapolation outside it. |
| [rotation-chain](rotation-chain/) | Nested rotation deformers, scale and reflection, and a warp deformer under a rotation deformer. |
| [rotation-under-warp](rotation-under-warp/) | A rotation deformer and a warp deformer under a warp deformer. |
| [part-opacity](part-opacity/) | Opacity multiplied through the part tree and the deformer tree; drawing order and ties. |
| [glued-quads](glued-quads/) | Glues applied in array order. |
| [blend-shapes](blend-shapes/) | Blend-shape grids on a mesh, a warp deformer and a rotation deformer. |
| [smooth-keys](smooth-keys/) | Smooth (cubic Hermite) interpolation on three and four keys, combined with a linear axis; fields that stay linear; a smooth rotation angle. |

Timeline examples:

| Example | Shows |
| --- | --- |
| [physics-pendulum](physics-pendulum/) | A pendulum chain driven by position and angle inputs; outputs of angles and positions to parameters, reflected and additive; the rest pose of the first update; interpolation of outputs between steps; the limit of 8 steps per update; a reset. |
| [physics-relay](physics-relay/) | Rigs chained through a parameter within one step (relay inputs); an input that reads the output of a later rig. |
| [physics-dynamic](physics-dynamic/) | Dynamic physics: the root anchor, outputs to a rotation deformer's angle and origin and to the vertices of a mesh and the control points of a warp; a gravity frame given by a rotation deformer. |
| [physics-frames](physics-frames/) | Rest poses in a gravity frame given by a warp deformer (the Jacobian inside the lattice) and with wind; an angle output under a mirrored parent. |
| [physics-colliders](physics-colliders/) | A circle collider and a capsule collider attached to a deformer: rest poses that intersect a collider, and the sweep that stops a swinging segment. |
| [motion-curves](motion-curves/) | Linear, Bézier (with control times inside and outside the segment), stepped and inverse-stepped segments; events; the end of a motion. |
| [motion-crossfade](motion-crossfade/) | Priorities, cross-fades, per-curve fades, loops with events in every loop, stopping. |
| [expressions](expressions/) | The three expression operations over host values, fades, setting and adding expressions. |
| [effects](effects/) | Eye blink with a 64-bit seed, breathing, lip sync, effect-group strengths from a motion, turning effects off. |
| [pose](pose/) | A pose group switched by part opacity curves, links, combined part opacity overrides, turning the pose off. |

## Files

The files of an evaluation example:

- `model.json` — the model, as a standalone JSON document of type [`Model`](../model.md#model). It
  validates against [model.schema.json](../schema/model.schema.json) and satisfies the
  [validation rules](../model.md#validation). It uses a single 1×1 texture named `white`, which
  is not included (only its size matters).
- `expected.json` — the expected results:

  ```json
  {
    "model": "model.json",
    "summary": "What the example shows.",
    "cases": [
      {
        "parameters": { "X": 0.5 },
        "meshes": {
          "Quad": {
            "positions": [[96, 64], [224, 64], [224, 192], [96, 192]],
            "opacity": 1,
            "draw_order": 500,
            "multiply_color": { "r": 1, "g": 1, "b": 1 },
            "screen_color": { "r": 0, "g": 0, "b": 0 },
            "mask_strength": 1
          }
        },
        "render_order": ["Quad"]
      }
    ]
  }
  ```

  | Field | Meaning |
  | --- | --- |
  | `model` | The model file, relative to `expected.json`. |
  | `summary` | A description of the example. |
  | `cases` | Independent cases. Each case starts from a freshly loaded model. |
  | `cases[].parameters` | Parameter ID → value. The value is set as a host would set it (so it is clamped to the parameter's range); parameters not listed keep their default values. Then the model is evaluated once. |
  | `cases[].meshes` | Mesh ID → the expected results for that mesh. Every field is optional; only the fields present are checked. |
  | `positions` | The final vertex positions in canvas coordinates, one `[x, y]` per vertex in vertex order. |
  | `opacity` | The final opacity of the mesh (the product of the mesh's, its parts' and its deformers' opacities). |
  | `draw_order` | The interpolated drawing order value of the mesh. |
  | `multiply_color`, `screen_color` | The interpolated colors. |
  | `mask_strength` | The interpolated mask strength. |
  | `cases[].render_order` | Optional. The IDs of all meshes in the order they are drawn (back to front). |

  No physics, motions, effects or tracking run in evaluation examples: the parameter values are
  exactly those of the case.

## Tolerances

An implementation matches an example if every checked value is within these absolute tolerances of
the expected value. They are the conformance tolerances for evaluation results defined in
[evaluation.md](../evaluation.md#conformance), which is normative; the table repeats the ones that
apply to the fields of `expected.json`.

| Value | Tolerance |
| --- | --- |
| `positions` (each coordinate) | 0.01 canvas pixels |
| `opacity`, colors, `mask_strength` | 0.0001 |
| `draw_order` | 0.001 |
| `render_order` | exact, except that meshes whose `draw_order` values differ by at most 0.001 may appear in either order |

The expected values are derived by hand from the rules of the specification, and are rounded to four
decimal places where they are not exact.

## Timeline examples

A timeline example runs a model through a sequence of frames, as a host would: before each frame it
performs some host actions, then it advances the model by one frame with the order of operations of
[motion.md](../motion.md#order-of-operations-in-a-frame), and it checks the results of that frame.

The files of a timeline example:

- `model.json` — the model, as for evaluation examples.
- `*.physics.json`, `*.motion.json`, `*.expression.json` — physics settings, motions and expressions
  as standalone JSON documents ([model.md](../model.md#physics-settings)). They validate against the
  schemas in [schema/](../schema/README.md).
- `timeline.json` — the frames and the expected results:

  ```json
  {
    "model": "model.json",
    "physics": "model.physics.json",
    "motions": { "wave": "wave.motion.json" },
    "expressions": { "smile": "smile.expression.json" },
    "summary": "What the example shows.",
    "frames": [
      {
        "actions": [ { "set": { "X": 6 } }, { "start_motion": "wave", "priority": "normal" } ],
        "dt": 0.03125,
        "expect": {
          "parameters": { "Sway": 12.5 },
          "part_opacity": { "Arm": { "value": 1, "weight": 0.5 } },
          "events": ["step"],
          "offsets": { "deformer_angles": { "Hair": -30 } },
          "meshes": { "Quad": { "positions": [[96, 64], [224, 64], [224, 192], [96, 192]] } }
        }
      }
    ]
  }
  ```

  | Field | Meaning |
  | --- | --- |
  | `model` | The model file, relative to `timeline.json`. |
  | `physics` | Optional. The physics settings of the model. Without it, the model has no physics. |
  | `motions`, `expressions` | Optional. Key → file. The keys name the motions and expressions in actions. |
  | `summary` | A description of the example. |
  | `frames` | The frames, in order. The model is loaded fresh before the first frame: every parameter has its default value, and no motion or expression plays. |
  | `frames[].actions` | Optional. Host actions performed in array order before the frame (below). |
  | `frames[].dt` | The time step of the frame, in seconds. |
  | `frames[].expect` | Optional. The expected results of the frame (below). Every field is optional; only the fields present are checked. |

  Actions:

  | Action | Meaning |
  | --- | --- |
  | `{ "set": { <parameter ID>: <value>, … } }` | The host writes the values to the parameters (clamped to their ranges). |
  | `{ "start_motion": <key>, "priority": "idle" \| "normal" \| "force", "accepted": <bool> }` | The host requests the motion ([Playing several motions](../motion.md#playing-several-motions)). `accepted` is optional; when present, it is the expected answer (whether the request is accepted). |
  | `{ "stop_motions": true }` | The host stops all motions. |
  | `{ "set_expression": <key> }` | The host sets the expression (stopping the active ones). |
  | `{ "add_expression": <key> }` | The host adds the expression on top of the active ones. |
  | `{ "lip_sync": <level> }` | The host supplies the lip sync level from now on; `null` removes it. Initially there is no level. |
  | `{ "reset_physics": true }` | The host resets physics ([physics.md](../physics.md#state)). |
  | `{ "stages": { <stage>: <bool>, … } }` | The host turns stages on or off. Stages: `physics`, `eye_blink`, `breath`, `lip_sync`, `pose`. Initially all are on. |

  Expected results:

  | Field | Meaning |
  | --- | --- |
  | `parameters` | Parameter ID → the value passed to evaluation in this frame (the value remembered in step 10 of the frame order). |
  | `part_opacity` | Part ID → the part opacity override `{ "value", "weight" }` passed to evaluation. A weight of 0 means no override; then the value is not checked. |
  | `events` | The values of the motion events fired in this frame, in the order they were fired. Checked exactly. |
  | `offsets` | The dynamic physics offsets passed to evaluation ([evaluation.md](../evaluation.md#dynamic-physics-offsets)): `deformer_angles` (deformer ID → degrees), `deformer_origins` (deformer ID → `[x, y]`), `warp_points` (warp deformer ID → one `[x, y]` per control point), `mesh_vertices` (mesh ID → one `[x, y]` per vertex). Absent offsets are 0. |
  | `meshes` | The results of evaluating the model in this frame, as in `expected.json`. |

  No tracking input is supplied in the timeline examples.

### Timeline tolerances

| Value | Tolerance |
| --- | --- |
| Parameters that are the target of a physics output to a parameter | `1e-3 × (max − min)` ([physics.md](../physics.md#conformance)) |
| Other parameters | `1e-4 × max(1, max − min)` ([motion.md](../motion.md#conformance)) |
| `part_opacity` (value and weight) | 0.0001 |
| Offsets of angles | 0.05 degrees |
| Offsets of positions, and `meshes` positions (each coordinate) | 0.05 canvas pixels |
| Other `meshes` fields | as in [Tolerances](#tolerances) |

The time steps are chosen so that the number of physics steps per frame is unambiguous (the examples
use `fps = 32` and time steps that are exact in binary), and no candidate position of a physics step
lies within rounding distance of a collider's boundary.

### How the expected values were derived (informative)

The results of the first frames, of rest poses and of motions, expressions and effects are derived
by hand from the text (the summaries give the closed forms where they are short). The later frames of
swinging chains were computed with a separate, direct transcription of [physics.md](../physics.md) and
[motion.md](../motion.md), written from the text only, and checked against a second independent
implementation. Neither is the reference implementation.

## Relation to other test data (informative)

The ugoe repository also keeps a corpus of binary files for forward-compatibility tests (outside
this specification). That corpus checks that files written by each release stay readable and that
files using unknown features degrade as specified; it has no expected evaluation results. The
examples here are part of the specification: they are JSON, they show how the rules evaluate, and
they are not tied to any release of the reference implementation.

## License

The examples are part of the ugoe Specification and are licensed under [CC BY 4.0](../LICENSE).
If you include them in your program's test suite, keep a notice of their origin and license with
them, as described in [README.md](../README.md#license).
