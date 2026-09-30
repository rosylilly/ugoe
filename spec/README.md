# ugoe Specification

**Version 0.1 — Draft.** This specification is not stable. Until version 1.0, any part of it may
change in incompatible ways, and files written according to one draft are not guaranteed to be
readable by implementations of a later draft.

ugoe is a 2D rigging and animation system: a character is drawn as textured triangle meshes that
are deformed, recolored and faded by named scalar *parameters*, and animated by motions, physics
and live tracking. This specification defines the data and file formats of ugoe models so that
programs other than the ugoe reference implementation can read and write them.

## Audience

- Authors of **runtimes** that load ugoe models into games, applications or other engines.
- Authors of **tools** that create, convert or inspect ugoe models (exporters, importers,
  validators, asset pipelines).
- Authors of **applications** that drive avatars (tracking software, streaming tools), who mainly
  need the avatar profile.

The specification is written to be read on its own. You do not need the source code of the
reference implementation to understand it.

## Documents

| Document | Contents | Status |
| --- | --- | --- |
| [model.md](model.md) | The model data: every type and field, default values, encodings (JSON and MessagePack), identifiers, extensible enumerations, validation rules. Also the data of physics settings, motions, expressions, avatar profiles and dress-up modules. | Draft |
| [bundle.md](bundle.md) | The binary container: `.ugoe` bundles and `.ugoemod` module files (chunks, MessagePack, zstd, textures, normal maps), versioning, feature declarations and forward compatibility. | Draft |
| [evaluation.md](evaluation.md) | How parameter values produce deformed meshes: keyform interpolation (linear and smooth), blend shapes, deformers, glues, evaluation order, dynamic physics offsets, hit testing, determinism and conformance tolerances. | Draft |
| [rendering.md](rendering.md) | How evaluated meshes are drawn: mesh color, blend modes, masks, culling, lighting with normal maps, pixel tolerances. | Draft |
| [physics.md](physics.md) | Pendulum physics: time steps, inputs, outputs, rig order and relays, dynamic physics outputs, gravity frames, colliders. | Draft |
| [motion.md](motion.md) | Motions, cross-fades, expressions, eye blink, breathing, lip sync, pose, part opacity, the order of operations within a frame. | Draft |
| [avatar-profile.md](avatar-profile.md) | The avatar profile: standard and extended parameters and their meanings, conformance levels, the face tracking mapping (ARKit blend shapes) and its overrides, gaze, hit areas, license metadata, and the rules for applications that drive avatars. | Draft |
| [schema/](schema/README.md), [examples/](examples/README.md) | JSON Schema for the JSON encodings of the data, and example models with the expected results of evaluating them. | Draft (informative) |

Where this draft needs something that belongs to a planned document, it states only what is needed
to read and write the data correctly and names the planned document.

## Versions

| Specification | `schema_version` (model data) | Container `version` (bundle header) | `profile_version` (avatar profile) |
| --- | --- | --- | --- |
| 0.1 (Draft) | 0 | 0 | 0 |

- `schema_version` is written in every model and module ([model.md](model.md#model)).
- The container version is written in the header of every `.ugoe` and `.ugoemod` file
  ([bundle.md](bundle.md#header)).
- `profile_version` is written in the avatar profile of a model
  ([model.md](model.md#avatar-profile)).

While the specification is a draft, `schema_version` stays 0 and incompatible changes are made
without raising it. From version 1.0 on, new capabilities are added as *features* with defined
fallbacks, and `schema_version` is raised only for changes that cannot be expressed that way
([bundle.md](bundle.md#forward-compatibility)).

## Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "MAY", and "OPTIONAL" in this specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and
[RFC 8174](https://www.rfc-editor.org/rfc/rfc8174) when, and only when, they appear in all
capitals.

- A **reader** is a program that loads a file or payload. A **writer** is a program that produces
  one. A **runtime** is a reader that evaluates and displays a model.
- Sections marked *informative* explain or give examples; everything else is normative.
- Field names, enumeration values and chunk tags are written in `code` exactly as they appear in
  files.

## Terms

| Term | Meaning |
| --- | --- |
| Model | One rigged character: parameters, parts, deformers, meshes and related data. |
| Parameter | A named scalar input with a range and a default value (for example, the head angle). |
| Part | A node of a tree that groups meshes and deformers; its opacity multiplies into its descendants. |
| Deformer | A node of a tree that transforms the coordinates of its children: a *warp* (a lattice of control points) or a *rotation* (origin, angle, scale). |
| Mesh | A textured triangle mesh; the unit of drawing. Also called a drawable. |
| Keyform | The values of an element (vertex positions, opacity, …) at one combination of parameter values. |
| Keyform grid | The keyforms of an element placed on the grid formed by one or more parameter axes; values between keys are interpolated. |
| Blend shape | A keyform grid whose difference from its default is added on top of the normal interpolation result. |
| Glue | A constraint that pulls vertex pairs of two meshes together to hide seams. |
| Mask | A mesh whose coverage limits where another mesh is drawn. |
| Pose | Groups of mutually exclusive parts (for example, alternative arm drawings), of which one is shown at a time. |
| Slot | A named attachment point that a model exposes for dress-up modules. |
| Module | A partial model (for example, a hair style or an outfit) that is attached to a slot at run time. |
| Bundle | A `.ugoe` file: one model with its textures, physics, motions and expressions. |
| Feature | A capability of the format with a stable ID, used for forward compatibility. |
| Fallback | The behavior a reader applies to data it does not understand. |
| Canvas | The model's coordinate space: pixels, origin at the top-left, x to the right, y down. |

## License

This specification, including its examples and any JSON Schema published with it, is licensed
under the [Creative Commons Attribution 4.0 International License](LICENSE) (CC BY 4.0).
Copyright (c) 2026 Sho KUSANO and the ugoe contributors.

- You may implement the formats described here in software under any license, including
  proprietary software. The ugoe contributors do not ask for attribution for implementing them.
- If you copy or adapt the text of this specification, attribute it as "ugoe Specification,
  © Sho KUSANO and the ugoe contributors, CC BY 4.0", link to the license, and indicate changes.
- If you embed examples or JSON Schema files from this specification in your program, keep a
  notice of their origin and license with them (for example, in a comment or in your third-party
  notices). Short fragments needed only to interoperate (field names, chunk tags, magic numbers,
  enumeration values) are facts about the format and need no notice.

The source code of the ugoe reference implementation is licensed separately (MIT for the runtime,
libraries and tools; GPL-3.0-only for the editor and viewer applications).
