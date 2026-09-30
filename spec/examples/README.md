# Examples

**ugoe Specification 0.1 — Draft.** See [../README.md](../README.md) for the status, conventions and
license of this specification.

Each directory contains a small model and the results a runtime is expected to produce for it.
**Evaluation examples** give the results at given parameter values ([Files](#files)); **timeline
examples** give the results frame by frame while time advances, with physics, motions, expressions,
effects, the pose and tracking ([Timeline examples](#timeline-examples)). **Rendering examples** give the
expected colors of chosen pixels of a rendered frame ([Rendering examples](#rendering-examples)).
**File examples** are bundle and module files with the data a reader obtains from them
([File examples](#file-examples)). The examples illustrate the rules and let an implementation check
itself against them, with the tolerances of [evaluation.md](../evaluation.md#conformance),
[physics.md](../physics.md#conformance), [motion.md](../motion.md#conformance) and
[rendering.md](../rendering.md#conformance). They are **informative**: where an example and the text
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
| [pose-select](pose-select/) | The host showing pose entries: fades to the chosen entry, parts that name no entry, motion requests that decide the shown entry, showing an entry while the pose is off. |
| [effects-replace](effects-replace/) | The host replacing the effect settings: the eye blink and breathing restart from the new settings (also when they are equal), lip sync moves the kept level to new parameters, empty settings. |
| [tracking](tracking/) | Face tracking input mapped by the avatar profile: the standard mapping, overrides that change the sign, redirect a formula, map a blend shape and sum two inputs, ignored elements, internal parameters, clamping; tracking after motions and before expressions; removing the input. |

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
  | `{ "stop_expressions": true }` | The host stops all expressions ([Expressions](../motion.md#expressions)). |
  | `{ "lip_sync": <level> }` | The host supplies the lip sync level from now on; `null` removes it. Initially there is no level. |
  | `{ "tracking": <input> }` | The host supplies this tracking input in every frame from now on ([avatar-profile.md](../avatar-profile.md#tracking-input)); `null` removes it. Initially there is no tracking input. The input is `{ "head": { "yaw", "pitch", "roll" }, "body": { "yaw", "pitch", "roll" }, "shoulders": { "left", "right" }, "blend_shapes": { <ARKit name>: <value>, … } }`: `head` is required, `body` and `shoulders` and each of their fields are optional (absent: the input does not have it), and blend shapes not listed are 0. |
  | `{ "show_pose": <part ID> }` | The host shows the pose entry whose `part` is this part ([Pose](../motion.md#pose)). |
  | `{ "set_effects": <Effects> }` | The host replaces the effect settings with this [`Effects`](../model.md#effects) object (with the defaults of model.md for absent fields; [Replacing the effect settings](../motion.md#replacing-the-effect-settings)). The `seed` is read exactly, as in a model. |
  | `{ "reset_physics": true }` | The host resets physics ([physics.md](../physics.md#state)). |
  | `{ "stages": { <stage>: <bool>, … } }` | The host turns stages on or off. Stages: `physics`, `eye_blink`, `breath`, `lip_sync`, `pose`. Initially all are on. |

  Gaze ([avatar-profile.md](../avatar-profile.md#gaze)) has no action: an application turns a gaze
  target into parameter values itself (the mapping is a recommendation, not an operation of the
  runtime) and writes them like any other value, which `set` covers.

  Expected results:

  | Field | Meaning |
  | --- | --- |
  | `parameters` | Parameter ID → the value passed to evaluation in this frame (the value remembered in step 10 of the frame order). |
  | `part_opacity` | Part ID → the part opacity override `{ "value", "weight" }` passed to evaluation. A weight of 0 means no override; then the value is not checked. |
  | `events` | The values of the motion events fired in this frame, in the order they were fired. Checked exactly. |
  | `offsets` | The dynamic physics offsets passed to evaluation ([evaluation.md](../evaluation.md#dynamic-physics-offsets)): `deformer_angles` (deformer ID → degrees), `deformer_origins` (deformer ID → `[x, y]`), `warp_points` (warp deformer ID → one `[x, y]` per control point), `mesh_vertices` (mesh ID → one `[x, y]` per vertex). Absent offsets are 0. |
  | `meshes` | The results of evaluating the model in this frame, as in `expected.json`. |


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

Rendering examples:

| Example | Shows |
| --- | --- |
| [render-blend](render-blend/) | Premultiplied texture colors, multiply and screen colors, opacity; the `normal`, `additive` and `multiply` blend modes over an opaque mesh and over the transparent target; overlapping triangles of one mesh; the view transform. |
| [render-masks](render-masks/) | Union and intersection, inverted masks, `mask_strength`, nested masks, masks with opacity 0 and `mask_only`, a culled mask, overlapping triangles in a mask, a mask referenced twice. |
| [render-culling](render-culling/) | Back-face culling in canvas coordinates, including under a rotation deformer that reflects and under view transforms that mirror or turn the canvas. |
| [render-lighting](render-lighting/) | Normal maps, normals turned by rotation, mirroring and shear; ambient, directional and point lights, attenuation, light above 1, invalid lights, unlit meshes; lights under view transforms that mirror or turn the canvas; invalid lights before eight valid ones, and more than eight lights. |
| [render-lighting-off](render-lighting-off/) | A model whose `lighting` is `false`: no mesh is lit. |
| [render-texture](render-texture/) | Bilinear filtering of premultiplied values; clamp-to-edge addressing with texture coordinates outside `[0, 1]`. |

## Rendering examples

A rendering example draws a model, evaluated at given parameter values, into a render target of a
given size and gives the expected colors of chosen pixels ([rendering.md](../rendering.md)).

The files of a rendering example:

- `model.json` — the model, as for evaluation examples. Its textures are given in `render.json`.
- `render.json` — the textures, the frames and the expected pixels:

  ```json
  {
    "model": "model.json",
    "summary": "What the example shows.",
    "textures": [
      {
        "width": 1, "height": 1, "rgba": [204, 204, 204, 255],
        "normal_map": { "width": 1, "height": 1, "rgba": [204, 204, 196, 255] }
      }
    ],
    "cases": [
      {
        "summary": "What the case shows.",
        "parameters": { "X": 0.5 },
        "target": [112, 16],
        "view": [1, 0, 0, 1, 0, 0],
        "lights": {
          "ambient": [0.2, 0.2, 0.2],
          "directional": [{ "direction": [-1, 0, -1], "color": [1, 1, 1] }]
        },
        "pixels": [{ "at": [4, 11], "rgba": [185.05, 185.05, 185.05, 255] }]
      }
    ]
  }
  ```

  | Field | Meaning |
  | --- | --- |
  | `model` | The model file, relative to `render.json`. |
  | `summary` | A description of the example. |
  | `textures` | One entry per texture, in the order of the model's `textures`: `width`, `height` and the color `rgba` of every pixel (row by row from the top-left; non-premultiplied, 0 to 255), and `normal_map`, the same for the texture's normal map if it has one (the model's `normal_map` names it). |
  | `cases` | Independent frames. |
  | `cases[].parameters` | Optional. Parameter values, as in evaluation examples. The model is evaluated once. |
  | `cases[].target` | The width and height of the render target in pixels. The target is cleared to transparent black before drawing. |
  | `cases[].view` | Optional. The view transform ([rendering.md](../rendering.md#inputs)) as six numbers `[a, b, c, d, e, f]`: the canvas point `(x, y)` is drawn at the target position `(a × x + c × y + e, b × x + d × y + f)` (target pixels from the top-left, `y` down). Absent: the view transform scales the canvas to the whole target (`[width / canvas.width, 0, 0, height / canvas.height, 0, 0]`); the examples then keep the canvas's aspect ratio. |
  | `cases[].lights` | Optional. The lights the host supplies ([rendering.md](../rendering.md#lights)): `ambient` (default `[1, 1, 1]`), `directional` lights and `point` lights; a point light without `range` has an infinite range. Absent: the host supplies no lights. |
  | `cases[].pixels` | The checked pixels: `at` is `[column, row]` of a target pixel (from the top-left), `rgba` its expected premultiplied color on the scale 0 to 255. |

  The expected colors are the output defined by [rendering.md](../rendering.md#conformance) (real
  numbers, clamped to `[0, 1]` after each draw), multiplied by 255 and rounded to two decimal places.
  They are not rounded to 8 bits.

To stay independent of rasterization details, the examples follow these rules:

- The center of every checked pixel is at least one target pixel away from every edge of every
  triangle of the model (of meshes that are drawn and of meshes used as masks), so it does not
  depend on how edges and shared edges are rasterized.
- Textures have a single color, except in `render-texture`, which checks filtering at pixels whose
  sampling position lies strictly between texel centers or beyond the clamped edge.
- A case with more than 8 valid lights (outside [Conformance](../rendering.md#conformance), since a
  renderer MAY use only 8 of them) is chosen so that the checked pixels are the same whichever 8 of
  its lights, or all of them, a renderer uses.

### Rendering tolerances

An implementation matches a rendering example if every channel of every checked pixel, premultiplied
and on the scale 0 to 255, is within **3** of the expected value (the tolerance of
[rendering.md](../rendering.md#conformance)). Every checked pixel has to match; the allowance for
the remaining 1% of pixels in rendering.md is for edge pixels, which the examples do not check.

### How the expected values were derived (informative)

The expected colors were computed at the checked pixels only, directly from the per-pixel equations
of [rendering.md](../rendering.md), with a separate transcription written from the text (not the
reference implementation); the blends of `render-blend` and the mask factors of `render-masks` were
also checked by hand. A small CPU rasterizer written independently from the text
renders the whole frames and agrees with them.

## File examples

The directory [bundles/](bundles/) holds small bundle (`.ugoe`) and module (`.ugoemod`) files in the
container of [bundle.md](../bundle.md):

| File | Shows |
| --- | --- |
| `single-quad.ugoe` | A model and its texture; no compression; fields with default values and fields without a value omitted. |
| `single-quad-zstd.ugoe` | The same model with a zstd-compressed `MODL` chunk; every field with a default value written, fields without a value written as nil. |
| `layout.ugoe` | Payloads stored in the reverse order of the chunk table; metadata; an unknown chunk flagged as compressed whose payload is not a zstd frame (skipped without decompressing it) and an unknown chunk of length 0. |
| `physics-pendulum.ugoe` | Physics settings; compressed `MODL` and `PHYS`. |
| `motion-crossfade.ugoe` | Two motions in table order. |
| `expressions.ugoe` | Three compressed expressions and metadata. |
| `lit-quads.ugoe` | Four textures, three with a normal map; two textures that name the same normal map, each with its own `NRML` chunk. |
| `optional-feature.ugoe` | An unknown optional feature: the file is read and the feature is reported. |
| `required-feature.ugoe` | An unknown required feature: the file is refused. |
| `short-hair.ugoemod` | A module file with a compressed `MODU` chunk, a texture and metadata. |

The directory also holds the JSON documents of the files that are not taken from the other examples
(`lit-quads.model.json`, `short-hair.module.json`), and the list of the files, `bundles.json`:

```json
{
  "summary": "What the files show.",
  "files": [
    {
      "file": "single-quad-zstd.ugoe",
      "summary": "What the file shows.",
      "style": "explicit",
      "chunks": [ { "tag": "MODL", "compressed": true }, { "tag": "TEXR" } ],
      "result": "ok",
      "model": "../single-quad/model.json",
      "textures": [ { "width": 1, "height": 1, "rgba": [255, 255, 255, 255] } ]
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `file` | The file, relative to `bundles.json`. Files ending in `.ugoemod` are module files; the others are bundles. |
| `summary` | A description of the file. |
| `style` | Informative. How the file writes fields that have their default value and nullable fields without a value: `minimal` omits them, `explicit` writes them (the value, or nil). [Authoring data](../model.md#authoring-data) with its default value is omitted in both. A reader obtains the same data from both. |
| `chunks` | The chunk table in order: `tag`, and `compressed` (bit 0 of the flags; `false` if absent). For unknown chunks, `payload` is the stored payload as a UTF-8 string. |
| `payload_order` | Optional. The order in which the payloads are stored in the file, as indices into `chunks`. Absent: table order. |
| `features` | Optional. The content of the `FEAT` chunk. |
| `result` | `ok`: a reader reads the file. `error`: a reader refuses it. |
| `error` | For `error`: the step of [Reading](../bundle.md#reading) that refuses the file (below). |
| `unknown_required_features` | For `error` `feature_declarations`: the unknown required features the error lists. |
| `warnings` | Optional. For `ok`: `unknown_features`, the unknown optional features a reader reports. Apart from these, reading the file produces no warnings. |
| `model`, `physics`, `motions`, `expressions`, `module` | The JSON documents the file is equivalent to, relative to `bundles.json`. `motions` and `expressions` are arrays in table order. For `ok`, the data a reader obtains from the file is the data it obtains from these documents. For `error`, they are the data the file carries. |
| `meta` | Optional. The JSON value of the `META` chunk. Absent: the file has no `META` chunk. |
| `textures` | One entry per texture, in the order of the model's (or module's) `textures`: the image's `width` and `height` in pixels and the color `rgba` of every pixel (non-premultiplied, 0 to 255), and `normal_map`, the same for the texture's normal map if it has one. Every image is a PNG. |

Values of `error`, named after the steps of [Reading](../bundle.md#reading):

| `error` | Step |
| --- | --- |
| `header` | 1. Header |
| `chunk_table` | 2. Chunk table |
| `feature_declarations` | 3. Feature declarations |
| `schema_version` | 4. Schema version |
| `chunks` | 5. Chunks (including payloads that fail to decompress or decode) |
| `counts` | 6. Counts |
| `validation` | 7. Validation |

The examples of this version use only `feature_declarations`. A reader that checks in another order
than the one recommended may report a different step for a file that breaks several rules; the
examples break only one.

The feature IDs used (`test.example_optional`, `test.example_required`) and the unknown chunk tags
(`Xzst`, `Xnul`) are reserved for testing ([bundle.md](../bundle.md#feature-declarations),
[bundle.md](../bundle.md#chunk-types)), so no reader knows them.

The files were written by a writer built from this specification alone, and checked to be read by
that writer's reader and by the reference implementation.

## Relation to other test data (informative)

The ugoe repository also keeps a corpus of binary files for forward-compatibility tests (outside
this specification). That corpus checks that files written by each release stay readable and that
files using unknown features degrade as specified; it has no expected evaluation results. The
examples here are part of the specification: they show how the rules evaluate and how files are
laid out, and they are not tied to any release of the reference implementation.

## License

The examples are part of the ugoe Specification and are licensed under [CC BY 4.0](../LICENSE).
If you include them in your program's test suite, keep a notice of their origin and license with
them, as described in [README.md](../README.md#license).
