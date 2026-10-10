# ugoe Avatar Profile

**ugoe Specification 0.1 — Draft. Avatar profile version 0.** See [README.md](README.md) for the
status, conventions and license of this specification.

The avatar profile is a set of agreements that let any application drive any conforming 2D avatar
in the same way: which parameters an avatar has and what they mean, how face tracking values are
turned into parameter values, what an application may learn about the avatar (gaze, hit areas,
slots), and under which terms the author allows the avatar to be used.

A model carries its profile in the `avatar` field ([model.md](model.md#avatar-profile)), which
defines the data: field names, types, defaults and encodings. This document defines what that data
means, what a model needs in order to conform, and what applications do with it.

## Contents

- [Overview](#overview)
- [Parameters](#parameters)
- [Conformance levels](#conformance-levels)
- [Face tracking](#face-tracking)
- [Audio](#audio)
- [Gaze](#gaze)
- [Hit areas](#hit-areas)
- [Slots](#slots)
- [Metadata and license terms](#metadata-and-license-terms)
- [Rules for applications](#rules-for-applications)
- [Versions](#versions)

## Overview

*This section is informative.*

- An **avatar** is a model whose `avatar` field is present. A model without it conforms to no level
  of this profile, but applications can still drive whatever well-known parameters it has.
- A **conforming model** has the parameters its claimed [level](#conformance-levels) requires and
  the required metadata, and passes the [conformance checks](#conformance-checks).
- An **application** is a program that drives an avatar: tracking software, a streaming tool, a
  game, a viewer. Applications read the profile, compute parameter values from tracking and audio,
  show the metadata, and warn their users about uses the license terms do not allow
  ([Rules for applications](#rules-for-applications)).
- A **validator** is a program that checks a model against this profile.

In this document, "the model has parameter `X`" means that the model's `parameters` contain a
parameter whose `id` is `X`.

## Parameters

### Orientation and sides

- **Left and right (`L` / `R`, `Left` / `Right`) are the character's own left and right.** A
  character that faces the viewer has its `EyeL` on the right side of the screen.
- `AngleX` is positive when the character turns to its own left (toward the right of the screen).
  `AngleY` is positive when it looks up. `AngleZ` is positive when the head tilts clockwise as seen
  on the screen.
- The same signs apply to `BodyAngleX`, `BodyAngleY` and `BodyAngleZ`, to the gaze (`EyeBallX`
  positive toward the character's own left, `EyeBallY` positive up), and to every parameter below
  that moves something sideways (positive toward the character's own left).
- Open/close parameters are 0 when closed and 1 when open. Parameters with a range of −1 to 1 are
  neutral at 0. Parameters with a range of 0 to 1 other than open/close parameters are neutral at 0
  and fully expressed at 1.

### Standard parameters

The ranges below are those of the well-known parameter IDs
([model.md](model.md#well-known-parameter-ids)). A parameter with one of these IDs SHOULD have
exactly this minimum, default and maximum; a different range is a
[warning](#conformance-checks), not an error.

| ID | min / default / max | Meaning |
| --- | --- | --- |
| `AngleX` | −30 / 0 / 30 | Head yaw in degrees. Positive: the character turns to its own left. |
| `AngleY` | −30 / 0 / 30 | Head pitch in degrees. Positive: up. |
| `AngleZ` | −30 / 0 / 30 | Head roll in degrees. Positive: clockwise on screen. |
| `BodyAngleX` | −10 / 0 / 10 | Body yaw in degrees, same sign as `AngleX`. |
| `BodyAngleY` | −10 / 0 / 10 | Body pitch in degrees, same sign as `AngleY`. |
| `BodyAngleZ` | −10 / 0 / 10 | Body roll in degrees, same sign as `AngleZ`. |
| `Breath` | 0 / 0 / 1 | Breathing. 0: breathed out (rest), 1: breathed in. |
| `EyeLOpen`, `EyeROpen` | 0 / 1 / 1 | Opening of the character's left and right eye. 0: closed, 1: normally open. |
| `EyeLSmile`, `EyeRSmile` | 0 / 0 / 1 | Narrowing of the eye in a smile (squint). |
| `EyeBallX` | −1 / 0 / 1 | Horizontal gaze. Positive: the character looks to its own left. |
| `EyeBallY` | −1 / 0 / 1 | Vertical gaze. Positive: up. |
| `BrowLY`, `BrowRY` | −1 / 0 / 1 | Height of the left and right eyebrow. Positive: raised. |
| `MouthForm` | −1 / 0 / 1 | Shape of the mouth. Positive: smile (corners up), negative: frown (corners down). |
| `MouthOpenY` | 0 / 0 / 1 | Opening of the mouth. 0: closed. |
| `Cheek` | 0 / 0 / 1 | Blush. |
| `HairFront`, `HairSide`, `HairBack` | −1 / 0 / 1 | Sway of the front, side and back hair. Usually driven by physics, not by tracking. |

### Extended parameters

Extended parameters express facial detail that face trackers can measure. They are needed only for
[level 3](#conformance-levels).

| ID | min / default / max | Meaning |
| --- | --- | --- |
| `EyeLWide`, `EyeRWide` | 0 / 0 / 1 | Widening of the eye beyond its normal opening. |
| `EyeLSmile`, `EyeRSmile` | 0 / 0 / 1 | Narrowing of the eye in a smile (also a standard parameter). |
| `BrowInnerUp` | 0 / 0 / 1 | Raising of the inner ends of both eyebrows (a worried look). |
| `BrowLAngle`, `BrowRAngle` | −1 / 0 / 1 | Slant of the eyebrow. Positive: the outer end goes up. |
| `MouthX` | −1 / 0 / 1 | Sideways position of the mouth. Positive: toward the character's own left. |
| `JawX` | −1 / 0 / 1 | Sideways position of the jaw. Positive: toward the character's own left. |
| `MouthPucker` | 0 / 0 / 1 | Pursing the lips (a kiss). |
| `MouthFunnel` | 0 / 0 / 1 | Pushing the lips forward in a round shape. |
| `MouthPress` | 0 / 0 / 1 | Pressing the lips together. |
| `MouthShrug` | 0 / 0 / 1 | Pushing the lips upward (a pout). |
| `CheekPuff` | 0 / 0 / 1 | Puffing out the cheeks. |
| `TongueOut` | 0 / 0 / 1 | Sticking out the tongue. |
| `NoseSneer` | 0 / 0 / 1 | Wrinkling the nose. |

These are 16 parameter IDs; `EyeLSmile` and `EyeRSmile` are both standard and extended.

### Body extended parameters

Body extended parameters express movements of the upper body that body trackers can measure. They
are optional at every level and are not counted among the extended parameters of
[level 3](#conformance-levels).

| ID | min / default / max | Meaning |
| --- | --- | --- |
| `ShoulderLY`, `ShoulderRY` | −1 / 0 / 1 | Height of the character's own left and right shoulder relative to its body. 0: relaxed, 1: fully raised (a shrug), negative: lowered. |

Tilting the whole upper body is `BodyAngleZ`, not a difference between the two shoulders.

### Vowel parameters

| ID | min / default / max | Meaning |
| --- | --- | --- |
| `MouthVowelA`, `MouthVowelI`, `MouthVowelU`, `MouthVowelE`, `MouthVowelO` | 0 / 0 / 1 | Mouth shape of the vowel /a/, /i/, /u/, /e/, /o/. |

Vowel parameters are optional at every level ([Audio](#audio)).

### Stability

The IDs, ranges and meanings in this section do not change within a profile version. New IDs MAY be
added, either in a later profile version or, when no level requires them, within a profile version
(as the body extended parameters were added to profile version 0). Until version 1.0 of this
specification, the draft rules of [README.md](README.md) apply.

## Conformance levels

A model claims a conformance level in `avatar.level`. Each level requires everything the lower
levels require.

| Level | Name | Required |
| --- | --- | --- |
| 1 | Basic | Metadata (`meta.name` and `meta.authors`, [below](#metadata-and-license-terms)); head orientation `AngleX`, `AngleY`, `AngleZ`; eye opening `EyeLOpen`, `EyeROpen`; mouth opening `MouthOpenY`. |
| 2 | Standard | Level 1, and gaze `EyeBallX`, `EyeBallY`; eyebrows `BrowLY`, `BrowRY`; mouth shape `MouthForm`; body orientation `BodyAngleX`, `BodyAngleY`, `BodyAngleZ`; `Breath`. |
| 3 | Expressive | Level 2, and at least 8 of the 16 [extended parameters](#extended-parameters). |

- The metadata requirement is met when `meta.name` contains a character other than white space and
  `meta.authors` contains at least one entry that contains a character other than white space.
- Profile version 0 defines levels 1 to 3. `level` values outside this range are errors.
- A model without the `avatar` field conforms to no level.

### Conformance checks

A model **conforms** to its claimed level when none of the checks below marked *error* fails.
Checks marked *warning* do not prevent conformance; they point to data that some applications may
not handle as the author expects. A validator SHOULD report every failed check.

| Check | Severity |
| --- | --- |
| The model has the `avatar` field. | error |
| `level` is between 1 and 3. | error |
| The metadata requirement above is met. | error |
| The model has every parameter that the claimed level requires (for level 3, at least 8 extended parameters). If `level` is greater than 3, the checks of level 3 apply. | error |
| For each element of `tracking` whose `source` is known: `input` is a known input for that `source` ([Overrides](#overrides)); the parameter it writes is determined (`parameter` is present, or `source` is `standard`) and the model has it. For each element of `tracking`, including those whose `source` is unknown: `scale` and `offset` are finite. | error |
| If `look_at` is present: the model has its `x` and `y` parameters, and `range_x` and `range_y` are finite and greater than 0. | error |
| The `name` values of `hit_areas` are unique, and each `hit_area` is the ID of one of the model's hit areas. | error |
| Each element of `slots` is the ID of one of the model's slots. | error |
| `meta.thumbnail`, if present, is an index into the model's `textures`. | error |
| Every parameter whose ID is a standard, extended, body extended or vowel parameter has the minimum, default and maximum listed in [Parameters](#parameters). | warning |
| `profile_version` is not greater than the version the validator supports (0 in this document). A validator that meets a greater version SHOULD say that its checks may be incomplete. | warning |
| The profile contains no unknown value (an unknown license term or an unknown `tracking` `source`). | warning |
| No parameter that tracking or gaze writes is `internal` ([model.md](model.md#parameters)): a parameter of the [standard mapping](#standard-mapping) that the model has, the parameter written by an element of `tracking`, and the `x` and `y` parameters of `look_at`. Tracking never writes internal parameters. | warning |

Every `f32` of the profile (`scale` and `offset` of each `tracking` element, and `range_x` and
`range_y` of `look_at`) MUST be finite, as in [model.md](model.md#data-conventions); a validator
reports a NaN or an infinity as an error together with its location (for example
`avatar.tracking[0].scale`). This rule does not depend on the version, so it also applies to
elements whose `source` is unknown. Apart from `range_x` and `range_y`, the profile defines no
range for its numbers: a `scale` of 0 or less is valid.

A validator MAY also report the **satisfied level**: the highest level whose metadata and parameter
requirements the model meets, regardless of the claimed level, or 0 if it meets none (for example
when the metadata requirement is not met). The satisfied level is informative; conformance is
always judged against the claimed level.

The avatar profile is not part of the validation rules that decide whether a model can be loaded
([model.md](model.md#validation)). A runtime MUST NOT reject a model because it fails a conformance
check.

## Face tracking

This section defines how an application turns the output of a face tracker into parameter values.

### Tracking input

The input of one frame consists of:

- **Head rotation**: `yaw`, `pitch` and `roll` in degrees, with the signs of `AngleX`, `AngleY` and
  `AngleZ` ([Orientation and sides](#orientation-and-sides)). Seen from the tracked person: yaw is
  positive when they turn to their own left, pitch is positive when they look up, and roll is
  positive when their head tilts clockwise in an unmirrored image of them.
- **Head rotation** is the orientation of the head relative to the camera, not relative to the body.
- **Body rotation** (optional): the same three angles for the upper body, relative to the camera.
  Each of the three angles may be absent on its own (for example, a tracker that cannot see the
  hips may provide yaw and roll but no pitch).
- **Shoulder raise** (optional): for the tracked person's own left and right shoulder, 0 when
  relaxed, 1 when fully raised (a shrug) and negative when lowered, relative to the body. Each may
  be absent on its own.
- **Blend shapes**: the 52 face blend shapes of Apple ARKit, identified by their ARKit names (for
  example `eyeBlinkLeft`), each nominally 0 to 1.

`Left` and `Right` in blend shape names are the tracked person's own left and right. This profile
maps them to the character's own left and right: when the person closes their own left eye, the
character closes its own left eye, and when the person turns to their own left, the character turns
to its own left. On the screen, the character therefore moves like the person seen through a
camera, not like the person's mirror image. An application that shows the character as a mirror
image of the user does so by flipping the display, or by swapping the inputs
([Driving the model](#driving-the-model)).

The 52 names, in the order ARKit lists them:

`eyeBlinkLeft`, `eyeLookDownLeft`, `eyeLookInLeft`, `eyeLookOutLeft`, `eyeLookUpLeft`,
`eyeSquintLeft`, `eyeWideLeft`, `eyeBlinkRight`, `eyeLookDownRight`, `eyeLookInRight`,
`eyeLookOutRight`, `eyeLookUpRight`, `eyeSquintRight`, `eyeWideRight`, `jawForward`, `jawLeft`,
`jawRight`, `jawOpen`, `mouthClose`, `mouthFunnel`, `mouthPucker`, `mouthLeft`, `mouthRight`,
`mouthSmileLeft`, `mouthSmileRight`, `mouthFrownLeft`, `mouthFrownRight`, `mouthDimpleLeft`,
`mouthDimpleRight`, `mouthStretchLeft`, `mouthStretchRight`, `mouthRollLower`, `mouthRollUpper`,
`mouthShrugLower`, `mouthShrugUpper`, `mouthPressLeft`, `mouthPressRight`, `mouthLowerDownLeft`,
`mouthLowerDownRight`, `mouthUpperUpLeft`, `mouthUpperUpRight`, `browDownLeft`, `browDownRight`,
`browInnerUp`, `browOuterUpLeft`, `browOuterUpRight`, `cheekPuff`, `cheekSquintLeft`,
`cheekSquintRight`, `noseSneerLeft`, `noseSneerRight`, `tongueOut`.

A blend shape the tracker does not provide has the value 0. The mapping uses the input values as
they are: it does not clamp them to 0 to 1 and does not smooth or calibrate them.

### Standard mapping

The standard mapping gives a formula for each parameter below. In the formulas, a blend shape name
stands for its value, and `head.yaw` etc. for the head rotation.

**Head and body:**

| Parameter | Formula |
| --- | --- |
| `AngleX` | `head.yaw` |
| `AngleY` | `head.pitch` |
| `AngleZ` | `head.roll` |
| `BodyAngleX` | `body.yaw` if the input has a body rotation, otherwise `0.3 × head.yaw` |
| `BodyAngleY` | `body.pitch` if the input has a body rotation, otherwise `0.3 × head.pitch` |
| `BodyAngleZ` | `body.roll` if the input has a body rotation, otherwise `0.3 × head.roll` |
| `ShoulderLY` | the left shoulder raise if the input has it, otherwise no value |
| `ShoulderRY` | the right shoulder raise if the input has it, otherwise no value |

Angles map one to one: a head yaw of 12 degrees gives `AngleX = 12`. "If the input has a body
rotation" applies to each angle separately. The head angles are not reduced by the body rotation:
`AngleX` is the head's orientation relative to the camera whether or not the body is tracked, as it
is without body tracking, where the body adds `0.3 × head.yaw` on top.

This is a trade-off. In most rigs the head is a child of the body, so when the person turns their
whole upper body, the head on screen turns by the body angle and the head angle together, more than
the person's head did (by up to the range of `BodyAngleX`, 10 degrees). The head angles are not
reduced because the body rotation estimated from a single camera can be off by 10 to 15 degrees,
and subtracting it would move that error into the most visible parameters (`AngleX` etc.), while the
overlap is bounded by the narrower range of the body angles. A model that wants the head relative to
the body can express it with [overrides](#overrides), for example by adding the `standard` input
`BodyAngleX` with a `scale` of −1 to the `head` input `yaw`. An application MAY offer its user an
option to subtract the body rotation from the head rotation before the mapping; it SHOULD NOT do so
by default.

**Blend shapes:**

| Parameter | Formula |
| --- | --- |
| `EyeLOpen` | `1 − eyeBlinkLeft` |
| `EyeROpen` | `1 − eyeBlinkRight` |
| `EyeLWide` | `eyeWideLeft` |
| `EyeRWide` | `eyeWideRight` |
| `EyeLSmile` | `eyeSquintLeft` |
| `EyeRSmile` | `eyeSquintRight` |
| `EyeBallX` | `((eyeLookOutLeft − eyeLookInLeft) − (eyeLookOutRight − eyeLookInRight)) / 2` |
| `EyeBallY` | `(eyeLookUpLeft + eyeLookUpRight − eyeLookDownLeft − eyeLookDownRight) / 2` |
| `BrowLY` | `(browOuterUpLeft + browInnerUp) / 2 − browDownLeft` |
| `BrowRY` | `(browOuterUpRight + browInnerUp) / 2 − browDownRight` |
| `BrowInnerUp` | `browInnerUp` |
| `BrowLAngle` | `browOuterUpLeft − browInnerUp` |
| `BrowRAngle` | `browOuterUpRight − browInnerUp` |
| `MouthOpenY` | `max(0, jawOpen − mouthClose)` |
| `MouthForm` | `(mouthSmileLeft + mouthSmileRight − mouthFrownLeft − mouthFrownRight) / 2` |
| `MouthX` | `mouthLeft − mouthRight` (see the [open issue](#open-issue-sign-of-mouthx-and-jawx)) |
| `JawX` | `jawLeft − jawRight` (see the [open issue](#open-issue-sign-of-mouthx-and-jawx)) |
| `MouthPucker` | `mouthPucker` |
| `MouthFunnel` | `mouthFunnel` |
| `MouthPress` | `(mouthPressLeft + mouthPressRight) / 2` |
| `MouthShrug` | `(mouthShrugUpper + mouthShrugLower) / 2` |
| `CheekPuff` | `cheekPuff` |
| `TongueOut` | `tongueOut` |
| `NoseSneer` | `(noseSneerLeft + noseSneerRight) / 2` |

`eyeLookOutLeft` means the left eye looks outward, that is, to the person's own left; `EyeBallX` is
therefore positive when the person looks to their own left, the same direction as `AngleX`.

These 32 parameters are the **tracked parameters**. The standard mapping does not use `jawForward`,
`mouthRollLower`, `mouthRollUpper`, `mouthDimpleLeft`, `mouthDimpleRight`, `mouthStretchLeft`,
`mouthStretchRight`, `mouthLowerDownLeft`, `mouthLowerDownRight`, `mouthUpperUpLeft`,
`mouthUpperUpRight`, `cheekSquintLeft` or `cheekSquintRight`. A model can map them to its own
parameters with [overrides](#overrides).

#### Open issue: sign of `MouthX` and `JawX`

> **Unresolved in this draft.** The formulas for `MouthX` and `JawX` assume that `mouthLeft` and
> `jawLeft` rise when the tracked person moves their mouth or jaw toward their own left, as the
> `Left` eye blend shapes refer to the person's own left eye. This has not yet been verified with
> a real camera and tracker. The meaning of the parameters (positive toward the character's own
> left) is settled; the sign of these two formulas may be reversed in a later draft. Until then,
> a model or an application that finds the direction reversed can correct it with an override,
> for example `{ "source": "standard", "input": "MouthX", "scale": -1 }`.

### Overrides

A model adjusts the standard mapping with the elements of `avatar.tracking`
([model.md](model.md#avatar-profile)). Each element reads one **input** and writes
`value × scale + offset` to one parameter. The input depends on `source`:

| `source` | `input` | Input value |
| --- | --- | --- |
| `standard` | The ID of a [tracked parameter](#standard-mapping). The model need not have that parameter. | The result of its standard formula, before clamping, or no value if the formula gives none. |
| `blend_shape` | One of the 52 ARKit blend shape names. | The value of that blend shape. |
| `head` | `yaw`, `pitch` or `roll`. | That angle of the head rotation in degrees (never the body rotation; use the `standard` inputs `BodyAngleX`, `BodyAngleY` and `BodyAngleZ` for the body). |

The parameter an element writes (its **target**) is `parameter` if present. If `parameter` is
absent and `source` is `standard`, the target is `input`. Otherwise the element has no target.

An element is **ignored** — it writes nothing and replaces nothing — when any of these holds:

- `source` is an unknown value (a file written for a later version);
- `input` is not a known input for its `source` (table above);
- it has no target, or the model does not have the target parameter;
- `scale` or `offset` is not finite.

A validator reports the last three as errors (and a non-finite `scale` or `offset` also when `source` is unknown) ([Conformance checks](#conformance-checks)), but an
application MUST ignore such elements and go on.

### Computing the parameter values

For each frame, an application computes the values as follows. The result is the same regardless
of the order in which the elements of `tracking` appear.

1. **Overrides.** For each target parameter of the elements that are not ignored, compute
   `input value × scale + offset` of each such element that writes it, and add them up. That sum is
   the value of the target. If the input of any of these elements has no value, the target gets no
   value.
2. **Standard mapping.** For each tracked parameter `P` that the model has, the value is the result
   of `P`'s formula (which may be no value), unless `P` is *replaced*. `P` is replaced when an element that is not ignored
   has `source` `standard` and `input` `P`, or has `P` as its target. A replaced parameter gets a
   value only from step 1 (if it is a target) or none at all.
3. **Exclusions.** Discard the value of every parameter that is `internal`
   ([model.md](model.md#parameters)), and every value that is not finite (for example, when an
   input is NaN). Such parameters are not written. An element whose target is `internal` still
   replaces standard mappings as described in step 2.
4. **Clamp and write.** Clamp each remaining value to its parameter's `[min, max]` and write it.
   Parameters that received no value are not written by tracking; they keep the value they have
   from other sources.

*Informative:* where tracking values enter the per-frame order of operations (after motions, before
expressions, eye blink, breathing, physics and lip sync, which are layered on top of the tracked
values) is defined in [motion.md](motion.md#order-of-operations-in-a-frame).

Example (JSON):

```json
"tracking": [
  { "source": "standard", "input": "EyeBallX", "scale": -1 },
  { "source": "standard", "input": "MouthForm", "parameter": "MouthSmile" },
  { "source": "blend_shape", "input": "jawForward", "parameter": "JawForward", "scale": 0.5 }
]
```

- The first element reverses the sign of `EyeBallX`: its target is `EyeBallX` (the `parameter` is
  omitted), and it replaces the standard mapping of `EyeBallX`.
- The second element drives the model's own `MouthSmile` with the value of the `MouthForm` formula,
  and replaces the standard mapping of `MouthForm`, so `MouthForm` is no longer written by
  tracking.
- The third element maps a blend shape that the standard mapping does not use to the model's own
  `JawForward` parameter.

## Audio

- An application that derives mouth movement from audio (lip sync) writes the mouth opening, 0 to
  1, to `MouthOpenY`.
- An application that estimates vowels MAY write their weights, 0 to 1, to the
  [vowel parameters](#vowel-parameters) that the model has. A model without vowel parameters is
  driven by `MouthOpenY` alone.

## Gaze

`avatar.look_at` names the parameters an application uses to make the character look at a point
(for example, the camera, the mouse pointer or another character), and how far the eyes can turn.

- If `look_at` is absent, the gaze parameters are `EyeBallX` and `EyeBallY` with a range of 30
  degrees on both axes, if the model has them.
- `x` is positive when the character looks to its own left, `y` is positive when it looks up.
- `range_x` and `range_y` are angles in degrees. A target that lies `range_x` degrees to the side of
  the character's forward direction moves `x` to the end of its range, and likewise for `y`. An
  application SHOULD map the angle to the parameter linearly: 0 degrees gives the parameter's
  default, `+range` its maximum, `−range` its minimum, and angles beyond the range are clamped.
- Applications MUST NOT write gaze to an `internal` parameter.

When face tracking is active, the tracked gaze (`EyeBallX`, `EyeBallY`) and an application's own
gaze control write the same parameters; the application decides which one wins.

## Hit areas

`avatar.hit_areas` gives standard names to hit areas of the model, so that applications can react
to touches and clicks without knowing the model.

| Name | Meaning |
| --- | --- |
| `Head` | The head (for example, for patting). |
| `Body` | The body. |

- An application tests a standard name by testing the model's hit area that the name is mapped to
  ([model.md](model.md#hit-areas)). A name without a mapping never hits.
- A name appears at most once. A model MAY map names other than those above; applications ignore
  names they do not know.

## Slots

`avatar.slots` lists the IDs of the model's [slots](model.md#slots) that applications may use to
attach dress-up modules (for example, to let a user change the hair style). An application SHOULD
offer only the listed slots to its users; the other slots of the model are for the author's own
use.

## Metadata and license terms

`avatar.meta` holds information about the avatar and the terms under which its author allows it to
be used. The data (types and defaults) is defined in [model.md](model.md#avatar-profile).

### Descriptive fields

| Field | Meaning |
| --- | --- |
| `name` | Name of the avatar. Required for conformance. |
| `version` | Version of the avatar, as the author names it. |
| `authors` | Authors. At least one is required for conformance. |
| `copyright` | Copyright notice. |
| `contact` | How to contact the author (for example, an e-mail address or a URL). |
| `references` | Original works the avatar is based on, or references. |
| `thumbnail` | Index into the model's `textures` of an image that represents the avatar. |
| `license_url` | URL of the author's terms of use, which may state details or conditions beyond the fields below. |
| `third_party_licenses` | License notices of third-party material used in the avatar (fonts, textures, …). |

### License terms

| Field | Values (most restrictive first) | Meaning |
| --- | --- | --- |
| `avatar_permission` | `only_author`: only the authors. `only_licensed`: only persons the author has permitted. `everyone`: anyone. | Who may use the model as their avatar (appear as, or perform as, the character). |
| `commercial_usage` | `personal_non_profit`: individuals, for non-profit purposes only. `personal_profit`: individuals, including for profit. `corporation`: also corporations and other organizations. | Who may use the model commercially. |
| `allow_violent_usage` | `false`, `true` | Use in excessively violent content. |
| `allow_sexual_usage` | `false`, `true` | Use in excessively sexual content. |
| `allow_political_or_religious_usage` | `false`, `true` | Use for political or religious purposes. |
| `allow_antisocial_or_hate_usage` | `false`, `true` | Use for antisocial or hateful purposes. |
| `credit_notation` | `required`, `unnecessary` | Whether a user of the avatar must credit the author. |
| `allow_redistribution` | `false`, `true` | Redistribution of the model (its files) as it is. |
| `modification` | `prohibited`: no modification. `allow`: modification is allowed, but not redistribution of the modified model. `allow_redistribution`: modification and redistribution of the modified model are allowed. | Modification of the model. |

- **Defaults.** A term that is absent has its most restrictive value. A profile with no `meta` at
  all therefore allows only the authors to use the avatar, and allows nothing else.
- **Unknown values.** A reader MUST treat an unknown value of a license term (from a file written
  for a later version) as the most restrictive value. It MUST NOT treat it as more permissive.
- The terms are the author's declaration. This specification does not decide their legal effect,
  and a file format cannot enforce them; the rules below ask applications to make them visible.

## Rules for applications

This section applies to applications that load a model as an avatar.

### What to read

- An application reads `avatar` ([model.md](model.md#avatar-profile)): `profile_version`, `level`,
  `meta`, `tracking`, `look_at`, `hit_areas` and `slots`. It MAY ignore the parts it does not use,
  but it SHOULD always read `meta` so that it can show the license terms
  ([Showing the license terms](#showing-the-license-terms)).
- An application SHOULD NOT rely on the claimed `level`. It drives the tracked parameters that the
  model has, whatever level the model claims, and it MAY check the model itself
  ([Conformance checks](#conformance-checks)).
- An application MUST NOT reject or stop using a model because it fails a conformance check or has
  a newer `profile_version` ([Versions](#versions)).

### Driving the model

An application that drives the model from face tracking MUST compute the parameter values as
defined in [Face tracking](#face-tracking), including the overrides in `tracking`. In particular:

- It gives the tracking input in the conventions of [Tracking input](#tracking-input): angles in
  degrees with the signs of this profile, and blend shapes with the tracked person's own `Left` and
  `Right`. If the tracker receives a mirrored (horizontally flipped) camera image, the application
  swaps every `Left` blend shape with its `Right` counterpart and negates `yaw` and `roll` before
  the mapping. It does the same with the body input: it swaps the left and right shoulder raise and
  negates the body yaw and roll.
- It MUST NOT write parameters that are `internal`, or parameters the model does not have.
- The following are the application's responsibility and are not part of the mapping: converting
  the tracker's output (for example, a rotation matrix) into yaw, pitch and roll; smoothing;
  calibration (subtracting the user's neutral pose and expression); the user's own tuning of the
  input to their face, camera and room (for example, the sensitivity of the head rotation, a dead
  zone around neutral, or the input values at which an eye counts as fully open or closed); and what
  to do when tracking is lost (keep the last values, or return to neutral). This tuning belongs to
  the user, not to the model; a model's own adjustments use [Overrides](#overrides).
- An application that uses the model's automatic eye blink ([model.md](model.md#effects)) while
  tracking eye opening SHOULD let the user turn the blink off, because both act on `EyeLOpen` and
  `EyeROpen`.
- An application that shows the character as the user's mirror image flips the display (or swaps
  the inputs as above); it does not change the mapping.

*Informative, body tracking:* the reference implementation obtains the body input from MediaPipe
Pose Landmarker. It takes the roll from the tilt of the line between the shoulders in the image, the
yaw from the depth of the shoulders in the world landmarks, and the pitch from the direction from
the hips to the shoulders in the world landmarks. It estimates one shoulder raise from the distance
between the ears and the line between the shoulders, and gives the same value to both shoulders,
because raising one shoulder cannot be told apart from tilting the upper body with the shoulder
landmarks alone. It gives the pitch and the shoulder raise only after the user has calibrated a
neutral pose, because their absolute values depend strongly on the camera's height and angle. As
with faces, the landmark names refer to the person's own left and right, and the application swaps
them for a mirrored image. With a single camera, the yaw of the upper body was observed to be off by
10 to 15 degrees in some images, which is one reason the head angles are not reduced by the body
rotation.

*Informative, MediaPipe Face Landmarker:* the blend shape category names of MediaPipe Face
Landmarker are the ARKit names, plus `_neutral`, which the mapping does not use. It provides 51 of
the 52 blend shapes: `tongueOut` is missing, so `TongueOut` does not move with MediaPipe alone. Its
`Left` and `Right` refer to the left and right of the face in the input image as seen by that face,
and swap when the input image is flipped
([google-ai-edge/mediapipe#6368](https://github.com/google-ai-edge/mediapipe/issues/6368)). With an
unflipped camera image, the values therefore follow this profile without swapping. The reference
implementation has confirmed this with images and videos of real faces for the eye blend shapes and
the head rotation: closing the person's own right eye raises `eyeBlinkRight`, and the person looking
to their own right gives a negative `EyeBallX`.

### Showing the license terms

- An application that loads a model for use as an avatar SHOULD show the user the metadata: at
  least `name`, `authors`, `copyright`, `license_url`, `third_party_licenses` and the license
  terms. It SHOULD show them before the first use of a model, and keep them available afterwards.
- An application MUST NOT show a license term as more permissive than its effective value (with
  the defaults and the fallback for unknown values applied).
- An application SHOULD warn the user when a use the application offers is not allowed by the
  terms, at the time the user chooses that use:

  | Term | When to warn |
  | --- | --- |
  | `avatar_permission` is not `everyone` | When the user starts using the model as their avatar: only the authors (`only_author`) or persons the author permitted (`only_licensed`) may. |
  | `commercial_usage` | When the application or the chosen feature serves commercial purposes (for example, monetized streaming or a commercial product) beyond what the value allows. |
  | `allow_violent_usage`, `allow_sexual_usage`, `allow_political_or_religious_usage`, `allow_antisocial_or_hate_usage`: each one that is `false` | When the application or the chosen content is of that kind. |
  | `credit_notation` is `required` | When the user shows or publishes the avatar (streaming, recording, a game). The application SHOULD make the credit (`name`, `authors`, `copyright`) easy to display. |
  | `allow_redistribution` is `false` | Before exporting, uploading or sharing the model's files. |
  | `modification` is `prohibited` | Before saving a modified model. |
  | `modification` is `allow` | Before redistributing a modified model. |

- An application cannot know whether the user is an author or has the author's permission. A
  warning SHOULD state the term and let the user proceed; this specification does not require
  applications to block uses.
- If the model has no `avatar` field, it declares no terms. An application SHOULD tell the user
  that the terms are unknown and MUST NOT present the model as permitting any use.

### Unknown data

- An application MUST ignore elements of `tracking` with an unknown `source` and apply the other
  rules of [Overrides](#overrides) for ignored elements.
- An application MUST apply the most restrictive value for unknown license terms.
- An application SHOULD ignore standard hit area names it does not know.

## Versions

- The version of the rules in this document is the **profile version**. This document defines
  profile version 0. A model states the profile version it follows in `avatar.profile_version`
  (default 0). The correspondence with the specification version and `schema_version` is listed in
  [README.md](README.md#versions).
- The profile version is independent of `schema_version`: a model with a newer profile version is
  still read with the rules of its `schema_version` ([bundle.md](bundle.md#versioning)).
- A reader that meets a `profile_version` greater than it supports MUST NOT treat it as an error.
  It applies the rules it knows; it SHOULD warn that parts of the profile may not be understood.
- Within a profile version, parameter IDs, ranges and meanings, the formulas of the standard
  mapping and the requirements of the levels do not change, except where this draft marks an issue
  as unresolved ([Open issue](#open-issue-sign-of-mouthx-and-jawx)). Optional additions that no
  level requires MAY be made within a profile version: the body tracking input and the
  [body extended parameters](#body-extended-parameters) were added to profile version 0 in this
  way. An application that does not know them does not drive them, and a model that has them is
  read the same way as before. Until version 1.0 of this
  specification, any of them may change between drafts.
