# ugoe Model Data

**ugoe Specification 0.1 — Draft.** See [README.md](README.md) for the status, conventions and
license of this specification.

This document defines the data of a ugoe model: every type and field, their default values, how
they are encoded in JSON and MessagePack, and the rules that a valid model satisfies. It also
defines the data that travels with a model (physics settings, motions, expressions) and the data of
dress-up modules. How the data is packed into files is defined in [bundle.md](bundle.md).

This document states what each field *means* only as far as needed to produce and consume the data
correctly. The exact computations belong to [evaluation.md](evaluation.md) (interpolation and
deformation), [rendering.md](rendering.md), [physics.md](physics.md), [motion.md](motion.md)
(playback, effects and the order of operations in a frame) and [avatar-profile.md](avatar-profile.md).

## Contents

- [Data conventions](#data-conventions)
- [Coordinate system](#coordinate-system)
- [Identifiers](#identifiers)
- [Model](#model)
- [Parameters](#parameters)
- [Parts](#parts)
- [Keyform grids](#keyform-grids)
- [Deformers](#deformers)
- [Meshes](#meshes)
- [Glues](#glues)
- [Textures](#textures)
- [Hit areas](#hit-areas)
- [Pose](#pose)
- [Effects](#effects)
- [Avatar profile](#avatar-profile)
- [Slots](#slots)
- [Authoring data](#authoring-data)
- [Physics settings](#physics-settings)
- [Motions](#motions)
- [Expressions](#expressions)
- [Modules](#modules)
- [Unknown values](#unknown-values)
- [Validation](#validation)

## Data conventions

### Types

Field tables in this document use the following types.

| Type | JSON | MessagePack |
| --- | --- | --- |
| `string` | string | str (UTF-8) |
| `bool` | `true` / `false` | bool |
| `u8`, `u16`, `u32`, `u64` | integer number in the type's range | any MessagePack integer whose value is in the type's range |
| `f32` | number | float 32, float 64 or any integer |
| `vec2` | array of two `f32`: `[x, y]` | array of two `f32` |
| `rgb` | object `{ "r": f32, "g": f32, "b": f32 }` (all three fields required) | map with the same keys |
| `id` | string | str |
| `T[]` | array of `T` | array of `T` |
| `T?` | `T` or `null` | `T` or nil |
| object types (for example `Part`) | object | map with string keys |

- `f32` values are IEEE 754 single-precision numbers. A reader converts a decoded number to the
  nearest `f32`. Writers SHOULD write `f32` values so that they convert back to the same `f32`
  (for example, the shortest decimal representation that round-trips in JSON, and float 32 in
  MessagePack).
- Unless a field says otherwise, `f32` values MUST be finite. JSON cannot represent NaN or
  infinity (a JSON number too large for `f32`, such as `1e39`, converts to infinity). The
  [validation rules](#validation) check every `f32` of a model, of physics settings, of motions and
  of expressions. The `f32` values of the avatar profile are checked by its
  [conformance checks](avatar-profile.md#conformance-checks) instead.
- Integer fields MUST be integers in the range of their type. A reader MUST reject a float where an
  integer is required.
  - In MessagePack, an integer is a value of one of the integer types; a float 32 or float 64 is a
    float even when its value is integral.
  - In JSON, writers MUST write integers without a fraction or an exponent (`1`, not `1.0` or
    `1e0`). A reader MUST reject a number that is not integral, and SHOULD also reject an integral
    number written with a fraction or an exponent. *Informative:* the reference reader rejects
    `1.0`; JSON parsers that do not keep the written form of numbers (for example `JSON.parse` in
    JavaScript) cannot tell `1.0` from `1`, and such readers accept both.
- `rgb` components are non-linear (sRGB-encoded) values, nominally 0 to 1.

### Presence and default values

Each field has one of these presences.

| Presence | Reader | Writer |
| --- | --- | --- |
| required | The field MUST be present; otherwise the payload is invalid. | MUST write the field. |
| optional, default *d* | An absent field has the value *d*. | MAY omit the field when its value is *d*; otherwise MUST write it. |
| nullable (`T?`) | An absent field and `null` both mean "no value". | MAY write `null` or omit the field when there is no value. |

- A writer MUST NOT write `null` for a field whose type is not nullable. A reader MUST reject such a
  `null` (it is a value of the wrong type), also for an optional field: `null` does not mean "use
  the default".
- The column "Default" in the field tables gives *d*; "—" means the field is required, and
  "none" means the field is nullable and has no value by default.
- *Informative:* the ugoe reference writer omits some default-valued fields (for example
  `interpolation: "linear"`) and writes others explicitly (for example `blend: "normal"`), and it
  writes `null` for most nullable fields. Readers must accept every combination that this table
  allows.

### Objects

- Objects are unordered. A reader MUST accept the fields of an object in any order.
- A writer MUST NOT write the same field twice in one object. A reader MUST reject an object in which
  a field that it knows appears more than once; it MAY also reject duplicates of fields it does not
  know.
- A reader MUST ignore fields it does not know. This is how new optional data is added without
  breaking older readers ([bundle.md](bundle.md#unknown-fields)).
- In MessagePack, every object is a map whose keys are the field names as strings (never an array
  of values in declaration order).

### Enumerations

Enumerations are encoded in one of four ways.

| Form | Encoding | Used by |
| --- | --- | --- |
| String | A string naming the value, for example `"multiply"`. | most enumerations |
| Tagged object | An object with a tag field naming the variant and the variant's own fields beside it, for example `{ "type": "rotation", "keyforms": … }`. The tag field is `type`, or `method` for [keyform inference](#keyform-grid-authoring-fields). A variant without fields is an object with only the tag. | deformer kinds, physics output targets, collider shapes, motion segments, keyform inference |
| Kind and id | An object `{ "kind": <string>, "id": <string> }`. | [curve targets](#motions) |
| Single-key object | An object with exactly one field, whose name is the variant and whose value is the variant's content, for example `{ "mesh": "Skirt" }`. | [vertex owners](#physics-output-targets) |

Most enumerations are **extensible**: a later version of the format may add values. A reader MUST
read a value it does not know as an *unknown value* instead of failing, and apply the fallback
defined for that enumeration ([Unknown values](#unknown-values)). Enumerations that are not
extensible say so.

### Arrays and order

The order of elements in an array is significant only where this document says so. In particular:
the order of `meshes` breaks ties in drawing order; keyforms, keys, vertices, indices, control
points and chunk-ordered images are positional; the order of physics rigs is their evaluation
order. The order of other arrays (parameters, parts, deformers, …) carries no meaning for a
runtime; tools SHOULD preserve it because it is the order in which editors display elements.

## Coordinate system

- **Canvas coordinates** are pixels with the origin at the top-left of the canvas, x to the right
  and y **down**.
- Angles are in degrees. A positive angle rotates **clockwise** as seen on screen (because y points
  down).
- Every deformer defines a *local coordinate system* for its children:
  - An element without a parent deformer is expressed in canvas coordinates.
  - The children of a **warp deformer** are expressed in the deformer's normalized lattice space:
    `(0, 0)` is the first control point and `(1, 1)` the last ([Warp deformers](#warp-deformers)).
    Values outside `[0, 1]` are allowed.
  - The children of a **rotation deformer** are expressed in pixels relative to the deformer's
    origin, before its rotation, scale and reflection are applied.
- Texture coordinates (`uvs`) range from 0 to 1 with the origin at the **top-left** of the image.
- Time values are in seconds.

## Identifiers

- Elements are referred to by string **IDs**. An ID MUST be unique among the elements of its kind
  within one model: parameters, parameter groups, parts, deformers, meshes, glues, hit areas and
  slots are separate kinds, so a part and a mesh may share an ID. Physics rigs and physics colliders
  are also unique within their physics settings.
- Any string is a valid ID. IDs SHOULD match `[A-Za-z0-9_]+`. In particular, IDs in a model SHOULD
  NOT contain `/` and SHOULD NOT start with `#`: [module composition](#composition) generates IDs of
  the forms `<module>/<id>` and `#replaced/<id>`.
- IDs are compared as exact strings (case-sensitive, no normalization).
- A reference to an ID that does not exist makes a model invalid ([Validation](#validation)),
  unless the field says otherwise.

## Model

A model is one object. In a bundle it is the payload of the `MODL` chunk
([bundle.md](bundle.md#chunk-types)). As a standalone JSON document it is conventionally named
`model.json`.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `schema_version` | `u32` | — | Version of the data schema. This draft is version 0 ([Versions](README.md#versions)). |
| `meta` | [`ModelMeta`](#model-metadata) | — | Name and canvas. |
| `parameters` | [`Parameter`](#parameters)`[]` | — | Parameter definitions. |
| `parameter_groups` | [`ParameterGroup`](#parameter-groups)`[]` | `[]` | Groups for displaying parameters in tools. No effect on evaluation. |
| `parts` | [`Part`](#parts)`[]` | — | The part tree. |
| `deformers` | [`Deformer`](#deformers)`[]` | `[]` | The deformer tree. |
| `meshes` | [`Mesh`](#meshes)`[]` | — | Meshes. Their order breaks ties in drawing order. |
| `glues` | [`Glue`](#glues)`[]` | `[]` | Glues. |
| `textures` | [`TextureRef`](#textures)`[]` | — | Texture references. The images themselves are stored by the container. |
| `hit_areas` | [`HitArea`](#hit-areas)`[]` | `[]` | Hit areas. |
| `pose` | [`Pose`](#pose) | see [Pose](#pose) | Mutually exclusive part groups. |
| `effects` | [`Effects`](#effects) | see [Effects](#effects) | Eye blink, breathing and lip sync settings. |
| `avatar` | [`AvatarProfile`](#avatar-profile)`?` | none | Avatar profile. |
| `slots` | [`Slot`](#slots)`[]` | `[]` | Dress-up slots. |
| `lighting` | `bool` | `false` | Whether the model is lit by lights supplied by the host ([Textures](#normal-maps-and-lighting)). |

Physics settings, motions and expressions are separate objects. A bundle carries them in their own
chunks ([bundle.md](bundle.md#chunk-types)).

### Model metadata

`ModelMeta`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | — | Display name of the model. |
| `canvas` | `Canvas` | — | The canvas. |

`Canvas`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `width` | `f32` | — | Canvas width in pixels. |
| `height` | `f32` | — | Canvas height in pixels. |
| `origin` | `vec2` | — | Anchor point used when displaying the model (for example, between the feet), in canvas coordinates. How a host positions and scales the canvas on screen is up to the host. |

## Parameters

A parameter is a named scalar input.

`Parameter`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Parameter ID. |
| `name` | `string` | — | Display name. |
| `min` | `f32` | — | Minimum value. |
| `max` | `f32` | — | Maximum value. |
| `default` | `f32` | — | Default value. `min <= default <= max`. |
| `kind` | `ParameterKind` | `"normal"` | See below. |
| `mirror` | `id?` | none | Authoring data: the parameter that forms a left–right pair with this one, for example `EyeLOpen` and `EyeROpen` ([Mirror pairs](#mirror-pairs)). |
| `group` | `id?` | none | ID of a [parameter group](#parameter-groups). |
| `internal` | `bool` | `false` | Marks a relay parameter that users and applications are not expected to drive directly. |

- The value of a parameter is always clamped to `[min, max]`.
- `ParameterKind` (string, extensible):

  | Value | Meaning |
  | --- | --- |
  | `normal` | An ordinary parameter. It may be an axis of ordinary keyform grids. |
  | `blend_shape` | A blend-shape parameter. It may be an axis of [blend-shape grids](#blend-shapes) only. |

  Fallback for an unknown value: treat the parameter as `normal` for every purpose except that
  validation allows it on both kinds of grid ([Unknown values](#unknown-values)).
- `internal = true` does not change how the parameter is evaluated. It means: tracking and
  applications SHOULD NOT write to it, and user interfaces SHOULD hide it by default. It may be
  referenced from anywhere (keyform axes, physics inputs and outputs, motions). A reader that does
  not know the field treats the parameter as an ordinary one.

### Parameter groups

`ParameterGroup`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Group ID. |
| `name` | `string` | — | Display name. |

### Well-known parameter IDs

Some parameter IDs have a fixed meaning so that motions, physics presets, tracking and dress-up
modules work across models. A model is not required to define any of them, but a parameter with one
of these IDs SHOULD have the range and default below. The meaning of each ID, and which of them an
avatar must define, is specified by [avatar-profile.md](avatar-profile.md#parameters).

Conventions: left and right (`L` / `R`) are the character's own left and right. `AngleX` is positive
when the character turns to its own left (the right of the screen), `AngleY` is positive when it
looks up, and `AngleZ` is positive when the head tilts clockwise on screen. Open/close parameters
are 0 when closed and 1 when open. Parameters with a range of −1 to 1 are neutral at 0.

**Standard parameters:**

| ID | min | default | max |
| --- | --- | --- | --- |
| `AngleX`, `AngleY`, `AngleZ` | −30 | 0 | 30 |
| `BodyAngleX`, `BodyAngleY`, `BodyAngleZ` | −10 | 0 | 10 |
| `Breath` | 0 | 0 | 1 |
| `EyeLOpen`, `EyeROpen` | 0 | 1 | 1 |
| `EyeLSmile`, `EyeRSmile` | 0 | 0 | 1 |
| `EyeBallX`, `EyeBallY` | −1 | 0 | 1 |
| `BrowLY`, `BrowRY` | −1 | 0 | 1 |
| `MouthForm` | −1 | 0 | 1 |
| `MouthOpenY` | 0 | 0 | 1 |
| `Cheek` | 0 | 0 | 1 |
| `HairFront`, `HairSide`, `HairBack` | −1 | 0 | 1 |

**Extended parameters** (avatar profile):

| ID | min | default | max |
| --- | --- | --- | --- |
| `EyeLWide`, `EyeRWide` | 0 | 0 | 1 |
| `EyeLSmile`, `EyeRSmile` (also standard) | 0 | 0 | 1 |
| `BrowInnerUp` | 0 | 0 | 1 |
| `BrowLAngle`, `BrowRAngle` | −1 | 0 | 1 |
| `MouthX`, `JawX` | −1 | 0 | 1 |
| `MouthPucker`, `MouthFunnel`, `MouthPress`, `MouthShrug` | 0 | 0 | 1 |
| `CheekPuff`, `TongueOut`, `NoseSneer` | 0 | 0 | 1 |

**Body extended parameters** (avatar profile):

| ID | min | default | max |
| --- | --- | --- | --- |
| `ShoulderLY`, `ShoulderRY` | −1 | 0 | 1 |

**Vowel parameters** (lip sync):

| ID | min | default | max |
| --- | --- | --- | --- |
| `MouthVowelA`, `MouthVowelI`, `MouthVowelU`, `MouthVowelE`, `MouthVowelO` | 0 | 0 | 1 |

The IDs in these four tables are the **well-known parameter IDs**. Dress-up modules may always use
them ([Modules](#modules)).

## Parts

Parts form a tree that groups meshes and deformers. A part's opacity multiplies into the opacity of
all its descendant parts and of the meshes that belong to it.

`Part`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Part ID. |
| `name` | `string` | — | Display name. |
| `parent` | `id?` | none | Parent part. None for a root part. |
| `keyforms` | [`KeyformGrid`](#keyform-grids)`<PartKeyform>` | — | The part's keyforms. |

`PartKeyform`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `opacity` | `f32` | — | Opacity of the part, nominally 0 to 1. |

- Every mesh and every deformer belongs to exactly one part (its `part` field). The part tree and
  the deformer tree are independent.
- The `part` of a deformer does not affect evaluation: a deformer's opacity is not multiplied by the
  opacity of its part, and meshes under the deformer are not affected by that part unless they belong
  to it themselves ([evaluation.md](evaluation.md#meshes)). It groups the deformer with the part in
  tools.
- The final opacity of a mesh is the product of the mesh's own opacity, the opacities of its part
  and all ancestor parts, and the opacity of its parent deformer (which already includes that
  deformer's ancestors).

## Keyform grids

A keyform grid stores the values of an element at the grid points formed by one or more parameter
axes. At run time, the values for the current parameter values are interpolated from the grid
([evaluation.md](evaluation.md#keyform-interpolation)).

`KeyformGrid<T>`, where `T` is the keyform type of the element (for example `MeshKeyform`):

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `bindings` | `KeyformBinding[]` | `[]` | The axes of the grid. |
| `keyforms` | `T[]` | — | The keyforms, one per grid point. |
| `interpolation` | `KeyformInterpolation` | `"linear"` | How values between keys are interpolated. |
| `blend_shapes` | `KeyformGrid<T>[]` | `[]` | [Blend-shape grids](#blend-shapes). |
| `origins` | `KeyformOrigin[]` | `[]` | Authoring data ([Keyform grid authoring fields](#keyform-grid-authoring-fields)). |
| `inference` | `KeyformInference?` | none | Authoring data. |
| `mirror_source` | `MirrorSource?` | none | Authoring data. |

`KeyformBinding`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameter` | `id` | — | The parameter of this axis. |
| `keys` | `f32[]` | — | The parameter values of the grid points on this axis. At least one; strictly increasing. |

- With `N` bindings of `n_0, …, n_(N−1)` keys, the grid has `n_0 × … × n_(N−1)` keyforms. With no
  bindings the grid has exactly one keyform, which is used for every parameter value.
- The keyform at grid coordinates `(i_0, i_1, …, i_(N−1))` (where `i_k` indexes the keys of axis
  `k`) is at index `i_0 + n_0 × (i_1 + n_1 × (i_2 + …))` of `keyforms`: **axis 0 varies fastest**.
- A parameter MUST NOT be the axis of more than one binding of the same grid.
- `KeyformInterpolation` (string, extensible):

  | Value | Meaning |
  | --- | --- |
  | `linear` | Multilinear interpolation between neighbouring keys. |
  | `smooth` | On axes with three or more keys, a smooth curve through the keys (cubic Hermite). Affects only shape values (vertex positions, control points, and the origin, angle and scale of rotation deformers); other values stay linear. Grids of parts and glues are always linear. |

  Fallback for an unknown value: `linear`.

*Informative example* — a mesh grid on one axis with three keys:

```json
"keyforms": {
  "bindings": [{ "parameter": "AngleX", "keys": [-30, 0, 30] }],
  "keyforms": [ { "positions": [], "opacity": 1, "draw_order": 500 }, { … }, { … } ],
  "interpolation": "smooth"
}
```

### Blend shapes

A **blend-shape grid** is a keyform grid in the `blend_shapes` array of another grid (the *normal
grid*). Its axes are `blend_shape` parameters. At run time, the difference between the blend-shape
grid's value at the current parameter values and its value at the parameters' default values is
added to the normal grid's result, one blend-shape grid after another in array order.

- Blend-shape grids are allowed on meshes, warp deformers and rotation deformers. Grids of parts
  and glues MUST NOT have `blend_shapes`.
- A blend-shape grid holds complete keyforms (not differences), in the same coordinate system as
  the normal grid. Placing the default-shape keyform at the default key therefore makes every other
  keyform an ordinary target shape. Because only the difference from the default is added, a model
  looks the same as without blend shapes when all blend-shape parameters are at their defaults.
- Every field of the keyform that is a number is affected; boolean fields (`reflect_x`,
  `reflect_y`) always come from the normal grid.
- A blend-shape grid MUST have at least one binding and MUST NOT have `blend_shapes` of its own.
- The axes of a normal grid MUST NOT be `blend_shape` parameters. The axes of a blend-shape grid
  MUST NOT be `normal` parameters. (Parameters of an unknown kind may be used on either.)
- The same parameter MUST NOT be an axis of two blend-shape grids of the same element. A
  blend-shape grid may have several axes (for example, to correct the combination of two blend
  shapes).
- A blend-shape grid may have its own `interpolation`.
- Fallback for readers that do not support blend shapes: ignore `blend_shapes` (feature
  `deform.blend_shape`, [bundle.md](bundle.md#baseline-features)).

```json
"keyforms": {
  "bindings": [{ "parameter": "AngleX", "keys": [-30, 0, 30] }],
  "keyforms": [ … ],
  "blend_shapes": [
    { "bindings": [{ "parameter": "mouthSmileLeft", "keys": [0, 1] }], "keyforms": [ … , … ] },
    { "bindings": [{ "parameter": "jawOpen", "keys": [0, 1] }], "keyforms": [ … , … ] }
  ]
}
```

## Deformers

Deformers form a tree. A deformer transforms the coordinates of its children (child deformers and
meshes) from its local coordinate system into its parent's.

`Deformer`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Deformer ID. |
| `name` | `string` | — | Display name. |
| `part` | `id` | — | The part the deformer belongs to. |
| `parent` | `id?` | none | Parent deformer. None: the deformer is expressed in canvas coordinates. |
| `kind` | `DeformerKind` | — | The kind and its data. |
| `mirror` | `id?` | none | Authoring data ([Mirror pairs](#mirror-pairs)). |
| `mirror_axis` | `f32?` | none | Authoring data ([Mirror pairs](#mirror-pairs)). |

`DeformerKind` is a tagged object with the tag field `type` (extensible):

| `type` | Further fields |
| --- | --- |
| `warp` | [`WarpDeformer`](#warp-deformers) |
| `rotation` | [`RotationDeformer`](#rotation-deformers) |

Fallback for an unknown kind: the deformer is the identity transform with opacity 1 (its children
receive only the transform of the deformer's parent).

*Informative example:*

```json
{ "id": "Head", "name": "Head", "part": "Head", "parent": "Body",
  "kind": { "type": "rotation", "keyforms": { "bindings": [], "keyforms": [
    { "origin": [0.5, 0.25], "angle": 0, "scale": 1, "opacity": 1 } ] } } }
```

### Warp deformers

A warp deformer is a lattice of `(rows + 1) × (cols + 1)` control points.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `rows` | `u16` | — | Number of cells vertically. At least 1. |
| `cols` | `u16` | — | Number of cells horizontally. At least 1. |
| `keyforms` | `KeyformGrid<WarpKeyform>` | — | Keyforms. |
| `depth` | `f32[]?` | none | Authoring data ([Depth](#depth)). |

`WarpKeyform`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `points` | `vec2[]` | — | Control points in the parent's local coordinates. Length `(rows + 1) × (cols + 1)`. |
| `opacity` | `f32` | — | Opacity, multiplied into all descendants. |

- Control points are stored row by row with x varying fastest: the point in column `x`
  (`0 ≤ x ≤ cols`) and row `y` (`0 ≤ y ≤ rows`) is at index `x + (cols + 1) × y`.
- A child at local coordinates `(u, v)` is mapped through the lattice: `u = 0` is column 0 and
  `u = 1` is column `cols`; `v = 0` is row 0 and `v = 1` is row `rows`. Inside a cell the mapping is
  bilinear; outside `[0, 1]²` it is extrapolated ([evaluation.md](evaluation.md#warp-deformers)).

### Rotation deformers

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `keyforms` | `KeyformGrid<RotationKeyform>` | — | Keyforms. |

`RotationKeyform`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `origin` | `vec2` | — | Origin in the parent's local coordinates. |
| `angle` | `f32` | — | Rotation in degrees, clockwise positive. |
| `scale` | `f32` | — | Uniform scale. |
| `reflect_x` | `bool` | `false` | Mirror the children horizontally (negate their local x). |
| `reflect_y` | `bool` | `false` | Mirror the children vertically (negate their local y). |
| `opacity` | `f32` | — | Opacity, multiplied into all descendants. |

A child at local coordinates `p` (pixels relative to the origin) is placed at
`origin + R(angle) · S · p`, where `S` applies `scale` and the reflections. When the parent is a warp
deformer, only the origin and the direction are taken through the lattice, so the children are not
stretched by it ([evaluation.md](evaluation.md#rotation-deformers)).

## Meshes

A mesh is a textured triangle mesh, the unit of drawing.

`Mesh`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Mesh ID. |
| `name` | `string` | — | Display name. |
| `part` | `id` | — | The part the mesh belongs to. |
| `parent` | `id?` | none | Parent deformer. None: the vertices are in canvas coordinates. |
| `texture` | `u32` | — | Index into the model's `textures`. |
| `uvs` | `vec2[]` | — | Texture coordinates, one per vertex. Their number is the vertex count. |
| `indices` | `u16[]` | — | Triangle list: each three consecutive indices form a triangle. |
| `blend` | `BlendMode` | `"normal"` | Blend mode. |
| `culling` | `bool` | `false` | Back-face culling. Triangles whose vertices run counter-clockwise in canvas coordinates (as seen on screen, y down) face the front. |
| `masks` | `MaskRef[]` | `[]` | Clipping masks. |
| `mask_mode` | `MaskMode` | `"union"` | How the non-inverted masks combine. |
| `mask_only` | `bool` | `false` | If `true`, the mesh is not drawn in color and only acts as a mask. |
| `keyforms` | `KeyformGrid<MeshKeyform>` | — | Keyforms. |
| `mirror` | `id?` | none | Authoring data ([Mirror pairs](#mirror-pairs)). |
| `mirror_axis` | `f32?` | none | Authoring data ([Mirror pairs](#mirror-pairs)). |
| `depth` | `f32[]?` | none | Authoring data ([Depth](#depth)). |
| `lit` | `bool` | `true` | Whether the mesh receives lighting when the model's `lighting` is `true` ([Normal maps and lighting](#normal-maps-and-lighting)). |

`MeshKeyform`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `positions` | `vec2[]` | — | Vertex positions in the parent's local coordinates. Same length as `uvs`. |
| `opacity` | `f32` | — | Opacity of the mesh itself, nominally 0 to 1. |
| `draw_order` | `f32` | — | Drawing order; larger values are drawn in front. By convention 0 to 1000, with 500 as the usual value. |
| `multiply_color` | `rgb` | `{r: 1, g: 1, b: 1}` | Color multiplied into the texture color. |
| `screen_color` | `rgb` | `{r: 0, g: 0, b: 0}` | Color screen-blended onto the texture color. |
| `mask_strength` | `f32` | `1` | How strongly the masks apply, 0 to 1. Values outside the range are allowed and are clamped after interpolation. No meaning for a mesh without masks. |

- The vertex count and the topology (`uvs`, `indices`) are shared by all keyforms; keyforms hold
  positions only. Because indices are `u16`, a mesh addresses at most 65,536 vertices.
- `uvs` outside `[0, 1]` are allowed.
- When two meshes have the same `draw_order`, the one later in the model's `meshes` array is drawn in
  front.
- `BlendMode` (string, extensible): `normal`, `additive`, `multiply`. Fallback for an unknown value:
  `normal`. The blend equations are defined in [rendering.md](rendering.md#blend-modes).

### Masks

`MaskRef`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `mesh` | `id` | — | The mesh whose coverage is used as a mask. |
| `invert` | `bool` | `false` | If `true`, the mask hides the mesh where the mask mesh covers it (inverted mask). |

- A mesh is visible where its non-inverted masks cover it (combined according to `mask_mode`) and
  where none of its inverted masks cover it. Masks are evaluated with their own masks applied
  (nesting). The exact coverage equations are defined in [rendering.md](rendering.md#masks).
- `MaskMode` (string, extensible): `union` (visible where any non-inverted mask covers) or
  `intersection` (visible where all non-inverted masks cover). Fallback for an unknown value:
  `union`.
- Mask references MUST NOT form a cycle (including a mesh masking itself).

## Glues

A glue pulls pairs of vertices of two meshes towards each other to hide seams.

`Glue`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Glue ID. |
| `mesh_a` | `id` | — | First mesh. |
| `mesh_b` | `id` | — | Second mesh. |
| `pairs` | `GluePair[]` | — | Vertex pairs. |
| `keyforms` | `KeyformGrid<GlueKeyform>` | — | Keyforms. |

`GluePair`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `a` | `u16` | — | Vertex index in `mesh_a`. |
| `b` | `u16` | — | Vertex index in `mesh_b`. |
| `weight` | `f32` | — | Where the pair meets: 0 at vertex `a`, 1 at vertex `b`. |

`GlueKeyform`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `intensity` | `f32` | — | How strongly the vertices are pulled together, 0 to 1. |

## Textures

`TextureRef`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | — | Name of the texture. |
| `width` | `u32` | — | Width of the image in pixels. |
| `height` | `u32` | — | Height of the image in pixels. |
| `normal_map` | `string?` | none | Name of the texture's normal map. None: no normal map. |

- The images are not part of the model. In a bundle, the `i`-th texture's image is the `i`-th
  `TEXR` chunk and normal maps are `NRML` chunks ([bundle.md](bundle.md#textures-and-normal-maps)).
- Texture images are non-premultiplied RGBA.
- `width` and `height` describe the image; a reader does not have to check them against the image.

### Normal maps and lighting

- A normal map is an image in the same UV space as its texture that stores tangent-space normals.
  Its size may differ from the texture's. A texture without a normal map has the normal `(0, 0, 1)`
  everywhere. The encoding of normals and the lighting equations are defined in
  [rendering.md](rendering.md#lighting).
- A mesh is lit only if both the model's `lighting` and the mesh's `lit` are `true`.
- Lights are not part of the model: the host supplies them at run time.
- Fallback for readers that do not support lighting: draw without lighting (feature
  `lighting.normal_map`, [bundle.md](bundle.md#baseline-features)).

## Hit areas

`HitArea`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Hit area ID. |
| `mesh` | `id` | — | The mesh whose deformed shape is tested. |

## Pose

The pose groups mutually exclusive parts (for example, alternative drawings of an arm). In each
group, one entry is shown at a time; switching cross-fades.

`Pose`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `fade_duration` | `f32` | `0.5` | Cross-fade time in seconds when the shown entry changes. At least 0. |
| `groups` | `PoseGroup[]` | `[]` | Groups. With no groups the pose does nothing. |

When the `pose` field is absent, the pose is `{ "fade_duration": 0.5, "groups": [] }`.

`PoseGroup`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `entries` | `PoseEntry[]` | — | The mutually exclusive entries. The first entry is shown initially. |

`PoseEntry`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `part` | `id` | — | The part of this entry. |
| `links` | `id[]` | `[]` | Parts that follow the opacity of `part` (for example, a sleeve that belongs to an arm). |

- A part MUST NOT appear more than once in the whole pose (counting every `part` and every entry of
  `links` in every group).

## Effects

Settings of the automatic effects. When the `effects` field or any of its fields is absent, the
defaults below apply; the default targets no parameters and does nothing.

`Effects`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `eye_blink` | `EyeBlink` | see below | Eye blink. |
| `breath` | `Breath` | `{ "parameters": [] }` | Breathing. |
| `lip_sync` | `LipSync` | `{ "parameters": [] }` | Lip sync. |

`EyeBlink` — moves its parameters from 1 to 0 and back at random intervals:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameters` | `id[]` | `[]` | Target parameters. |
| `interval` | `f32` | `4.0` | Mean time between blinks, in seconds. Greater than 0. |
| `closing` | `f32` | `0.1` | Time to close, in seconds. At least 0. |
| `closed` | `f32` | `0.05` | Time the eyes stay closed, in seconds. At least 0. |
| `opening` | `f32` | `0.15` | Time to open, in seconds. At least 0. |
| `seed` | `u64` | `0` | Seed of the deterministic random sequence of intervals. |

*Informative:* `seed` may exceed 2⁵³; JSON readers in languages whose numbers are doubles must take
care to read it exactly.

`Breath`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameters` | `BreathParameter[]` | `[]` | Breathing waves. |

`BreathParameter` — adds `weight × (offset + peak × sin(2π t / cycle))` to its parameter:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameter` | `id` | — | Target parameter. |
| `offset` | `f32` | — | Center of the wave. |
| `peak` | `f32` | — | Amplitude of the wave. |
| `cycle` | `f32` | — | Period in seconds. Greater than 0. |
| `weight` | `f32` | — | Weight applied to the wave. |

`LipSync` — writes an externally supplied volume (0 to 1) to its parameters:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameters` | `id[]` | `[]` | Target parameters. |

Motions control the strength of the eye blink and lip sync through the *effect groups* named
`EyeBlink` and `LipSync` ([Motions](#motions)). The exact behavior of the effects is defined in
[motion.md](motion.md#effects).

## Avatar profile

The avatar profile (`Model.avatar`) tells applications how to drive the model as an avatar: which
conformance level it claims, its license metadata, and adjustments to the standard tracking
mapping. This section defines the data. The meaning of the conformance levels, the tracking
mappings and the rules for applications are specified by [avatar-profile.md](avatar-profile.md).

Readers that do not use avatar profiles MAY ignore the `avatar` field entirely (feature
`avatar.profile`, [bundle.md](bundle.md#baseline-features)). Accordingly, the
[validation rules](#validation) of a model do not check the avatar profile.

`AvatarProfile` (every field is optional):

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `profile_version` | `u32` | `0` | Version of the avatar profile the model follows. |
| `level` | `u8` | `1` | Conformance level the model claims (1 to 3 in profile version 0). |
| `meta` | `AvatarMeta` | see below | Metadata and license terms. |
| `tracking` | `TrackingOverride[]` | `[]` | Overrides and additions to the standard tracking mapping. |
| `look_at` | `LookAt?` | none | Parameters for gaze following. None: use `EyeBallX` / `EyeBallY` with a range of 30 degrees if the model has them. |
| `hit_areas` | `AvatarHitArea[]` | `[]` | Mapping from standard hit area names to the model's hit areas. |
| `slots` | `id[]` | `[]` | IDs of the model's [slots](#slots) exposed to applications. |

`AvatarMeta` (every field is optional; the license terms default to their most restrictive value):

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | `""` | Name of the avatar. Required for conformance to the avatar profile. |
| `version` | `string?` | none | Version of the avatar. |
| `authors` | `string[]` | `[]` | Authors. At least one is required for conformance to the avatar profile. |
| `copyright` | `string?` | none | Copyright notice. |
| `contact` | `string?` | none | Contact information. |
| `references` | `string[]` | `[]` | Original works or references. |
| `thumbnail` | `u32?` | none | Index into the model's `textures` of a thumbnail image. |
| `license_url` | `string?` | none | URL of the terms of use. |
| `avatar_permission` | `AvatarPermission` | `"only_author"` | Who may use the model as an avatar. |
| `commercial_usage` | `CommercialUsage` | `"personal_non_profit"` | Permitted commercial use. |
| `allow_violent_usage` | `bool` | `false` | Use in excessively violent content. |
| `allow_sexual_usage` | `bool` | `false` | Use in excessively sexual content. |
| `allow_political_or_religious_usage` | `bool` | `false` | Use for political or religious purposes. |
| `allow_antisocial_or_hate_usage` | `bool` | `false` | Use for antisocial or hateful purposes. |
| `credit_notation` | `CreditNotation` | `"required"` | Whether credit is required. |
| `allow_redistribution` | `bool` | `false` | Redistribution of the model. |
| `modification` | `Modification` | `"prohibited"` | Permitted modification. |
| `third_party_licenses` | `string?` | none | License notices of third-party material. |

License enumerations (string, extensible). The fallback for an unknown value is the most
restrictive value, listed first:

| Enumeration | Values |
| --- | --- |
| `AvatarPermission` | `only_author`, `only_licensed`, `everyone` |
| `CommercialUsage` | `personal_non_profit`, `personal_profit`, `corporation` |
| `CreditNotation` | `required`, `unnecessary` |
| `Modification` | `prohibited`, `allow`, `allow_redistribution` |

`TrackingOverride` — writes `input value × scale + offset` to a parameter:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `source` | `TrackingSource` | — | Kind of input. |
| `input` | `string` | — | Name of the input; its meaning depends on `source`. |
| `parameter` | `id?` | none | Parameter to write. May be omitted only when `source` is `standard`, in which case it is the same as `input`. |
| `scale` | `f32` | `1` | Coefficient. |
| `offset` | `f32` | `0` | Offset. |

`TrackingSource` (string, extensible; fallback for an unknown value: ignore the override):

| Value | `input` |
| --- | --- |
| `standard` | A parameter ID from the standard tracking mapping; the input is the value that mapping computes. |
| `blend_shape` | The name of an ARKit face blend shape (for example `jawOpen`); the input is 0 to 1. |
| `head` | `yaw`, `pitch` or `roll`: the head rotation in degrees. |

`LookAt` (every field is optional):

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `x` | `id` | `"EyeBallX"` | Horizontal gaze parameter (positive: the character's left). |
| `y` | `id` | `"EyeBallY"` | Vertical gaze parameter (positive: up). |
| `range_x` | `f32` | `30` | Horizontal range in degrees: a target this far to the side moves `x` to the end of its range. Greater than 0. |
| `range_y` | `f32` | `30` | Vertical range in degrees. Greater than 0. |

`AvatarHitArea`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | — | Standard hit area name: `Head` or `Body`. |
| `hit_area` | `id` | — | ID of a hit area of the model. |

## Slots

A slot is a named attachment point for dress-up [modules](#modules). A slot holds at most one module
at a time.

`Slot`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `id` | — | Slot name (for example `Hair`, `Outfit`). Modules refer to the slot by this name only. |
| `anchor` | `id?` | none | Deformer the module is attached to. None: the canvas. |
| `draw_order` | `DrawOrderRange` | — | Range of drawing order for the module's meshes. |
| `replaces` | `id[]` | `[]` | Parts of the model hidden while a module is attached (for example, the default hair). |
| `parameters` | `id[]` | `[]` | Parameters of the model, besides the well-known ones, that a module in this slot may use. |
| `mask_sources` | `id[]` | `[]` | Meshes of the model that a module in this slot may use as masks. |
| `part` | `id?` | none | Part of the model that becomes the parent of the module's root parts. None: the module's root parts stay roots. |
| `colliders` | `string[]` | `[]` | IDs of physics colliders of the model that the module's physics rigs may collide with. |
| `pose_parts` | `id[]` | `[]` | Parts of pose entries of the model that the module may link its own parts to. |

`DrawOrderRange`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `min` | `f32` | — | Lower end. |
| `max` | `f32` | — | Upper end. `min <= max`. |

Fallback for readers that do not support dress-up: ignore the slots and show the model alone
(feature `modules`, [bundle.md](bundle.md#baseline-features)).

## Authoring data

The fields in this section record how keyforms were made, so that authoring tools can redo the work.
They are the fields marked "Authoring data" in the field tables. **They do not affect evaluation,
playback, physics or rendering**: a runtime MAY ignore them, and a tool that does not understand them
MAY drop them. Every one of them is optional or nullable. A writer SHOULD omit an optional one when it
has its default value; a nullable one without a value is written as `null` or omitted, as any
nullable field ([Presence and default values](#presence-and-default-values)).

### Keyform grid authoring fields

| Field | Description |
| --- | --- |
| `origins` | The origin of each keyform, in the same order as `keyforms`. Empty (default) means every keyform is `authored`; otherwise its length MUST equal the number of keyforms. |
| `inference` | The method used to rebuild keyforms whose origin is `derived`. None means `additive`. |
| `mirror_source` | Where the keyforms with origin `mirror` were mirrored from. |

`KeyformOrigin` (string, extensible; fallback for an unknown value: `authored`):

| Value | Meaning |
| --- | --- |
| `authored` | Made or edited by a person. Tools do not overwrite it. |
| `derived` | Made by inference with an unrecorded method; treated as made with the grid's `inference` method. |
| `additive` | Made by additive combination of single-axis keyforms. |
| `depth_rotation` | Made by rotating the shape using per-vertex depth. |
| `mirror` | Made by mirroring another keyform; see `mirror_source`. |

`KeyformInference` is a tagged object with the tag field `method` (extensible; fallback for an unknown
value: do not rebuild the `derived` keyforms):

| `method` | Further fields |
| --- | --- |
| `additive` | none |
| `depth_rotation` | `angle_scale` (`f32`, default `1`): degrees per unit of parameter value. `pivot` (`vec2`, required): center of rotation in the element's parent coordinates. `pivot_depth` (`f32`, default `0`): depth of the center. `yaw` (`id?`): parameter for rotation about the vertical axis. `pitch` (`id?`): parameter for rotation about the horizontal axis. A parameter given as `yaw` or `pitch` MUST be an axis of the grid. |

`MirrorSource`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `string` | — | ID of the element mirrored from, of the same kind as the element that owns the grid (may be the element itself). |
| `axis` | `f32?` | none | The mirror axis `x = axis`, if chosen explicitly. |
| `tolerance` | `f32?` | none | Distance within which vertices are matched, if chosen explicitly. None: 1 canvas pixel. |

```json
"keyforms": {
  "bindings": [ … ], "keyforms": [ … ],
  "origins": ["depth_rotation", "authored", "mirror"],
  "inference": { "method": "depth_rotation", "pivot": [128, 128], "yaw": "AngleX" },
  "mirror_source": { "id": "Face" }
}
```

### Mirror pairs

- `mirror` on a mesh or deformer names the element of the same kind that is its left–right
  counterpart. None, or the element's own ID, means the element is its own counterpart (a
  symmetric element).
- Pairs are bidirectional: if `A.mirror = B` with `B ≠ A`, then `B.mirror` MUST be `A`.
- `mirror_axis` is the x coordinate of the vertical mirror axis in the element's parent coordinates.
  None: a default determined by the parent.
- `mirror` on a parameter tells a tool that mirrors keyforms which key of which parameter a key
  corresponds to. If `A.mirror = B` with `B ≠ A`, a key of `A` corresponds to the same key of `B`
  (for example `EyeLOpen` and `EyeROpen`). If `A.mirror = A`, the parameter is its own counterpart
  and a key corresponds to the negated key (for example `AngleX`: the key 30 corresponds to −30).
  None: the parameter is not changed by mirroring, and a key corresponds to the same key of the same
  parameter (for example `MouthOpenY`: the key 1 corresponds to 1). Unlike on meshes and deformers,
  where none and the element's own ID mean the same, none and the parameter's own ID differ on a
  parameter (the same key and the negated key). Pairs SHOULD be bidirectional
  (`B.mirror = A`); unlike those of meshes and deformers, validation does not check it.

### Depth

- `depth` on a mesh gives one depth per vertex (same length as `uvs`); on a warp deformer, one per
  control point (length `(rows + 1) × (cols + 1)`). Depths are in canvas pixels, positive towards
  the viewer, and relate to the shape of the default keyform.

## Physics settings

Physics settings are a separate object, carried by the `PHYS` chunk of a bundle
([bundle.md](bundle.md#chunk-types)) or as a standalone `*.physics.json` file. They refer to the
model's parameters, deformers and meshes by ID. The simulation itself is defined in
[physics.md](physics.md).

`PhysicsSettings`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `gravity` | `vec2` | — | Gravity direction and strength. |
| `wind` | `vec2` | — | Wind. |
| `fps` | `f32` | — | Frequency of the fixed simulation step, in hertz. |
| `rigs` | `PhysicsRig[]` | — | Rigs, evaluated in array order. |
| `colliders` | `PhysicsCollider[]` | `[]` | Colliders. |

`PhysicsRig` — a chain of particles (a pendulum) driven by parameters:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `string` | — | Rig ID. |
| `name` | `string` | — | Display name. |
| `inputs` | `PhysicsInput[]` | — | Inputs. |
| `outputs` | `PhysicsOutput[]` | — | Outputs. |
| `particles` | `PhysicsParticle[]` | — | Particles; the first one is the root. |
| `normalization` | `PhysicsNormalization` | — | Ranges used to normalize inputs. |
| `colliders` | `string[]` | `[]` | IDs of colliders this rig's particles collide with. |
| `frame` | `id?` | none | Deformer whose orientation defines the frame in which gravity and wind are interpreted. None: the canvas. |

`PhysicsInput`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameter` | `id` | — | Source parameter. |
| `source` | `PhysicsSource` | — | Which component of the root the parameter moves. |
| `weight` | `f32` | — | Weight in percent (0 to 100). |
| `reflect` | `bool` | `false` | Negate the input. |

`PhysicsSource` (string, extensible; fallback for an unknown value: ignore that input or output):
`x`, `y`, `angle`.

`PhysicsOutput`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `target` | `PhysicsTarget` | — | Where the output goes. |
| `particle` | `u32` | — | Index of the particle whose motion is output. |
| `source` | `PhysicsSource` | — | Which component of the particle is output. |
| `scale` | `f32` | — | Scale applied to the output. |
| `weight` | `f32` | — | Weight in percent (0 to 100). |
| `reflect` | `bool` | `false` | Negate the output. |
| `blend` | `PhysicsBlend` | `"overwrite"` | How an output to a parameter is combined with the parameter's current value. Not used for other targets. |

- *Legacy form:* a reader MUST also accept an output that has no `target` but has a field
  `parameter` (`id`); it means `target = { "type": "parameter", "id": <parameter> }`. If both are
  present, `target` wins. Writers MUST write `target`.
- `PhysicsBlend` (string, extensible; fallback for an unknown value: `overwrite`): `overwrite`
  (move the parameter towards the physics value) or `add` (add the physics value's deviation from
  rest to the parameter).

### Physics output targets

`PhysicsTarget` is a tagged object with the tag field `type` (extensible; fallback for an unknown
value: ignore that output):

| `type` | Further fields | Meaning |
| --- | --- | --- |
| `parameter` | `id` (`id`) | Write to a parameter. |
| `deformer_angle` | `deformer` (`id`) | Add to the angle of a rotation deformer. |
| `deformer_offset` | `deformer` (`id`) | Displace the origin of a rotation deformer. |
| `vertices` | `owner` (`VertexOwner`), `weights` (`f32[]`) | Make the vertices of a mesh, or the control points of a warp deformer, follow the particle's displacement with a weight per vertex (0 to 1). `weights` has one entry per vertex (mesh) or per control point (warp), in the same order. |

`VertexOwner` is a single-key object, `{ "mesh": <mesh id> }` or `{ "warp": <deformer id> }`, and
is extensible ([unknown values](#unknown-values)): a reader MUST read a single-key object with
another key, and a string other than `"mesh"` and `"warp"` (a later kind without content), as an
unknown value. An object with no key or more than one key, and a known key whose content is not an
`id`, make the payload invalid.

```json
{ "target": { "type": "vertices", "owner": { "warp": "SkirtWarp" }, "weights": [0, 0, 0.5, 1] },
  "particle": 1, "source": "x", "scale": 1, "weight": 100 }
```

Outputs to targets other than `parameter`, and colliders, form the feature `physics.dynamic`
([bundle.md](bundle.md#baseline-features)).

### Particles, normalization and colliders

`PhysicsParticle`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `position` | `vec2` | — | Initial position relative to the root. |
| `mobility` | `f32` | — | Mobility coefficient, 0 to 1. |
| `delay` | `f32` | — | Delay coefficient. At least 0. |
| `acceleration` | `f32` | — | Acceleration coefficient. At least 0. |
| `radius` | `f32` | — | Distance to the previous particle (segment length). At least 0. |
| `collision_radius` | `f32` | `0` | Radius used for collisions. At least 0. |

`PhysicsNormalization`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `position` | `NormalizationRange` | — | Range for position inputs. |
| `angle` | `NormalizationRange` | — | Range for angle inputs. |

`NormalizationRange`: `min`, `default`, `max` (all `f32`, required; `min <= default <= max`).

`PhysicsCollider`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `string` | — | Collider ID. |
| `name` | `string` | — | Display name. |
| `attach` | `id?` | none | Deformer the collider moves with; its shape is in that deformer's local coordinates. None: fixed to the canvas. |
| `shape` | `ColliderShape` | — | Shape. |

`ColliderShape` is a tagged object with the tag field `type` (extensible; fallback for an unknown
value: ignore the collider). Radii are in canvas pixels and at least 0.

| `type` | Further fields |
| --- | --- |
| `circle` | `center` (`vec2`), `radius` (`f32`) |
| `capsule` | `a` (`vec2`), `b` (`vec2`), `radius` (`f32`): the points within `radius` of segment `a`–`b`. |

## Motions

A motion is a time-line animation, carried by an `MOTN` chunk of a bundle or as a standalone
`*.motion.json` file. Playback is defined in [motion.md](motion.md#motion-playback).

`Motion`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | — | Name of the motion. |
| `duration` | `f32` | — | Length in seconds. |
| `looping` | `bool` | `false` | Whether playback loops. |
| `fade_in` | `f32` | `0` | Fade-in time in seconds. |
| `fade_out` | `f32` | `0` | Fade-out time in seconds. |
| `curves` | `Curve[]` | — | Curves. |
| `events` | `MotionEvent[]` | `[]` | Events. |

`Curve`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `target` | `CurveTarget` | — | What the curve animates. |
| `segments` | `Segment[]` | — | The curve. |
| `fade_in` | `f32?` | none | Fade-in time for this curve. None: the motion's `fade_in`. |
| `fade_out` | `f32?` | none | Fade-out time for this curve. None: the motion's `fade_out`. |

`CurveTarget` is an object `{ "kind": <string>, "id": <string> }` (extensible; fallback for an
unknown `kind`: ignore the curve; the `id` of an unknown kind may be any value and is ignored):

| `kind` | `id` |
| --- | --- |
| `parameter` | A parameter ID. |
| `part_opacity` | A part ID; the curve overrides the part's opacity. |
| `effect_group` | An effect group name: `EyeBlink` or `LipSync`. The curve sets the strength of that effect. Curves for unknown group names are ignored. |

`Segment` is a tagged object with the tag field `type` (extensible; fallback for an unknown value:
ignore the whole curve). `Point` is `{ "t": f32, "v": f32 }` (time in seconds, value).

| `type` | Further fields | Meaning |
| --- | --- | --- |
| `start` | `p` (`Point`) | The first point of the curve. |
| `linear` | `to` (`Point`) | Straight line to `to`. |
| `bezier` | `c1`, `c2`, `to` (`Point`) | Cubic Bézier with control points `c1`, `c2`. The control points' times SHOULD lie within the segment; times outside it are clamped into it when the curve is evaluated. |
| `stepped` | `to` (`Point`) | Holds the previous value until `to.t`. |
| `inverse_stepped` | `to` (`Point`) | Jumps to `to.v` right after the start of the segment. |

- The first segment of a curve MUST be `start`, and only the first. Each following segment starts
  at the end point of the previous one. Writers MUST follow this rule. A curve with no segments or
  whose first segment is not `start` has no value; a runtime MAY ignore a curve with a later `start`
  ([motion.md](motion.md#curves)). The times of successive points SHOULD NOT decrease.

`MotionEvent`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `t` | `f32` | — | Time in seconds. |
| `value` | `string` | — | Payload passed to the host when the event fires. |

```json
{ "name": "nod", "duration": 1.0, "fade_in": 0.2, "fade_out": 0.2,
  "curves": [ { "target": { "kind": "parameter", "id": "AngleY" },
                "segments": [ { "type": "start", "p": { "t": 0, "v": 0 } },
                              { "type": "linear", "to": { "t": 0.5, "v": -10 } },
                              { "type": "linear", "to": { "t": 1.0, "v": 0 } } ] } ] }
```

## Expressions

An expression is a set of parameter adjustments layered on top of motions, carried by an `EXPR`
chunk of a bundle or as a standalone `*.expression.json` file.

`Expression`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | — | Name of the expression. |
| `fade_in` | `f32` | `0` | Fade-in time in seconds. |
| `fade_out` | `f32` | `0` | Fade-out time in seconds. |
| `parameters` | `ExpressionParameter[]` | — | Adjustments. |

`ExpressionParameter`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `parameter` | `id` | — | Target parameter. |
| `value` | `f32` | — | Operand. |
| `op` | `ExpressionOp` | — | `add`, `multiply` or `overwrite` (string, extensible; fallback for an unknown value: ignore the item). |

## Modules

A module is a partial model (a hair style, an outfit, an accessory) that is attached to a
[slot](#slots) of a model at run time. It depends on the model only through the slot's name and
parameter IDs. A module is carried by a `.ugoemod` file ([bundle.md](bundle.md#module-files)).

`Module`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `schema_version` | `u32` | — | Same as the model's `schema_version`. |
| `name` | `string` | — | Module name. Non-empty, contains no `/`, does not start with `#`. Used to rename the module's IDs, so modules attached together need distinct names. |
| `slot` | `id` | — | Name of the slot the module attaches to. |
| `requires` | `ModuleRequirements` | all empty | Conditions on the model. |
| `parts` | `Part[]` | `[]` | Parts, as in a model. |
| `deformers` | `Deformer[]` | `[]` | Deformers, as in a model. |
| `meshes` | `Mesh[]` | `[]` | Meshes, as in a model. A mesh's `texture` indexes the module's `textures`. |
| `glues` | `Glue[]` | `[]` | Glues, as in a model. |
| `textures` | `TextureRef[]` | `[]` | Texture references of the module. |
| `physics` | `ModulePhysics` | empty | Physics rigs and colliders. Gravity, wind and frequency come from the model's physics settings. |
| `pose` | `ModulePose` | empty | Pose groups and links. |

`ModuleRequirements` (every field is optional):

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `avatar_level` | `u8?` | none | The `level` declared by the model's avatar profile must be at least this (the declared value, not the result of a conformance check). A model without an avatar profile does not satisfy the requirement. |
| `slots` | `id[]` | `[]` | Slots the model must have (besides the one attached to). |
| `parameters` | `id[]` | `[]` | Parameters the model must have. |

`ModulePhysics` (every field is optional): `rigs` (`PhysicsRig[]`, default `[]`) and `colliders`
(`PhysicsCollider[]`, default `[]`).

`ModulePose` (every field is optional):

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `groups` | `PoseGroup[]` | `[]` | Groups made of the module's parts only. |
| `links` | `PoseEntry[]` | `[]` | Links from pose entries of the model to the module's parts: `part` is a part of the model listed in the slot's `pose_parts`; `links` are parts of the module. |

### IDs and references inside a module

- IDs in a module are unique within the module. A module defines no parameters.
- A module refers to parameters by ID (keyform axes, `inference.yaw` / `pitch`, physics inputs and
  `parameter` outputs). Each such parameter MUST be a [well-known parameter ID](#well-known-parameter-ids)
  or be listed in the slot's `parameters`, and MUST exist in the model.
- Elements without a parent (deformers and meshes with no `parent`, colliders with no `attach`) are
  attached to the slot's `anchor`. **Their coordinates are therefore in the anchor's local
  coordinate system** (the lattice space of a warp, pixels from the origin of a rotation, or canvas
  pixels if the slot has no anchor).
- A mask reference names a mesh of the module or, if the module has no mesh with that ID, a mesh of
  the model listed in the slot's `mask_sources`.
- A rig's `colliders` entry names a collider of the module or, if the module has none with that ID, a
  collider of the model listed in the slot's `colliders`. If the model has no physics settings, such
  an entry names nothing ([Composition](#composition)).
- A rig's `frame`, if present, names a deformer of the module. Without `frame`, the rig uses the
  canvas.
- Drawing orders of the module's meshes are in the usual 0 to 1000 range; they are mapped into the
  slot's range when composed.

### Composition

Attaching modules to a model produces an ordinary model (and physics settings), as if the elements
had been authored in the model. A runtime that supports modules composes as follows; the result MUST
be equivalent.

1. For each module, check: its name is valid and not used by another module in the set; its slot
   exists in the model and holds no other module of the set; `requires` is satisfied; every parameter
   it uses exists in the model and is a well-known ID or listed in the slot's `parameters`. If any
   check fails, composition fails.
2. For each module, in the given order, append its elements after the model's elements (and after
   earlier modules' elements), with these changes:
   - Every ID of a part, deformer, mesh, glue, physics rig and physics collider, and every `name` of
     a texture reference, becomes `<module name>/<original>`. References to them (parents, `part`,
     masks to module meshes, `mirror`, `mirror_source.id`, glue meshes, pose entries and links, rig
     colliders, rig `frame`, collider `attach`, physics output deformers and meshes) are renamed the
     same way. Parameter IDs, and references to the model's meshes and colliders, are not renamed.
     (`normal_map` names of textures are not renamed.)
   - Root parts get the slot's `part` as parent (they stay roots if it is none). Parentless
     deformers and meshes get the slot's `anchor` as parent; parentless colliders get it as `attach`.
   - A mesh's `texture` is increased by the number of textures that precede the module's textures in
     the composed model.
   - Every `draw_order` of the module's mesh keyforms (including blend-shape grids) becomes
     `min + (max − min) × draw_order / 1000`, using the slot's `draw_order` range.
   - The module's pose groups are appended after the model's groups. For each of the module's pose
     links, the linked parts are appended to the `links` of the model's pose entry with that `part`;
     the part MUST be listed in the slot's `pose_parts` and be the `part` of an entry of the model's
     pose.
   - Physics rigs and colliders are appended after the model's. A rig's `colliders` entry that names
     a collider of the model is kept. If the model has no physics settings, there are no colliders
     of the model: such entries are removed (the rig does not collide with them), and the composed
     model's slots get empty `colliders`. An entry that names neither a collider of the module nor
     one listed in the slot's `colliders` makes composition fail, with or without physics settings.
3. For each module, in the given order, and for each part listed in the `replaces` of the slot it
   attaches to, in that order, insert a new part with ID `#replaced/<part id>`, name
   `#replaced/<part name>`, a constant opacity of 0 (one keyform, no bindings) and the part's former
   parent, and make it the part's parent. A part listed more than once (in several
   slots, or twice in one slot) gets only one new part, at its first occurrence. The new parts are
   appended at the end of the parts, after the modules' parts, in this order. This hides the part
   regardless of how its own opacity is animated. (The order does not change the result of
   evaluation, but it fixes the composed model.)
4. The composed model MUST satisfy the [validation rules](#validation).

The elements of the model keep their positions, so the indices of the model's parts, deformers and
meshes are the same before and after composition.

*Informative:* the order of the modules is chosen by the host. When a module is attached to a model
that already has modules, the reference runtime appends it to the end of the list, unless a module
on the same slot or with the same name is already attached; then the new module takes that module's
place in the list.

*Informative:* when the model has no physics settings but a module has rigs, the reference runtime
composes them into settings with gravity `(0, 1)`, no wind and 60 Hz.

## Unknown values

A value of an extensible enumeration that a reader does not know is an **unknown value**. It comes
from a file written for a later version of the format.

- A reader MUST NOT fail because of an unknown value. It reads the value as unknown, discards any
  content attached to it (for example the other fields of a tagged object with an unknown tag), and
  applies the fallback below.
- Validation does not report unknown values as errors. A reader SHOULD report where they occurred
  (as warnings).
- A writer MUST NOT write an unknown value, and MUST NOT silently replace it with another value
  (for example its fallback). A tool that re-saves data containing unknown values SHOULD ask the
  user before replacing them ([bundle.md](bundle.md#re-saving)).

| Where | Enumeration | Fallback |
| --- | --- | --- |
| `Mesh.blend` | `BlendMode` | Draw with the `normal` blend mode. |
| `Mesh.mask_mode` | `MaskMode` | `union`. |
| `Deformer.kind` | `DeformerKind` | Identity transform with opacity 1 (children receive only the transform of the deformer's parent). |
| `Parameter.kind` | `ParameterKind` | Treat as `normal`; validation allows the parameter as an axis of both normal and blend-shape grids. |
| `KeyformGrid.interpolation` | `KeyformInterpolation` | `linear`. |
| elements of `KeyformGrid.origins` | `KeyformOrigin` | `authored` (tools do not overwrite the keyform). |
| `KeyformGrid.inference` | `KeyformInference` | Tools do not rebuild the `derived` keyforms. |
| `AvatarMeta` license terms | `AvatarPermission`, `CommercialUsage`, `CreditNotation`, `Modification` | The most restrictive value: `only_author`, `personal_non_profit`, `required`, `prohibited`. |
| `TrackingOverride.source` | `TrackingSource` | Ignore the override. |
| `PhysicsInput.source`, `PhysicsOutput.source` | `PhysicsSource` | Ignore the input or output. |
| `PhysicsOutput.target` | `PhysicsTarget` | Ignore the output. |
| `owner` of a `vertices` target | `VertexOwner` | Ignore the output. |
| `PhysicsOutput.blend` | `PhysicsBlend` | `overwrite`. |
| `PhysicsCollider.shape` | `ColliderShape` | Ignore the collider. |
| `Curve.target` | `CurveTarget` | Ignore the curve. |
| elements of `Curve.segments` | `Segment` | Ignore the whole curve. |
| `ExpressionParameter.op` | `ExpressionOp` | Ignore the item. |

## Validation

A reader MUST reject a model, physics settings, a motion or an expression that breaks any rule in
this section, because the evaluation of such data is undefined (for example, an index out of range
or a non-finite value). Readers SHOULD report all violations, not
only the first. Unknown values are never violations. The avatar profile is not checked here; its
conformance rules are defined by [avatar-profile.md](avatar-profile.md#conformance-checks).

### Model validation

1. IDs are unique within each kind: parameters, parameter groups, parts, deformers, meshes, glues,
   hit areas and slots.
2. Parameters: `min`, `default` and `max` are finite and `min <= default <= max`. `mirror` and
   `group`, if present, refer to an existing parameter and parameter group.
3. References exist: the `part` and `parent` of every mesh and deformer; the `parent` of every part;
   every mask's `mesh`; `mesh_a` and `mesh_b` of every glue; the `mesh` of every hit area; every
   `part` and `links` entry of the pose; the parameters of the eye blink, breath and lip sync
   effects.
4. The parent chains of parts and of deformers have no cycles (an element that is its own parent is
   a cycle).
5. Mask references have no cycles (a mesh that masks itself is a cycle).
6. Every mesh's `texture` is less than the number of `textures`.
7. Meshes: the length of `indices` is a multiple of 3; every index is less than the vertex count
   (the length of `uvs`); every keyform (including blend-shape grids) has as many `positions` as
   there are vertices; every `uvs` component is finite.
8. Glues: every `a` is less than the vertex count of `mesh_a`, and every `b` less than that of
   `mesh_b`.
9. Warp deformers: `rows >= 1` and `cols >= 1`; every keyform (including blend-shape grids) has
   `(rows + 1) × (cols + 1)` points.
10. Every keyform grid (of parts, deformers of a known kind, meshes and glues, and every blend-shape
    grid):
    - the number of keyforms is the product of the key counts of its bindings (1 with no bindings);
    - every binding has at least one key, and its keys are finite and strictly increasing;
    - every binding refers to an existing parameter, and no parameter is bound twice in the grid;
    - `origins` is empty or as long as `keyforms`;
    - `mirror_source.id`, if present, refers to an existing element of the same kind as the owner of
      the grid; `mirror_source.axis` and `tolerance`, if present, are finite;
    - for `inference` with method `depth_rotation`: `angle_scale`, `pivot` and `pivot_depth` are
      finite; `yaw` and `pitch`, if present, refer to existing parameters that are axes of the grid.
11. Blend shapes ([Blend shapes](#blend-shapes)): grids of parts and glues have no `blend_shapes`;
    a blend-shape grid has at least one binding and no `blend_shapes` of its own; no axis of a
    normal grid is a `blend_shape` parameter; no axis of a blend-shape grid is a `normal` parameter;
    no parameter is an axis of two blend-shape grids of the same element.
12. Mirror pairs: `mirror` of a mesh or deformer refers to an existing element of the same kind, and
    pairs are bidirectional; `mirror_axis`, if present, is finite.
13. `depth`, if present, has one value per vertex (mesh) or control point (warp), and every value is
    finite.
14. Pose: no part appears more than once in the pose; `fade_duration` is finite and at least 0.
15. Effects: eye blink `closing`, `closed` and `opening` are finite and at least 0; eye blink
    `interval` and every breath `cycle` are finite and greater than 0.
16. Numbers: `width`, `height` and `origin` of the canvas are finite; every value of every keyform
    (including blend-shape grids) is finite: `opacity` of parts; `points` and `opacity` of warp
    deformers; `origin`, `angle`, `scale` and `opacity` of rotation deformers; `positions`,
    `opacity`, `draw_order`, `multiply_color`, `screen_color` and `mask_strength` of meshes;
    `intensity` of glues. The `weight` of every glue pair and the `offset`, `peak` and `weight` of
    every breath parameter are finite.
17. Slots: `anchor` refers to an existing deformer; `replaces` and `part` to existing parts;
    `parameters` to existing parameters; `mask_sources` to existing meshes; `pose_parts` to parts
    that are the `part` of a pose entry (not only a link); `draw_order.min` and `max` are finite and
    `min <= max`.

### Physics validation

Physics settings are validated together with the model they belong to (after composition, if
modules are attached).

1. Rig IDs are unique; collider IDs are unique.
2. References exist: every input's parameter; every output target's parameter, deformer or mesh;
   every rig's `colliders` entries and `frame`; every collider's `attach`; every entry of the
   `colliders` of the model's slots.
3. Outputs of type `deformer_angle` and `deformer_offset` refer to rotation deformers; `vertices`
   outputs with a `warp` owner refer to warp deformers. (Deformers of an unknown kind are not
   checked.)
4. `vertices` outputs have one weight per vertex of the mesh or per control point of the warp, and
   every weight is between 0 and 1.
5. Every particle's `collision_radius` and every collider's `radius` is finite and at least 0; the
   points of collider shapes are finite.
6. The components of `gravity` and `wind`, and `fps`, are finite.
7. Every input's `weight` and every output's `scale` and `weight` are finite.
8. Particles: `position` is finite; `mobility` is between 0 and 1; `delay`, `acceleration` and
   `radius` are finite and at least 0.
9. Both normalization ranges (`position` and `angle`): `min`, `default` and `max` are finite and
   `min <= default <= max`.

*Informative:* the following are not invalid, because the simulation defines a fallback for them
([physics.md](physics.md)): an `fps` of 0 or less (60 is used); an output whose `particle` is 0 or
not less than the number of particles (it has no effect, [physics.md](physics.md#output-values));
a rig without particles (none of its outputs has an effect); an output `weight` outside 0 to 100
(it is clamped).

### Motion and expression validation

Motions and expressions are validated on their own, not against a model.

1. Motions: `duration` is finite and at least 0; `fade_in`, `fade_out` and the `fade_in` and
   `fade_out` of every curve (if present) are finite; both components (`t` and `v`) of every point
   of every segment (`p`, `to`, and the `c1` and `c2` of `bezier`) are finite; the `t` of every
   event is finite. "Every curve" includes the curves that playback ignores (an unknown target
   `kind`, an unknown effect group name, or a segment of an unknown `type`): their fades and the
   points of their known segments are checked like any other. The fields of a segment of an
   unknown `type`, and the `id` of an unknown target `kind`, are not checked.
2. Expressions: `fade_in`, `fade_out` and the `value` of every adjustment (including adjustments
   with an unknown `op`) are finite.

*Informative:* the following are not invalid, because playback defines their result
([motion.md](motion.md)): a negative fade time (no fade); a control point outside its segment
(clamped when evaluated); events and points outside `0..duration`; points whose times decrease
(a segment that does not move forward in time has its end value). Readers SHOULD warn about
decreasing times, since writers SHOULD NOT produce them ([Motions](#motions)).

### Module validation

A module is validated on its own by treating its elements as a model in which every parameter the
module uses exists (with a kind that is allowed on any grid), and applying the model and physics
rules above, except that references to the model's meshes (masks), colliders and pose entries are
not checked. In addition, the module `name` MUST be valid (non-empty, no `/`, not starting with
`#`). The references to the model are checked when the module is composed ([Composition](#composition)).

Motions and expressions are not validated against the model: curves and adjustments whose target
does not exist in the model have no effect.
