# JSON Schema

**ugoe Specification 0.1 — Draft.** See [../README.md](../README.md) for the status, conventions and
license of this specification.

This directory contains [JSON Schema](https://json-schema.org/) (draft 2020-12) documents for the
JSON encodings of the data defined in [model.md](../model.md).

| File | Validates | Root type |
| --- | --- | --- |
| [model.schema.json](model.schema.json) | A model (`model.json`, the `MODL` chunk decoded to JSON) | [`Model`](../model.md#model) |
| [module.schema.json](module.schema.json) | A dress-up module | [`Module`](../model.md#modules) |
| [physics.schema.json](physics.schema.json) | Physics settings (`*.physics.json`) | [`PhysicsSettings`](../model.md#physics-settings) |
| [motion.schema.json](motion.schema.json) | A motion (`*.motion.json`) | [`Motion`](../model.md#motions) |
| [expression.schema.json](expression.schema.json) | An expression (`*.expression.json`) | [`Expression`](../model.md#expressions) |

## Status

The schemas are **informative**. [model.md](../model.md) and [bundle.md](../bundle.md) are
normative; where a schema and the text disagree, the text is correct.

- Each schema describes the data that a writer of this version of the specification produces. It
  accepts `schema_version` 0 only (`"const": 0`), every value of every enumeration defined by this
  version, and absent optional fields.
- The schemas do **not** describe everything a reader must accept. A reader MUST also accept
  *unknown values* of extensible enumerations ([model.md](../model.md#unknown-values)), and it MUST
  ignore unknown fields ([bundle.md](../bundle.md#unknown-fields)); the schemas reject unknown values
  and allow unknown fields. They do not check the rules in
  [Validation](../model.md#validation) (references between elements, array lengths, ranges).
- `$defs` names follow the type names of [model.md](../model.md). A keyform grid is generic in
  the text (`KeyformGrid<T>`); the schemas give each instance its own definition (`KeyformGrid`,
  `KeyformGrid2`, …).
- Numbers carry `"format"` annotations for their type in the text: `float` for `f32`, and
  `uint8` … `uint64` for the unsigned integer types. Validators that do not know these formats
  ignore them; the integer ranges are also given by `minimum` (and `maximum` where it matters).
- An enumeration of strings is always written as `oneOf` a list of `{"type": "string", "const": …}`,
  one per value, not with the `enum` keyword.
- The schemas carry no descriptions. The meaning of each field is in [model.md](../model.md).
- `$id` is an identifier for the schema of this version. It points to the file in the repository
  while the specification is a draft; the location will be fixed with version 1.0.

## Generation

The schemas are generated from the data types of the ugoe reference implementation and are not
edited by hand. The implementation's test suite checks that the checked-in files are current, and
that the [examples](../examples/README.md) validate against them.

## License

The schema files are part of the ugoe Specification and are licensed under
[CC BY 4.0](../LICENSE), like the rest of it (each file says so in `$comment`). If you embed them in
your program, keep a notice of their origin and license with them, as described in
[README.md](../README.md#license).
