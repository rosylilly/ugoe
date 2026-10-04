# ugoe Bundle and Module Files

**ugoe Specification 0.1 — Draft.** See [README.md](README.md) for the status, conventions and
license of this specification.

This document defines the binary container used by ugoe files, the two kinds of file built on it,
versioning, and the rules that let older readers load files written for newer versions (feature
declarations and fallbacks). The data inside the container is defined in [model.md](model.md).

## Contents

- [File kinds](#file-kinds)
- [Container layout](#container-layout)
- [Chunk types](#chunk-types)
- [Payload encoding](#payload-encoding)
- [Textures and normal maps](#textures-and-normal-maps)
- [Metadata](#metadata)
- [Bundle files](#bundle-files)
- [Module files](#module-files)
- [Reading](#reading)
- [Writing](#writing)
- [Versioning](#versioning)
- [Forward compatibility](#forward-compatibility)
- [Standalone JSON files](#standalone-json-files)

## File kinds

| Kind | Extension | Contents |
| --- | --- | --- |
| Bundle | `.ugoe` | One model, with its textures, normal maps, physics settings, motions, expressions and metadata. Self-contained; meant for distribution and for runtimes. |
| Module | `.ugoemod` | One dress-up module with its textures and normal maps ([model.md](model.md#modules)). |
| Physics | `.physics.json` | Physics settings as JSON ([Standalone JSON files](#standalone-json-files)). |
| Motion | `.motion.json` | One motion as JSON. |
| Expression | `.expression.json` | One expression as JSON. |

Bundles and module files use the same container. The suggested media type for both is
`application/octet-stream`.

## Container layout

All integers in the header and the chunk table are **little-endian**. Offsets are counted in bytes
from the start of the file.

```
Header           16 bytes
Chunk table      chunk_count × 24 bytes
Chunk payloads   anywhere after the chunk table, each starting at a multiple of 16
```

### Header

| Offset | Size | Field | Value |
| --- | --- | --- | --- |
| 0 | 4 | `magic` | The bytes `55 47 4F 45` (ASCII `UGOE`). |
| 4 | 4 | `version` | Container version, `u32`. This draft defines version 0. |
| 8 | 4 | `flags` | Reserved, `u32`. MUST be 0. |
| 12 | 4 | `chunk_count` | Number of entries in the chunk table, `u32`. |

### Chunk table

The chunk table immediately follows the header. Each entry is 24 bytes:

| Offset | Size | Field | Value |
| --- | --- | --- | --- |
| 0 | 4 | `tag` | Four bytes identifying the chunk type, normally ASCII ([Chunk types](#chunk-types)). |
| 4 | 4 | `flags` | `u32`. Bit 0: the payload is compressed with zstd. All other bits are reserved and MUST be 0. |
| 8 | 8 | `offset` | `u64`. Position of the payload. MUST be a multiple of 16. |
| 16 | 8 | `length` | `u64`. Length of the payload as stored (compressed length if compressed). |

- The payload of an entry occupies the bytes `[offset, offset + length)`, which MUST lie within the
  file and MUST NOT overlap the header, the chunk table or the payload of another entry. A payload
  of length 0 is allowed; it occupies no bytes and overlaps nothing (its `offset` may point into
  the payload of another entry), but its `offset` is still subject to the alignment rule, MUST NOT
  be less than the end of the chunk table and MUST NOT be greater than the length of the file.
- Bytes that belong to no payload (alignment padding and gaps) have no meaning. Writers SHOULD fill
  padding with zeros; readers MUST NOT interpret it.
- The order of entries in the table is significant for chunks of the same tag (for example, the
  `i`-th `TEXR` entry is the `i`-th texture). The order of payloads in the file need not match the
  order of the table.

*Informative example* — a bundle with a model and one texture, uncompressed:

```
00000000  55 47 4F 45  00 00 00 00  00 00 00 00  02 00 00 00   "UGOE", version 0, flags 0, 2 chunks
00000010  4D 4F 44 4C  00 00 00 00  40 00 00 00 00 00 00 00   "MODL", flags 0, offset 0x40
00000020  xx xx xx xx xx xx xx xx                              length of the model payload
00000028  54 45 58 52  00 00 00 00  …                          "TEXR", flags 0, offset (aligned), length
00000040  …                                                    MessagePack model, then padding, then PNG
```

## Chunk types

| Tag | Payload | In a bundle | In a module file |
| --- | --- | --- | --- |
| `FEAT` | [Feature declarations](#feature-declarations) (MessagePack) | 0 or 1 | 0 or 1 |
| `MODL` | A [model](model.md#model) (MessagePack) | exactly 1 | not allowed |
| `MODU` | A [module](model.md#modules) (MessagePack) | not allowed | exactly 1 |
| `PHYS` | [Physics settings](model.md#physics-settings) (MessagePack) | 0 or 1 | not allowed |
| `MOTN` | One [motion](model.md#motions) (MessagePack) | any number | not allowed |
| `EXPR` | One [expression](model.md#expressions) (MessagePack) | any number | not allowed |
| `META` | [Metadata](#metadata) (JSON) | 0 or 1 | 0 or 1 |
| `TEXR` | One texture image (PNG or KTX2) | one per texture | one per texture |
| `NRML` | One normal map image (PNG or KTX2) | one per normal map | one per normal map |

- A chunk whose tag is not in this table is an **unknown chunk**. Readers MUST skip unknown chunks
  without decompressing or decoding their payload (but their table entry is still subject to the
  layout rules above, including the reserved flag bits).
- A chunk that is "not allowed" in a kind of file makes the file invalid. This detects a bundle
  opened as a module file or vice versa.
- Tags whose first byte is `X` (0x58) are reserved for testing readers and writers (for example, to
  check that unknown chunks are skipped). No chunk type of this specification will use them.

## Payload encoding

| Chunks | Encoding |
| --- | --- |
| `FEAT` `MODL` `MODU` `PHYS` `MOTN` `EXPR` | [MessagePack](https://github.com/msgpack/msgpack/blob/master/spec.md). The payload (decompressed, if compressed) is exactly one MessagePack value. Every object is a map keyed by field name, as defined in [model.md](model.md#data-conventions). |
| `TEXR` `NRML` | The bytes of a PNG file or a KTX2 file, unchanged. |
| `META` | A UTF-8 JSON document ([RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)). |

### Compression

- If bit 0 of a chunk's `flags` is set, the stored payload is a single
  [zstd](https://www.rfc-editor.org/rfc/rfc8878) frame, and the payload is its decompressed content.
- Compression is optional for writers. Writers SHOULD NOT compress `TEXR` and `NRML` chunks (the
  images are already compressed).
- This specification sets no limit on the decompressed size of a chunk or on the total
  decompressed size of a file. Limits are a policy of the reader ([Resource limits](#resource-limits)).
- Writers SHOULD NOT write a compressed chunk that decompresses to more than 256 MiB (268,435,456
  bytes), so that every reader with its default limits can read the file. A file that does is still
  conforming, but a reader with a lower limit rejects it.

### Resource limits

A file can be small and still decompress to a very large size: a zstd frame of a few kilobytes can
describe hundreds of megabytes of repeated bytes, and a file can contain many such chunks.

- A reader MAY limit the resources it uses to read a file, for example the decompressed size of one
  chunk or the total decompressed size of all the chunks of a file. The values of the limits are the
  reader's choice; a reader SHOULD let its user set them.
- When reading a file would exceed a limit, the reader MUST stop reading the file and report an
  error. It MUST NOT crash or exhaust its memory. A reader SHOULD detect the excess while
  decompressing, without first decompressing the whole chunk or trusting a size stated in the zstd
  frame.
- If a reader limits the decompressed size of one chunk, the default value of that limit MUST be at
  least 256 MiB (268,435,456 bytes), the size that writers can rely on ([Compression](#compression)).
  A default limit on the total decompressed size SHOULD be at least as large. A user of the reader
  MAY set lower values.

*Informative:* by default, the reference reader limits one chunk to 256 MiB and all the decompressed
chunks of a file to 1 GiB (1,073,741,824 bytes) in total, and its user can change both. It counts
only the bytes of compressed chunks after decompressing them; uncompressed payloads are part of the
input, whose size the caller already knows.

### MessagePack limits

A reader MAY limit the nesting depth of MessagePack payloads to protect itself. A limit MUST be at
least 128 levels; the data defined by this specification nests far less deeply.

Writers MUST NOT write bytes after the MessagePack value of a payload. A reader MAY either reject a
payload that has bytes after its value or ignore those bytes. *Informative:* the reference reader
ignores them; a stricter reader that rejects them is also conforming.

## Textures and normal maps

- The `i`-th `TEXR` chunk in table order is the image of the `i`-th entry of the model's (or
  module's) `textures`. The number of `TEXR` chunks MUST equal the number of `textures`.
- Normal maps are stored in `NRML` chunks, one for each texture that has a `normal_map`, in the order
  of those textures. The number of `NRML` chunks MUST equal the number of textures with a
  `normal_map`. To find the normal map of texture `i`: if texture `i` has no `normal_map`, it has
  none; otherwise count the textures before `i` that have a `normal_map`; that count is the index of
  its `NRML` chunk.
- Several textures may name the same normal map; the container still holds one `NRML` chunk per
  such texture, and each texture uses its own chunk. Writers SHOULD store the same image in the
  chunks of textures that name the same normal map.
- The image format is determined by the first bytes of the payload:

  | Format | Signature |
  | --- | --- |
  | PNG | `89 50 4E 47 0D 0A 1A 0A` |
  | KTX2 | `AB 4B 54 58 20 32 30 BB 0D 0A 1A 0A` |

  A payload with neither signature makes the file invalid.
- The pixel data of textures is non-premultiplied RGBA in both formats; how renderers treat KTX2
  data (premultiplied flag, transfer function, compression) is defined in
  [rendering.md](rendering.md#textures). The encoding of normals in normal maps is
  defined in [rendering.md](rendering.md#normals).

## Metadata

The `META` chunk holds free-form metadata about the file, such as authorship or license notes, as a
JSON value. This specification does not define its contents. The avatar license terms that
applications act on are not stored here but in the model's avatar profile
([model.md](model.md#avatar-profile)).

## Bundle files

A bundle (`.ugoe`) contains:

- exactly one `MODL`;
- at most one each of `PHYS`, `META` and `FEAT`;
- any number of `MOTN` and `EXPR`, whose table order is the order of the motions and expressions;
- the `TEXR` and `NRML` chunks of the model's textures;
- no `MODU`.

The physics settings, if present, belong to the model and MUST be valid for it
([model.md](model.md#physics-validation)).

Writers SHOULD order the chunk table as `FEAT`, `MODL`, `PHYS`, `MOTN`…, `EXPR`…, `META`, `TEXR`…,
`NRML`…, and SHOULD omit `FEAT` when there is nothing to declare.

## Module files

A module file (`.ugoemod`) contains:

- exactly one `MODU`, which holds the whole module including its physics rigs and colliders;
- at most one each of `META` and `FEAT`;
- the `TEXR` and `NRML` chunks of the module's textures;
- no `MODL`, `PHYS`, `MOTN` or `EXPR`.

Writers SHOULD order the chunk table as `FEAT`, `MODU`, `META`, `TEXR`…, `NRML`…. A module file
is recognized by its `MODU` chunk; the use of modules is not declared as a feature.

## Reading

A reader MUST treat its input as untrusted: every violation below is an error that rejects the whole
file, and no input may cause undefined behavior. Readers SHOULD check in this order, because the
feature check is designed to happen before any payload is decoded:

1. **Header.** The file is at least 16 bytes; `magic` is `UGOE`; `version` is a container version the
   reader supports ([Versioning](#versioning)); header `flags` is 0.
2. **Chunk table.** The table fits in the file; for every entry, the reserved flag bits are 0,
   `offset` is a multiple of 16, the payload lies within the file and starts at or after the end of
   the table, and no two payloads overlap. Arithmetic on `offset` and `length` MUST NOT overflow.
3. **Feature declarations.** If there is more than one `FEAT`, the file is invalid. Decode `FEAT` and
   apply the [feature check](#reading-feature-declarations). An unknown required feature rejects the
   file.
4. **Schema version.** Reject a `MODL` or `MODU` whose `schema_version` is greater than the reader
   supports. Readers SHOULD read `schema_version` of the first `MODL` or `MODU` in table order
   (after decompressing it, if it is compressed) before decoding any other payload or the rest of
   that payload, so that a file from a newer, incompatible version reports this error rather than a
   decoding error. The value of the `schema_version` key can be read from the MessagePack map by
   skipping the values of the other keys without interpreting them (within the nesting limit the
   reader applies). If it cannot be read this way, the reader
   continues and the next step reports the problem; the reader MUST still check `schema_version`
   after decoding.
5. **Chunks.** Walk the table in order. Skip unknown chunks. Reject chunks that are not allowed in
   this kind of file, and duplicates of chunks that may appear at most once. Decompress and decode
   the others. A payload that fails to decompress or decode is an error, and so is a payload that
   exceeds a [resource limit](#resource-limits) of the reader.
6. **Counts.** The required chunk (`MODL` or `MODU`) is present; the numbers of `TEXR` and `NRML`
   chunks match the textures ([Textures and normal maps](#textures-and-normal-maps)); every image has
   a PNG or KTX2 signature.
7. **Validation.** The model satisfies the [model validation rules](model.md#model-validation), and the
   physics settings, if any, the [physics validation rules](model.md#physics-validation). A module
   satisfies the [module validation rules](model.md#module-validation).
8. **Warnings.** The reader SHOULD report unknown optional features and the locations of
   [unknown values](model.md#unknown-values) to the caller.

## Writing

- A writer MUST produce a file that satisfies every rule of [Reading](#reading) for a reader that
  knows everything the writer knows.
- A writer MUST NOT write [unknown values](model.md#unknown-values) ([Re-saving](#re-saving)).
- A writer MUST declare, in `FEAT`, every non-baseline feature that the file uses
  ([Feature declarations](#feature-declarations)).

## Versioning

Three independent version numbers apply ([README.md](README.md#versions)):

| Version | Where | Meaning |
| --- | --- | --- |
| Container version | Header `version` | The byte layout of the container (header, chunk table, alignment, flags). This draft defines 0. A reader MUST reject a container version it does not support. |
| Schema version | `schema_version` of a model or module | The data schema of [model.md](model.md). This draft defines 0. A reader MUST reject a schema version greater than it supports. A reader that supports an older schema version MAY read files of that version by converting them. |
| Profile version | `profile_version` of an avatar profile | The avatar profile rules. A newer profile version is not an error ([avatar-profile.md](avatar-profile.md#versions)). |

- Before version 1.0 of this specification, `schema_version` remains 0 and compatibility is not
  guaranteed between drafts.
- From version 1.0 on, capabilities are added as features ([Forward compatibility](#forward-compatibility)),
  and `schema_version` is raised only for changes that cannot be expressed as a feature, not even a
  required one.

## Forward compatibility

The goal of this section is that a runtime that cannot be updated (for example, one shipped inside a
game) can still load a file written for a newer version: it degrades the parts it does not
understand in a defined way instead of refusing the whole file.

Four mechanisms work together:

| Mechanism | Handles |
| --- | --- |
| [Unknown chunks](#chunk-types) are skipped. | New kinds of data in their own chunk. |
| [Unknown fields](#unknown-fields) are ignored. | New optional data inside existing objects. |
| [Unknown values](model.md#unknown-values) of extensible enumerations get a fallback. | New variants (blend modes, deformer kinds, …). |
| [Feature declarations](#feature-declarations) | Telling the reader what it is missing, and refusing files that cannot be degraded safely. |

### Unknown fields

A reader MUST ignore object fields it does not know ([model.md](model.md#objects)). A new version
adds data as new fields whose absence means "not used"; an older reader therefore behaves as if the
data were absent. The fallback of such a feature is designed so that this is acceptable.

### Features

A **feature** is a capability of the format with a stable ID (for example `mask.intersection`).
Every feature has a **fallback**: what a reader that does not know the feature does with data that
uses it. A feature for which no acceptable fallback exists is declared as required.

**Baseline features** are the features of version 1.0 of this specification. Every conforming 1.0
reader supports them, so they are never declared. Features added after 1.0 are declared by the
files that use them.

### Feature declarations

The `FEAT` chunk declares the non-baseline features a file uses. Its payload is a MessagePack map:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `features` | `FeatureDecl[]` | `[]` | The declarations. |

`FeatureDecl`:

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | `string` | — | Feature ID. |
| `required` | `bool` | `false` | If `true`, a reader that does not know the feature MUST refuse the file. If `false`, it applies the feature's fallback and continues. |

```
{ "features": [ { "id": "lighting.foo", "required": false },
                { "id": "physics.bar",  "required": true } ] }
```

- A writer MUST declare every non-baseline feature the file uses, and MUST declare a feature as
  required if the file cannot be displayed acceptably with its fallback. A writer MUST NOT declare
  baseline features. A writer SHOULD warn its user when it writes a required feature, because older
  runtimes will refuse the file.
- A writer SHOULD omit the `FEAT` chunk when it declares nothing.
- Declaring a feature that the file does not actually use is allowed but pointless.
- Feature IDs are compared as exact strings.
- Feature IDs that start with `test.` are reserved for testing readers and writers (for example,
  `test.future_optional`). No real feature will use them.

#### Reading feature declarations

1. Collect the declared IDs that the reader does not know. (Knowing a feature means supporting it;
   the baseline features are always known to a 1.0 reader.)
2. If any unknown ID is declared with `required: true` anywhere in the table (an ID declared both as
   required and as optional counts as required), refuse the file with an error that lists the
   unknown required IDs.
3. Otherwise, continue reading. Apply the fallback of each unknown optional feature, and SHOULD
   report each of them (once) as a warning.

The feature check happens before any other payload is decoded (step 3 of [Reading](#reading)), so a
file whose required feature changes the shape of the data is refused with a clear error rather than
a decoding error.

### Baseline features

The following features are the baseline of version 1.0 and are never declared. The fallbacks are
listed for readers built before 1.0 and to document the degradation each feature was designed for.

| Feature ID | Capability | Fallback |
| --- | --- | --- |
| `authoring.keyform_origins` | Origins and inference settings of keyforms ([model.md](model.md#keyform-grid-authoring-fields)) | Ignore them (they do not affect the runtime). |
| `mask.intersection` | `intersection` mask mode ([model.md](model.md#masks)) | Treat as `union`. |
| `physics.dynamic` | Physics outputs to deformers and vertices, and colliders ([model.md](model.md#physics-output-targets)) | Ignore outputs to targets other than parameters, and ignore colliders. |
| `lighting.normal_map` | Normal maps and lighting ([model.md](model.md#normal-maps-and-lighting)) | Draw without lighting. |
| `modules` | Dress-up slots and modules ([model.md](model.md#slots)) | Ignore the slots and show the model alone. |
| `avatar.profile` | Avatar profile ([model.md](model.md#avatar-profile)) | Ignore it (application integration is unavailable). |
| `deform.blend_shape` | Blend-shape grids ([model.md](model.md#blend-shapes)) | Ignore blend-shape grids (the shape is that of blend-shape parameters at their defaults). |
| `deform.smooth_interpolation` | `smooth` keyform interpolation ([model.md](model.md#keyform-grids)) | Interpolate linearly (the same shapes at the keys; straight paths between them). |
| `physics.additive_output` | `add` physics output blend ([model.md](model.md#physics-settings)) | Treat as `overwrite`. |

### Features after 1.0

Features added after version 1.0 are listed here with their fallbacks. Adding a feature to the
format requires adding a row to this table.

| Feature ID | Capability | Fallback | Declared as |
| --- | --- | --- | --- |
| (none yet) | | | |

### Re-saving

Rules for tools that read a file and write it back:

- **Unknown values cannot be written.** Their original content was discarded when they were read.
  A writer MUST NOT write them, and MUST NOT silently replace them with their fallback or any other
  value. A tool SHOULD ask its user before replacing them.
- **Data of unknown optional features is lost.** It was ignored when the file was read (unknown
  fields and chunks are not kept), so the written file is a file that does not use the feature, with
  the fallback in effect. A writer MUST NOT copy declarations of features it does not know into the
  file it writes; it declares only the features the written data uses.

## Standalone JSON files

Physics settings, motions and expressions may also be stored as standalone JSON documents
(`*.physics.json`, `*.motion.json`, `*.expression.json`), encoded as defined in
[model.md](model.md#data-conventions) with the object types `PhysicsSettings`, `Motion` and
`Expression`. A model and a module may likewise be stored as JSON documents of type `Model` and
`Module`.

- The JSON documents are UTF-8 ([RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)).
- The rules for [unknown fields](#unknown-fields) and [unknown values](model.md#unknown-values)
  apply. JSON documents carry no feature declarations.
- A reader of a model or module JSON document MUST check `schema_version` as for a bundle. It
  SHOULD validate the data before evaluating it; physics settings can only be validated together
  with the model they belong to.
- Writers SHOULD produce stable output (for example, fixed field order and indentation), so that
  documents kept in version control show meaningful differences.
