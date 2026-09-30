# Examples

**ugoe Specification 0.1 — Draft.** See [../README.md](../README.md) for the status, conventions and
license of this specification.

Each directory contains a small model and the results a runtime is expected to produce for it at
given parameter values. The examples illustrate the evaluation rules and let an implementation check
itself against them, with the tolerances of [evaluation.md](../evaluation.md#conformance). They are
**informative**: where an example and the text disagree, the text is
correct (and the example is a bug in the specification).

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

## Files

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

  No physics, motions, effects or tracking run in the examples: the parameter values are exactly
  those of the case.

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
