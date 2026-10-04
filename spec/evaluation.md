# ugoe Evaluation

**ugoe Specification 0.1 — Draft.** See [README.md](README.md) for the status, conventions and
license of this specification.

This document defines how a runtime turns a [model](model.md) and a set of parameter values into
the state of every mesh: deformed vertex positions in canvas coordinates, opacity, drawing order,
colors and mask strength. It covers keyform interpolation (linear and smooth), blend shapes, the
deformer tree (warp and rotation deformers), glues, the order of evaluation, the additions made by
dynamic physics, and hit testing. How the resulting meshes are drawn is defined in
[rendering.md](rendering.md); how parameter values change over time (motions, effects, physics) is
defined in [motion.md](motion.md) and [physics.md](physics.md).

This document also defines what it means for an implementation to produce the *same* result as
another ([Conformance](#conformance)); [rendering.md](rendering.md), [physics.md](physics.md) and
[motion.md](motion.md) refer to it.

## Contents

- [Overview](#overview)
- [Numbers and determinism](#numbers-and-determinism)
- [Conformance](#conformance)
- [Parameter values](#parameter-values)
- [Keyform interpolation](#keyform-interpolation)
- [Blend shapes](#blend-shapes)
- [Order of evaluation](#order-of-evaluation)
- [Parts](#parts)
- [Deformers](#deformers)
- [Meshes](#meshes)
- [Glues](#glues)
- [Render order](#render-order)
- [Dynamic physics offsets](#dynamic-physics-offsets)
- [Hit testing](#hit-testing)

## Overview

Evaluation is a pure function of the model and the current parameter values (and, when dynamic
physics is used, of the offsets that physics supplies). It has no memory of earlier frames and does
not depend on time.

For every mesh `m` (in the order of the model's `meshes`), evaluation produces:

| Output | Type | Meaning |
| --- | --- | --- |
| `positions` | `vec2[]` | Deformed vertex positions in canvas coordinates, one per vertex. |
| `opacity` | `f32` | Final opacity: the mesh's own opacity times its parts' and deformers' opacities. |
| `draw_order` | `f32` | Interpolated drawing order. |
| `multiply_color`, `screen_color` | `rgb` | Interpolated colors. |
| `mask_strength` | `f32` | Interpolated mask strength, clamped to `[0, 1]`. |
| `visible` | `bool` | Whether the mesh is drawn in color. |

It also produces the **render order**: the indices of all meshes sorted from back to front
([Render order](#render-order)).

The texture coordinates, triangles, blend mode, culling and mask references of a mesh are static
data of the model and are not changed by evaluation.

Notation used in this document:

- `lerp(a, b, t) = a + (b − a) × t`, applied component-wise to vectors.
- `clamp(x, lo, hi) = min(max(x, lo), hi)`.
- `R(θ)` is the rotation matrix `[[cos θ, −sin θ], [sin θ, cos θ]]` (columns `(cos θ, sin θ)` and
  `(−sin θ, cos θ)`), with `θ` converted from degrees to radians. Because canvas `y` points down,
  a positive `θ` rotates **clockwise** on screen.
- Points are column vectors; `M · p` applies matrix `M` to point `p`.

## Numbers and determinism

- All values are IEEE 754 single-precision numbers (`f32`) in the data. Implementations MUST
  compute with at least single precision. They MAY compute with higher precision, fuse
  multiplications and additions, or reorder operations, as long as the results stay within the
  tolerances of [Conformance](#conformance).
- Where this document gives an order of operations (for example the order in which weighted terms
  are summed), that order is the order of the reference implementation. Following it lets an
  implementation reproduce the reference results closely; it is RECOMMENDED but not required.
- Evaluation MUST be deterministic: the same model and the same parameter values (and offsets)
  MUST always produce the same outputs within one implementation on one platform. Evaluation MUST
  NOT use random numbers, the clock, or state carried over from earlier evaluations.
- *Informative:* the reference implementation computes in `f32` exactly in the orders given here.
  On one platform it reproduces its own results bit for bit; across platforms, results may differ
  in the last bits because of different implementations of `sin`, `cos` and `atan2`.

## Conformance

This specification does not require bit-identical results. An implementation *conforms* to the
evaluation rules when, for every model and parameter values, its outputs differ from the outputs
defined here by no more than the following tolerances. The conformance examples published with this
specification ([examples/README.md](examples/README.md)) give expected outputs to be checked with
these tolerances.

| Output | Tolerance |
| --- | --- |
| Vertex positions (each coordinate, canvas pixels) | `|Δ| ≤ 0.01` |
| `opacity`, `multiply_color`, `screen_color`, `mask_strength` (each component) | `|Δ| ≤ 1e-4` |
| `draw_order` | `|Δ| ≤ 1e-3` |
| `visible` | Equal, except for meshes whose expected `opacity` is at most `1e-4` (then either value is accepted). |
| Render order | Equal, except that two meshes whose expected `draw_order` values differ by at most `1e-3` may appear in either order. |

- The tolerances apply to models whose values are of ordinary magnitude (canvas coordinates within
  ±100,000 pixels, parameter keys and values within ±100,000, no values that make intermediate
  results overflow). For other models, an implementation SHOULD stay as close to the defined
  results as single precision allows.
- Results that are not finite (NaN or infinity) in the defined computation are not covered by the
  tolerances. Implementations MAY produce any value for them.
- The tolerances for rendering, physics and motion are defined in their documents
  ([rendering.md](rendering.md#conformance), [physics.md](physics.md#conformance),
  [motion.md](motion.md#conformance)).

## Parameter values

- A runtime holds one current value for every parameter of the model. The initial value is the
  parameter's `default`.
- Every value written to a parameter is clamped to `[min, max]` of that parameter, by whoever
  writes it (the host, motions, effects, tracking, physics). Evaluation therefore always sees values
  within the parameter's range.
- A runtime MAY ignore (leave unchanged) a write of a value that is not finite.

## Keyform interpolation

Every [keyform grid](model.md#keyform-grids) is interpolated the same way: evaluation computes a list
of **weights** — pairs `(index, weight)` where `index` indexes the grid's `keyforms` — and every
number `x` of the interpolated keyform is the weighted sum

```
x = Σ_k  weight_k × keyforms[index_k].x          (summed in the order of the list)
```

The weights depend only on the grid's bindings and the current parameter values. They always sum to
1 (up to rounding). Linear weights are never negative; smooth weights can be negative.

### Locating a value on an axis

For an axis (binding) with keys `K = [k_0, …, k_(n−1)]` (strictly increasing) and the current value
`v` of its parameter, the **position** `(seg, t)` is:

1. If `n = 1` or `v ≤ k_0`: `(0, 0)`.
2. Otherwise, if `v ≥ k_(n−1)`: `(n − 1, 0)`.
3. Otherwise: `seg` is the index with `k_seg ≤ v < k_(seg+1)`, and
   `t = (v − k_seg) / (k_(seg+1) − k_seg)`.

Interpolation never extrapolates beyond the first and last keys: a value outside the keys behaves as
the nearest end key.

### Linear weights

With `N` bindings, where binding `a` has `n_a` keys, the weights are built axis by axis, starting
from the single weight `(0, 1)`:

```
W = [(0, 1)]
stride = 1
for a in 0 .. N−1:
    (seg, t) = position of binding a
    let m = length of W
    for i in 0 .. m−1:                     // existing entries, in order
        (index, w) = W[i]
        W[i] = (index + seg × stride, w × (1 − t))
        if t > 0:
            append (index + (seg + 1) × stride, w × t) to W
    stride = stride × n_a
remove every entry whose weight is 0 (or NaN)
```

The indices follow the keyform order of [model.md](model.md#keyform-grids) (axis 0 varies fastest).
A grid without bindings has the single weight `(0, 1)`. With `N` axes there are at most `2^N`
weights.

### Smooth weights

When a grid's `interpolation` is `smooth`, every axis with **three or more keys** is interpolated
with a cubic Hermite curve through the keys instead of linearly. Axes with one or two keys stay
linear. Grids of parts and glues, and grids whose `interpolation` is `linear` or an unknown value,
use only linear weights.

The result is still a weighted sum of keyforms, so every rule of this document that uses weights
(blend shapes, dynamic physics) works unchanged.

**Tangent coefficients.** For a smooth axis with keys `k_0 … k_(n−1)` (`n ≥ 3`), the tangent at key
`i` is the derivative, at `k_i`, of the parabola through three consecutive keys `j, j+1, j+2`, where
`j = clamp(i − 1, 0, n − 3)`. With `h0 = k_(j+1) − k_j`, `h1 = k_(j+2) − k_(j+1)`, `S = h0 + h1`,
the tangent is `a × d0 + b × d1`, where `d0` and `d1` are the slopes of the two intervals and:

| Key | `a` | `b` |
| --- | --- | --- |
| `i = j` | `(2 h0 + h1) / S` | `−h0 / S` |
| `i = j + 1` | `h1 / S` | `h0 / S` |
| `i = j + 2` | `−h1 / S` | `(2 h1 + h0) / S` |

Expressed in the keyform values `P_j, P_(j+1), P_(j+2)`, the tangent at key `i` is
`c_0 P_j + c_1 P_(j+1) + c_2 P_(j+2)` with

```
c_0 = −a / h0,   c_1 = a / h0 − b / h1,   c_2 = b / h1
```

The coefficients depend only on the keys. *Informative:* the reference implementation computes them
once, in double precision, and rounds them to single precision.

**Axis weights.** At position `(seg, t)` with `0 < t < 1` (at `t = 0` the axis has the single
weight of key `seg`, exactly as in linear interpolation), let `h = k_(seg+1) − k_seg` and

```
h00 = 2t³ − 3t² + 1      h10 = t³ − 2t² + t      h01 = −2t³ + 3t²      h11 = t³ − t²
```

The interpolated value is `h00 P_seg + h01 P_(seg+1) + h × (h10 m_seg + h11 m_(seg+1))`, where `m_i`
is the tangent at key `i`. Substituting the tangents gives one coefficient for each of the keys
`base … base + 3`, where `base = max(seg − 1, 0)`:

```
A = [0, 0, 0, 0]
A[seg − base]     += h00
A[seg + 1 − base] += h01
for (key, basis) in [(seg, h10), (seg + 1, h11)]:
    j = clamp(key − 1, 0, n − 3)
    for c in 0 .. 2:
        A[j + c − base] += h × basis × c_c(key)       // c_c(key): tangent coefficient c of key `key`
```

Only the first `min(4, n − base)` coefficients are used.

**Combining axes.** The weights of a grid with smooth axes are built like linear weights, except
that a smooth axis at a position with `t > 0` contributes its coefficients:

```
    for i in 0 .. m−1:
        (index, w) = W[i]
        W[i] = (index + base × stride, w × A[0])
        for c in 1 .. min(4, n_a − base) − 1:
            append (index + (base + c) × stride, w × A[c]) to W
```

A linear axis, or a smooth axis at `t = 0`, contributes as in [Linear weights](#linear-weights).
Entries whose weight is 0 or NaN are removed at the end.

*Informative:* with three keys the curve is exactly the parabola through them, so it reproduces
quadratic motion exactly even with unevenly spaced keys. The curve can overshoot the key values
(for a step `[0, 0, 1]`, by about 12.5%). At the keys the result equals the linear result.

### Shape fields and other fields

Each grid has two weight lists:

- the **shape weights**, computed with smooth axes as above, and
- the **linear weights**, computed with every axis treated as linear.

When no axis of the grid is smooth, the two lists are identical.

| Keyform | Fields interpolated with the shape weights | Fields interpolated with the linear weights |
| --- | --- | --- |
| `MeshKeyform` | `positions` | `opacity`, `draw_order`, `multiply_color`, `screen_color`, `mask_strength` |
| `WarpKeyform` | `points` | `opacity` |
| `RotationKeyform` | `origin`, `angle`, `scale` | `opacity`, `reflect_x`, `reflect_y` |
| `PartKeyform` | — | `opacity` |
| `GlueKeyform` | — | `intensity` |

*Informative:* only shape fields use the curve, because an overshoot of opacity, drawing order or
colors would be visible as an error (for example, drawing-order keys `[500, 500, 600]` would dip
below 500 between the first two keys and change which mesh is in front).

Vectors and colors are interpolated component by component. Vertex positions and control points
are interpolated point by point with the same weights.

### Boolean fields

A boolean field (`reflect_x`, `reflect_y`) takes the value of the keyform with the **largest** linear
weight. If several keyforms have the same largest weight, the one with the smallest index wins.

## Blend shapes

A [blend-shape grid](model.md#blend-shapes) adds its change from its default state to the result of
the normal grid.

For each blend-shape grid `B` of an element, in the order of the `blend_shapes` array:

1. Compute `B`'s shape weights and linear weights at the current parameter values
   (**current weights**).
2. Compute `B`'s shape weights and linear weights with every parameter at its `default` value
   (**rest weights**). They depend only on the model and MAY be computed once.
3. If the current shape weights equal the rest shape weights and the current linear weights equal
   the rest linear weights (same indices and same values, in the same order), skip `B`.
4. Otherwise, for every numeric field `x` of the keyform, add

   ```
   δx = interp(B, current weights) − interp(B, rest weights)
   ```

   to the element's interpolated value of `x`, where `interp` is the weighted sum of
   [Keyform interpolation](#keyform-interpolation), using the shape weights for shape fields and the
   linear weights for other fields ([table](#shape-fields-and-other-fields)). For vertex positions
   and control points, `δ` is computed and added point by point.

The additions are made one grid after another: `((x_normal + δx_0) + δx_1) + …`. Boolean fields are
not affected; they always come from the normal grid.

The additions happen in the element's local coordinates (the coordinates of its keyforms), before
any deformer transform is applied, and before the clamping of `mask_strength`.

Because the rest weights are subtracted, a model whose blend-shape parameters are all at their
defaults evaluates exactly as if it had no blend shapes.

Fallback (feature `deform.blend_shape`, [bundle.md](bundle.md#baseline-features)): a reader that does
not support blend shapes ignores `blend_shapes`.

## Order of evaluation

A runtime evaluates the model in this order:

1. **Parts:** compute every part's opacity, parents before children ([Parts](#parts)).
2. **Deformers:** compute every deformer's world transform and opacity, parents before children
   ([Deformers](#deformers)).
3. **Meshes:** interpolate every mesh and map its vertices to canvas coordinates; add dynamic
   physics offsets to the vertices ([Meshes](#meshes)).
4. **Glues:** apply glues in array order ([Glues](#glues)).
5. **Render order:** sort the meshes by drawing order ([Render order](#render-order)).

Steps 1 and 2 are independent of each other. Within step 2, deformers MAY be evaluated in any order
in which every deformer comes after its parent; the result does not depend on the order.
*Informative:* the reference implementation walks `deformers` in array order and, for each deformer
not yet evaluated, evaluates its not-yet-evaluated ancestors first, root first.

## Parts

For each part `p`, parents before children:

```
own(p)          = interpolated opacity of p                  (linear weights)
own(p)          = lerp(own(p), override_value, override_weight)   if p has an opacity override
part_opacity(p) = own(p) × part_opacity(parent(p))           (× 1 for a root part)
```

**Part opacity overrides** are supplied by the runtime's motion system, not by the model: part
opacity curves of motions and the pose write them each frame ([motion.md](motion.md#part-opacity)).
An override is a pair `(value, weight)` with `weight` clamped to `[0, 1]`; a part without an override
has weight 0. An override replaces the part's *own* opacity only; it still multiplies with the
parent's opacity. A runtime MAY ignore an override whose value or weight is not finite (the part
keeps its previous override), as for parameter values ([Parameter values](#parameter-values));
otherwise the part's opacity would become NaN. *Informative:* the reference implementation ignores
such overrides.

Part opacities are not clamped.

## Deformers

### World transforms

Every deformer `D` has a **world transform** `F_D`: the mapping from `D`'s local coordinate system
(the coordinates of `D`'s children) to canvas coordinates. An element whose `parent` is none has the
identity as its parent transform: `F(p) = p`.

A world transform is one of:

- the **identity**;
- an **affine** transform `F(p) = T + M · p` (a 2×2 matrix `M` and a translation `T`), produced by
  rotation deformers;
- a **warp**: a lattice of control points in canvas coordinates, produced by warp deformers.

### Warp deformers

For a warp deformer with `cols × rows` cells:

1. Interpolate the control points `Q[x, y]` (`0 ≤ x ≤ cols`, `0 ≤ y ≤ rows`, stored with `x` varying
   fastest) with the shape weights, and add the blend-shape deltas. They are in the parent's local
   coordinates.
2. Map every control point through the parent's world transform: `P[x, y] = F_parent(Q[x, y])`.
3. Add the dynamic physics offsets of the control points, if any
   ([Dynamic physics offsets](#dynamic-physics-offsets)).

The world transform `F_D` maps a local point `(u, v)` through the lattice `P`.

**Inside the lattice** (`0 ≤ u ≤ 1` and `0 ≤ v ≤ 1`), the mapping is bilinear within each cell:

```
gx = u × cols,                        gy = v × rows
i  = clamp(floor(gx), 0, cols − 1),   j  = clamp(floor(gy), 0, rows − 1)
fx = gx − i,                          fy = gy − j
B(u, v) = lerp( lerp(P[i, j],     P[i+1, j],     fx),
                lerp(P[i, j+1],   P[i+1, j+1],   fx),  fy )
```

**Outside the lattice**, the point is clamped to the lattice and the remainder is extrapolated along
the average edge vectors of the lattice:

```
C00 = P[0, 0],   C10 = P[cols, 0],   C01 = P[0, rows],   C11 = P[cols, rows]
A_u = ((C10 − C00) + (C11 − C01)) × 0.5
A_v = ((C01 − C00) + (C11 − C10)) × 0.5
c   = (clamp(u, 0, 1), clamp(v, 0, 1))
d   = (u, v) − c
F_D(u, v) = B(c) + A_u × d.x + A_v × d.y
```

Inside the lattice `d = 0` and `F_D = B`, so the mapping is continuous at the border.

*Informative:* extending the bilinear formula of the edge cells beyond the lattice was rejected
because it distorts quadratically and can turn tapered lattices inside out. The chosen
extrapolation is exact for affine lattices and never mirrors the image, so hair tips and other parts
that stick out of a lattice follow the lattice as a whole.

The warp's own opacity is its interpolated `opacity` (linear weights) plus the blend-shape deltas.

### Rotation deformers

A rotation deformer's interpolated keyform gives `origin`, `angle` (degrees, clockwise positive),
`scale` (shape weights, plus blend-shape deltas), and `reflect_x`, `reflect_y`
([Boolean fields](#boolean-fields)). Dynamic physics may add an angle offset `Δangle`
([Dynamic physics offsets](#dynamic-physics-offsets)); let `α = angle + Δangle` (0 without physics).

The linear part is

```
S = diag( scale × (reflect_x ? −1 : 1),  scale × (reflect_y ? −1 : 1) )
L(p) = origin + R(α) · S · p
```

The world transform depends on the parent:

- **No parent:** `F_D = L`.
- **Parent with an identity or affine world transform** (a rotation deformer, or a deformer of an
  unknown kind whose own parent is such): `F_D = F_parent ∘ L`, the composition of the two affine
  transforms.
- **Parent with a warp world transform:** only the origin and the direction are taken through the
  warp. The children are not stretched by it:

  ```
  ε  = 0.01                                        (in the parent's lattice units)
  o  = F_parent(origin)
  up = F_parent(origin + (0, −ε)) − o
  δ  = atan2(up.x, −up.y), in degrees             (atan2(0, 0) = 0)
  F_D(p) = o + R(α + δ) · S · p
  ```

  `δ` is the clockwise angle by which the lattice turns the "up" direction `(0, −1)` at the origin.
  When `origin + (0, −ε)` lies outside the lattice, the extrapolation above is used.

Finally, if dynamic physics supplies an origin offset `Δo` (in canvas pixels), the translation of
`F_D` is moved by it: `F_D(p) = (previous F_D)(p) + Δo`.

Because every rotation deformer turns its coordinate system into an affine one, all descendants of a
rotation deformer have affine world transforms until the next warp deformer.

### Unknown kinds

A deformer of an unknown kind has the world transform of its parent (identity if it has no parent)
and its own opacity is 1. Its children are therefore mapped as if they were children of the unknown
deformer's parent. Its keyforms are not interpolated.

### Deformer opacity

```
deformer_opacity(D) = own opacity of D × deformer_opacity(parent(D))     (× 1 without a parent)
```

Deformer opacities are not clamped.

## Meshes

For each mesh (any order):

1. Interpolate `positions` with the shape weights and the other fields with the linear weights, and
   add the blend-shape deltas ([Blend shapes](#blend-shapes)).
2. Map every vertex through the world transform of the mesh's parent deformer (identity if none):
   `position[i] = F_parent(local_position[i])`.
3. Add the dynamic physics offsets of the vertices, if any, in canvas pixels.
4. Compute the outputs:

   ```
   opacity       = own opacity × part_opacity(mesh.part) × deformer_opacity(mesh.parent)
                                                      (× 1 when the mesh has no parent deformer)
   draw_order    = interpolated draw_order
   multiply_color, screen_color = interpolated colors
   mask_strength = clamp(interpolated mask_strength, 0, 1)
   visible       = (mask_only is false) and (opacity > 0)
   ```

- Opacity and colors are not clamped by evaluation. Colors are not inherited from parts or
  deformers.
- The vertex positions of step 3 are further changed by glues (below).

## Glues

Glues are applied after all meshes have been computed, in the order of the model's `glues` array.
Within a glue, the pairs are applied in array order. For each glue:

1. Interpolate `intensity` with the linear weights (it is not clamped).
2. For each pair `(a, b, weight)`, with `pa` the current position of vertex `a` of `mesh_a` and `pb`
   the current position of vertex `b` of `mesh_b`:

   ```
   target = lerp(pa, pb, weight)
   pa'    = lerp(pa, target, intensity)
   pb'    = lerp(pb, target, intensity)
   ```

   and store `pa'` and `pb'` as the new positions.

When several glues or pairs move the same vertex, each one starts from the result of the previous
one. Positions include the dynamic physics offsets added before glues.

## Render order

The render order lists the indices of all meshes (visible or not), sorted by `draw_order` in
ascending order (back to front). Meshes with equal `draw_order` keep the order of the model's
`meshes` array (stable sort). Values are compared with the IEEE 754 `totalOrder` predicate (so
`−0` sorts before `+0`).

## Dynamic physics offsets

[Dynamic physics](physics.md#dynamic-physics) moves deformers and vertices directly, without
parameters. It supplies **offsets** that evaluation adds at these points:

| Offset | Added to | Coordinates |
| --- | --- | --- |
| Rotation deformer angle | The interpolated `angle`, before the world transform is built | degrees, in the deformer's own terms |
| Rotation deformer origin | The translation of the built world transform | canvas pixels |
| Warp control points | Each control point after mapping through the parent (`P[x, y]`) | canvas pixels |
| Mesh vertices | Each vertex after mapping through the parent, before glues | canvas pixels |

Offsets that are absent are 0. A runtime without dynamic physics never has offsets. Physics never
produces an offset that is not finite ([physics.md](physics.md#non-finite-state)); a runtime that lets
the host write offsets MAY treat an offset that is not finite (any of its components, for a position)
as 0. *Informative:* the reference implementation does.

Dynamic physics needs the world transforms of deformers *before* it can compute the offsets. A
runtime that supports it therefore evaluates a frame in two stages:

1. **First stage:** evaluate the deformers ([Deformers](#deformers)) with the current parameter
   values and **without** offsets.
2. **Physics:** from the first-stage world transforms, advance the physics and produce the offsets
   and the physics outputs to parameters ([physics.md](physics.md#evaluation-with-dynamic-physics)).
   The host MAY change further parameters at this point (the frame order of
   [motion.md](motion.md#order-of-operations-in-a-frame) does).
3. **Second stage:** evaluate the whole model ([Order of evaluation](#order-of-evaluation)) with the
   final parameter values and the offsets.

The result of the second stage MUST equal a complete evaluation with the final parameter values and
the offsets. *Informative:* the reference implementation re-evaluates in the second stage only the
deformers that receive offsets, whose bound parameters (including those of their blend-shape grids)
changed bit for bit since the first stage, and their descendants; the other deformers keep their
first-stage results, which are identical.

## Hit testing

Hit areas ([model.md](model.md#hit-areas)) are tested against the **deformed** shape of their mesh:
the `positions` of the most recent evaluation, in canvas coordinates.

- A point hits a mesh when it lies inside or on the boundary of any triangle of the mesh, regardless
  of the triangle's winding. Triangles that refer to a vertex index out of range are ignored.
- Hit testing uses the triangles only. It does not look at the texture (transparent pixels hit), at
  the opacity, or at `visible`: meshes with opacity 0, hidden meshes and mask-only meshes are hit
  too, so that invisible meshes can serve as hit areas. A host that wants to exclude hidden meshes
  checks `visible` itself.
- When a runtime lists the hit areas that contain a point, it lists them **front to back**: in the
  reverse of the render order of their meshes; hit areas that use the same mesh are listed in the
  order of the model's `hit_areas`.
- Hit testing does not evaluate the model. Changing parameters without evaluating does not change
  the result.
