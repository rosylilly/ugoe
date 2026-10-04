# ugoe Rendering

**ugoe Specification 0.1 — Draft.** See [README.md](README.md) for the status, conventions and
license of this specification.

This document defines how the evaluated meshes of a model are drawn: the color of each mesh, blend
modes, clipping masks, back-face culling, and lighting with normal maps. The inputs are the outputs
of [evaluation](evaluation.md). The rules are stated as per-pixel equations and do not depend on any
graphics API; a renderer may implement them with any technique (GPU or CPU) that gives the same
results within the tolerances of [Conformance](#conformance).

## Contents

- [Inputs](#inputs)
- [Color values](#color-values)
- [Textures](#textures)
- [Drawing a frame](#drawing-a-frame)
- [Mesh color](#mesh-color)
- [Blend modes](#blend-modes)
- [Masks](#masks)
- [Back-face culling](#back-face-culling)
- [Lighting](#lighting)
- [Unknown values](#unknown-values)
- [Conformance](#conformance)
- [Implementation notes](#implementation-notes)

## Inputs

| Input | Source |
| --- | --- |
| Per mesh: `positions`, `opacity`, `multiply_color`, `screen_color`, `mask_strength`, `visible` | [Evaluation](evaluation.md#overview) |
| Render order | [Evaluation](evaluation.md#render-order) |
| Per mesh: `uvs`, `indices`, `texture`, `blend`, `culling`, `masks`, `mask_mode`, `lit` | The model ([model.md](model.md#meshes)) |
| Texture images and normal maps | The container ([bundle.md](bundle.md#textures-and-normal-maps)) |
| The model's `lighting` flag | The model ([model.md](model.md#model)) |
| View transform | The host |
| Lights | The host ([Lighting](#lighting)) |
| Render target | The host |

- The **view transform** maps canvas coordinates to the render target (for example, fitting,
  zooming, panning or flipping the canvas). It is chosen by the host and is not part of the model.
  It MUST be an affine transform. Everything in this document is defined in canvas coordinates or
  per target pixel, so the result does not depend on the view transform except through where and
  how large the model appears.
- A view transform that has no inverse (its linear part has determinant 0) maps every triangle onto
  a line or a point. Such a triangle covers no pixel center ([Drawing a frame](#drawing-a-frame)),
  so nothing is drawn, and the canvas position of a pixel (used by point lights) is never needed.
- The renderer draws on top of the render target's existing contents. Clearing the target or
  drawing a background is the host's job.

## Color values

- Colors are **non-linear (sRGB-encoded) values used as they are**: texture colors, multiply and
  screen colors, light colors and the render target are all combined without converting to linear
  light. *Informative:* this matches the look that 2D illustration tools and other 2D rigging
  systems produce.
- Colors are **premultiplied by alpha** during rendering. A color `(r, g, b, a)` in this document is
  premultiplied unless stated otherwise.
- The render target holds premultiplied RGBA. Values written to a target of limited range (for
  example 8 bits per channel) are clamped to `[0, 1]` by the target's format.

## Textures

- Texture images are stored non-premultiplied ([model.md](model.md#textures)). A renderer converts
  them to premultiplied alpha before sampling. *Informative:* the reference implementation converts
  8-bit images as `c' = (c × a + 127) div 255` (integer arithmetic, `div` rounding down) per color
  channel.
- KTX2 texture images ([bundle.md](bundle.md#textures-and-normal-maps)) are non-premultiplied as
  well. A renderer:
  - ignores `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` in the KTX2 data format descriptor (writers do not
    set it);
  - uses the stored texel values as they are, whatever the format's `_SRGB` / `_UNORM` suffix or
    transfer function says: they are not converted from sRGB to linear (see
    [Color values](#color-values)). A GPU renderer does not sample them through an sRGB format;
  - decodes block-compressed and supercompressed data (BCn, ETC2, ASTC, Basis Universal and so on)
    and premultiplies the decoded texels. Sampling the compressed data and premultiplying
    afterwards does not meet the bilinear filtering of premultiplied values below;
  - uses only the base level (level 0) of a file with mipmaps.
- Textures are sampled at the interpolated texture coordinates of the pixel with **bilinear
  filtering** of the premultiplied values and **clamp-to-edge** addressing, without mipmaps.
  Renderers SHOULD sample this way; other filtering changes edge pixels and may exceed the
  tolerances of [Conformance](#conformance).
- Texture coordinates are interpolated linearly across each triangle in target space (there is no
  perspective).

## Drawing a frame

1. Compute the mask factor of every mesh that has masks ([Masks](#masks)).
2. For each mesh in the render order (back to front), if its `visible` is `true`, draw its triangles
   onto the render target with its [color](#mesh-color), its [blend mode](#blend-modes) and, if
   `culling` is `true`, [back-face culling](#back-face-culling).

- Meshes whose `visible` is `false` (opacity 0, or `mask_only`) are not drawn. They still act as
  masks when referenced.
- Within one mesh, triangles are drawn in the order of `indices`. Where triangles of the same mesh
  overlap, later triangles are blended over earlier ones.
- A pixel is **covered** by a triangle when the pixel's **center** lies inside the triangle (in
  target pixels, the center of pixel `(x, y)` is `(x + 0.5, y + 0.5)`). Values that vary across a
  triangle (texture coordinates, and the pixel's canvas position used by point lights) are taken at
  the pixel's center. A pixel center that lies exactly on an edge shared by two triangles SHOULD be
  covered by exactly one of them, so that a mesh does not draw it twice. *Informative:* the top-left
  rule of common GPU APIs does this. Renderers that differ at triangle edges (for example, with
  antialiasing) stay within the tolerances of [Conformance](#conformance) only as long as the pixels
  that differ are few.
- The render order MUST NOT be changed. A renderer MAY group consecutive draws for efficiency only
  when the result is unchanged ([Implementation notes](#implementation-notes)).

## Mesh color

For each pixel covered by a triangle of the mesh, with `c` the sampled premultiplied texture color,
`M` the `multiply_color`, `S` the `screen_color` and `α` the mesh's `opacity`:

```
rgb = c.rgb × M
rgb = rgb + S × c.a − rgb × S            // screen blend, in premultiplied form
rgb = rgb × light                        // only if the mesh is lit (Lighting)
out = (rgb, c.a) × α × k                 // k: mask factor (1 if the mesh has no masks)
```

`out` is the premultiplied source color `src` of the blend equations below. The multiplications are
component-wise.

`α` is used as evaluation produced it (not clamped). *Informative:* when models keep opacities within
`[0, 1]` (as they normally do), `α` stays within that range.

*Informative:* premultiplied colors whose `rgb` exceeds their alpha are valid results: `additive`
drawing over transparent pixels leaves color with alpha 0, and light above 1 makes `rgb` exceed `a`
([Shading](#shading)). The render target keeps them as they are. Converting the target to
non-premultiplied colors (for example, to save a PNG image) loses that color; how a host does it is
outside this specification.

## Blend modes

With `src` the mesh color and `dst` the render target's current color (both premultiplied):

| `blend` | `rgb` | `a` |
| --- | --- | --- |
| `normal` | `src.rgb + dst.rgb × (1 − src.a)` | `src.a + dst.a × (1 − src.a)` |
| `additive` | `src.rgb + dst.rgb` | `dst.a` |
| `multiply` | `src.rgb × dst.rgb + dst.rgb × (1 − src.a)` | `dst.a` |

*Informative:* in common GPU APIs these are the blend factors (`One`, `OneMinusSrcAlpha`) for both
color and alpha (normal); color (`One`, `One`) and alpha (`Zero`, `One`) (additive); color (`Dst`,
`OneMinusSrcAlpha`) and alpha (`Zero`, `One`) (multiply).

## Masks

A mesh with `masks` is visible only where its masks allow. The mask rules are defined per target
pixel.

### Coverage

The **coverage** `c_i` of a mask mesh `i` at a pixel is

```
c_i = a_i × k_i
```

- `a_i` is the alpha of mesh `i`'s texture sampled at that pixel, if the pixel is covered by a
  triangle of mesh `i` (its deformed shape); 0 if it is not. Where triangles of mesh `i` overlap,
  their coverages combine as `1 − Π (1 − a)` over the overlapping triangles.
- `k_i` is mesh `i`'s own mask factor (below), or 1 if mesh `i` has no masks. Masks therefore
  **nest**: a mesh clipped by a mask is also clipped by that mask's own masks.

The coverage of a mask mesh does **not** depend on its opacity, `visible`, multiply and screen colors,
blend mode, lighting or `culling`. A mask mesh with opacity 0, a mask mesh in a hidden part and a
`mask_only` mesh mask according to their shape. Mask meshes are never culled.

### Mask factor

For a mesh with masks, let `P` be the set of meshes referenced with `invert = false` (**normal
masks**) and `N` the set of meshes referenced with `invert = true` (**inverted masks**). `P` and `N`
are sets: a mesh referenced twice with the same `invert` counts once. At each pixel:

```
pos = 1                               if P is empty
    = 1 − Π_{i∈P} (1 − c_i)           if mask_mode = union
    = Π_{i∈P} c_i                     if mask_mode = intersection
neg = 1 − Π_{j∈N} (1 − c_j)           (0 if N is empty)
m   = pos × (1 − neg)
k   = 1 − s × (1 − m)                 s = mask_strength of the mesh (already clamped to [0, 1])
```

`k` is the mask factor used in [Mesh color](#mesh-color) and, for mask meshes, in the coverage.

- A mesh is visible where its normal masks cover it (combined by `mask_mode`) and where none of its
  inverted masks cover it.
- With `mask_strength` below 1, the parts outside the masks (inside, for inverted masks) remain
  visible with the factor `1 − s`. With `mask_strength` 0 the masks have no effect.
- There is no special case for masks that are all invisible or empty: a mesh whose normal masks
  cover nothing is not visible (with `s = 1`).
- There is no limit on the number of masks or on the nesting depth. Mask references never form a
  cycle ([model.md](model.md#masks)).

### Resolution

Renderers SHOULD evaluate masks at the resolution of the render target, so that mask edges are as
sharp as the meshes themselves. A renderer MAY offer its users an option to evaluate masks at a lower
resolution for performance; such an option is not part of the model, and results produced with it
are not covered by [Conformance](#conformance).

## Back-face culling

A mesh with `culling = true` does not draw the triangles that face away from the viewer. A triangle
`(p0, p1, p2)` (vertex positions in canvas coordinates, in `indices` order) **faces the front** when

```
(p1.x − p0.x) × (p2.y − p0.y) − (p2.x − p0.x) × (p1.y − p0.y) < 0
```

that is, when its vertices run counter-clockwise as seen on screen with canvas `y` pointing down.
Triangles whose value is 0 or greater are culled.

Front and back are determined in canvas coordinates. They MUST NOT depend on the view transform:
a renderer whose view transform mirrors the canvas (for example, one that flips `y` for an API whose
`y` points up) compensates for it. *Informative:* the reference renderer checks the sign of the
determinant of the view transform's linear part to choose the winding it passes to the GPU.

Culling applies only to drawing the mesh in color, not to its use as a mask.

## Lighting

A mesh is **lit** when both the model's `lighting` and the mesh's `lit` are `true`. Unlit meshes are
drawn with the equations above and no light term. Lighting is feature `lighting.normal_map`
([bundle.md](bundle.md#baseline-features)); a renderer that does not support it draws every mesh
unlit.

### Lights

Lights are supplied by the host each frame. They are not part of the model.

- An **ambient** color `ambient` (default `(1, 1, 1)`).
- Any number of **directional** lights `(direction, color)` and **point** lights
  `(position, color, range)`.

Positions and directions use canvas coordinates (pixels, `y` down) extended with a depth `z` in
pixels, positive **towards the viewer**. Light colors (including `ambient`) may exceed 1; the color
also carries the intensity. The view transform does not affect lighting.

| Light | Vector towards the light `l` | Attenuation |
| --- | --- | --- |
| Directional | `−normalize(direction)`. `direction` is the direction in which the light travels; its length does not matter. Example: light from the top left, in front of the canvas: `(1, 1, −1)`. | 1 |
| Point | `normalize(position − p)`, where `p = (x, y, 0)` and `(x, y)` is the canvas position of the pixel's center (the center mapped back through the view transform). If `position = p`, `l = (0, 0, 1)`. | `clamp(1 − |position − p| / range, 0, 1)²`; 1 if `range` is infinite |

- A directional light whose `direction` has length 0 or is not finite, and a point light whose
  `range` is not positive (or is NaN) or whose `position` is not finite, are ignored.
- When the host supplies no lights, `ambient` is `(1, 1, 1)` and there are no lights, so lit meshes
  look exactly like unlit ones.
- A renderer MAY use only 8 of the valid lights when more are supplied; which ones it uses is not
  specified. [Conformance](#conformance) covers frames with at most 8 valid lights. *Informative:*
  the reference renderer uses at most the first 8 valid lights.

### Normals

The normal of a pixel of a lit mesh comes from the normal map of the mesh's texture
([model.md](model.md#normal-maps-and-lighting)), sampled with the same coordinates and filtering as
the texture:

- A normal map stores a tangent-space normal in its RGB channels as `n = 2 × rgb − 1`. The values are
  not sRGB-decoded; the alpha channel is not used. The normal map image is **not** premultiplied.
- `n.x` points to the right of the image (`+u`), `n.y` to the **top** of the image (`−v`), and `n.z`
  towards the viewer. (This is the "OpenGL" convention; images in the "DirectX" convention, whose
  green channel points down, have to be converted by inverting green before they are stored.)
- A texture without a normal map has the normal `(0, 0, 1)` everywhere.

The normal is then turned with the deformation of the triangle, so that shading follows the mesh:

1. Convert the normal to UV orientation: `n = (n.x, −n.y, n.z)`.
2. Let `J = [t b]` be the Jacobian of the triangle's mapping from texture coordinates to canvas
   coordinates (`t = ∂P/∂u`, `b = ∂P/∂v`; constant within a triangle, since both are interpolated
   linearly).
3. If `det J = t.x × b.y − b.x × t.y < 0` (the triangle is mirrored), replace `t` by `−t` and `n.x`
   by `−n.x`.
4. Let `r = (t.x + b.y, t.y − b.x)`. If `r` is not zero, let `(cos θ, sin θ) = r / |r|` (the rotation
   of the polar decomposition of `J`); otherwise let `θ = 0` (the mapping is degenerate).
5. Rotate the `xy` part of the normal: `n' = (cos θ × n.x − sin θ × n.y, sin θ × n.x + cos θ × n.y, n.z)`.
6. Normalize: `n = n' / |n'|`, or `(0, 0, 1)` if `|n'| = 0`.

Scaling of the mesh along the texture axes (including non-uniform scaling) therefore does not change
the normals; rotation and mirroring do. For example, a mesh mirrored left–right (`J = diag(−1, 1)`)
negates only `n.x`, and a mesh mirrored top–bottom (`J = diag(1, −1)`) negates only `n.y`. A shear
turns the normals by the rotation of its polar decomposition: for `t = (12, 0)` and `b = (4, 16)`,
`r = (28, −4)` and the normals turn by `atan2(−4, 28) ≈ −8.13°`.

### Shading

```
light = ambient + Σ_lights  color × max(0, dot(n, l)) × attenuation
```

`light` multiplies the mesh's `rgb` after the multiply and screen colors and before opacity and
masks ([Mesh color](#mesh-color)). The premultiplied `rgb` may exceed the alpha when `light` exceeds
1; the render target's format clamps the result.

## Unknown values

| Where | Behavior |
| --- | --- |
| Unknown `blend` | Draw with `normal`. |
| Unknown `mask_mode` | Combine normal masks as `union`. |

## Conformance

A renderer conforms when, for the same evaluated meshes, textures, view transform, lights and render
target size, its output differs from the output defined here by at most **3/255 in every channel**
(premultiplied, 8 bits per channel) for **at least 99% of the pixels** of the target. The remaining
pixels account for differences in rasterization at triangle edges and in texture filtering.

- The comparison is made on a render target cleared to transparent black `(0, 0, 0, 0)` before
  drawing, unless an example says otherwise.
- The evaluated meshes are the outputs of [evaluation](evaluation.md); differences allowed by the
  evaluation tolerances are included in the rendering tolerance.
- The output defined here is computed with real numbers; only writing to the render target clamps
  values to `[0, 1]` (after each draw). The tolerance includes the rounding of 8-bit textures,
  intermediate results and the target.
- The frame has at most 8 valid lights ([Lights](#lights)).
- The [rendering examples](examples/README.md#rendering-examples) check chosen pixels away from
  triangle edges, where the result does not depend on rasterization details; every checked pixel
  has to be within the tolerance.

## Implementation notes

*This section is informative.* It describes techniques of the reference renderer that do not change
the result.

- **Batching.** Consecutive meshes in the render order may be drawn with one draw call when they
  share the texture (and normal map), blend mode and culling, because GPUs blend the primitives of
  one draw call in order. Per-mesh values (opacity, colors, mask factor inputs, lighting mode) are
  passed per vertex or per instance. The render order is never changed.
- **Texture atlases.** Tools may pack textures into atlases at build time so that more consecutive
  meshes share a texture ([bundle.md](bundle.md#textures-and-normal-maps) stores whatever textures the
  model references).
- **Mask atlas.** The reference renderer draws each distinct mask *content* (a product
  `Π (1 − c_i)` over a set of masks, or a complete factor `m` when normal and inverted masks are
  combined or normal masks intersect) into one channel of an RGBA8 atlas, using multiplicative
  blending on a channel initialized to 1. Each content gets a cell at least as large, in pixels, as
  the screen bounding box of the meshes that read it; nested masks are drawn into separate atlases
  by depth. The main pass reads the cell with the mesh's screen position. Only RGBA8 targets,
  multiplicative blending, write masks and scissor rectangles are needed, so the technique works on
  WebGL 2.
- **Skipping unchanged work.** The reference renderer compares the data of each frame with what it
  last uploaded and uploads only what changed, and reuses the mask atlas when nothing that affects
  it changed.
