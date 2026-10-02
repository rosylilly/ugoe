# ugoe Studio API reference (version 1)

The ugoe Studio API lets other programs on the same computer — a Stream Deck plugin, a chat bot, a macro tool —
switch scenes, expressions and motions in ugoe Studio, move parameters, dress up the avatar and receive change
notifications. It is **off by default**, listens on the **loopback interface only**, and every new client must be
**approved by the user** in the studio's control window.

- Transport: WebSocket (RFC 6455), text messages, `ws://127.0.0.1:<port>/api` (also `ws://[::1]:<port>/api`).
- Protocol: [JSON-RPC 2.0](https://www.jsonrpc.org/specification), one JSON-RPC message per WebSocket text message.
  Named parameters only (`params` is an object); batches are not supported.
- Messages are described by the JSON Schema [studio-api.schema.json](studio-api.schema.json)
  (`$defs/<method>.params`, `$defs/<method>.result`, `$defs/<event>.event`).
- The API has its own names. It is not compatible with the API of any other product and does not use their
  message names or default ports.

The security rules are decided in [ADR 0028 §6](adr/0028-studio.md) (Japanese). The implementation is
`apps/studio/src-tauri/src/api/`.

## Turning the API on

In the control window, open the **Studio API** panel and switch on *Let other apps on this computer control the
studio*. The panel shows the address (`ws://127.0.0.1:28451/api` by default), the port (1024–65535) and the
**token**. Copy the address and the token into the client. While the API is on, the toolbar shows an **API** badge.

- The default port is **28451**. Change it in the panel if another program uses it.
- The port is shared with the **browser source** page for OBS (`http://127.0.0.1:<port>/source/<token>/`). The
  browser source has its own switch and its own read-only token; while only the browser source is on, `/api` answers
  `404`.
- **Regenerate token** makes the old token useless for new clients. Clients that were already approved keep
  working (they use their own key, see below).
- The token and the client keys are stored in the application data directory (`studio-api.json`), never in a
  profile, so sharing a profile does not leak them.

## Connecting

1. Open a WebSocket to `ws://127.0.0.1:<port>/api`. Non-browser clients should not send an `Origin` header.
2. Call `session.hello` with your client's name and the token:

   ```json
   {"jsonrpc":"2.0","id":1,"method":"session.hello",
    "params":{"name":"My Deck Plugin","token":"<token>","permission":"control"}}
   ```

3. The studio asks the user to approve *My Deck Plugin*. **The response arrives only after the user decides.**
   The user chooses the permission (`read` or `control`; `permission` in the request is only a suggestion).
   On approval the result includes a **`key`**:

   ```json
   {"jsonrpc":"2.0","id":1,"result":{"studioVersion":"1.0.0","apiVersion":1,"client":"5f0c…",
    "permission":"control","events":["scene.changed","…"],"key":"<64 hex digits>"}}
   ```

   If the user denies the client, the response is error `-32002` and the studio closes the connection
   (close code `4003`).
4. **Store the key.** Next time, call `session.hello` with `name` and `key` (no token); the session is ready at once
   and the result has no `key`.

Until `session.hello` succeeds, every other method fails with `-32000`. A wrong token or an unknown key is answered
with `-32002`, and the connection is closed (close code `4003`). At most four clients can wait for approval at the
same time.

The user can see approved clients in the panel, **disconnect** them (the key stays valid) or **revoke** them (the key
is deleted and open connections are closed with code `4001`).

### Permissions

| Permission | Allows |
| --- | --- |
| `read` | `studio.state`, `*.list`, `events.subscribe`, `events.unsubscribe` |
| `control` | Everything in `read`, plus every method that changes the studio |

Calling a `control` method with `read` permission fails with `-32001`.

## Security

- The server binds `127.0.0.1` and `::1` only. There is no setting to listen on other addresses.
- Requests whose `Host` header is not a loopback name (`127.0.0.1`, `[::1]`, `localhost`), and requests carrying
  proxy or tunnel headers (`Forwarded`, `X-Forwarded-For`, `X-Real-IP`, …) are answered with `403`. This blocks
  DNS rebinding and accidental exposure through tunnels.
- A WebSocket request with an `Origin` header is accepted only when the origin is the studio's own page
  (`http://127.0.0.1:<port>`, `http://localhost:<port>`, `http://[::1]:<port>`). Web pages opened in a browser cannot
  connect.
  The browser source page is such a page, but it never calls `/api`; it only reads frames from its own path.
- Tokens and keys are 256-bit random values and are compared in constant time.
- Parameter overrides expire (see `parameter.set`), so a crashed client cannot leave the avatar stuck.

## Messages

Requests without an `id` are JSON-RPC notifications: the studio runs them and sends no response.
A notification caused by a request (for example `scene.changed` after `scene.activate`) may arrive **before** the
request's response.

### Errors

| Code | Meaning |
| --- | --- |
| `-32700` | The message is not JSON |
| `-32600` | Not a JSON-RPC request (also: a batch, or `session.hello` sent twice) |
| `-32601` | Unknown method |
| `-32602` | Invalid parameters (the message says which) |
| `-32603` | Internal error |
| `-32000` | Not authenticated: call `session.hello` first, or the user has not approved the client yet |
| `-32001` | Forbidden: the method needs the `control` permission |
| `-32002` | Access denied: wrong token, unknown or revoked key, or the user denied the client |
| `-32010` | The studio could not do it. `error.data.code` says why (see below) |

`-32010` reasons (`error.data.code`): `noModel` (no avatar on air), `unknownMotion`, `unknownExpression`,
`unknownScene`, `missing` (a file of the scene was not found), `unreadable`, `wrongType`, `tooLarge`, `dressUp`
(the module does not fit the slot). Other fields in `data` give details (`name`, `path`, `message`).

`error.message` is English text meant for logs; show your own text to your users based on `code` and `data.code`.

### Close codes

| Code | Meaning |
| --- | --- |
| `1000` | Normal close (the client closed) |
| `1001` | The user disconnected the client or turned the API off |
| `1002` / `1007` / `1009` | Protocol error / text that is not UTF-8 / message larger than 256 KiB (or a frame larger than 64 KiB) |
| `1003` | Binary messages are not supported |
| `1013` | Too many connections (32) |
| `4001` | The user revoked the client's key |
| `4003` | Access denied (wrong token or key, or the user denied the client) |

The studio sends a WebSocket ping when it has sent nothing for 20 seconds, and closes connections that send nothing
(not even a pong) for 60 seconds. Standard WebSocket clients answer pings automatically.

## Methods

Parameters and results are listed by name; the JSON Schema has the exact types.

### `session.hello` — introduce the client

| Params | |
| --- | --- |
| `name` | string, 1–64 characters, shown to the user |
| `token` | string, the studio's token (first connection) |
| `key` | string, the key from an earlier approval |
| `permission` | `"read"` (default) or `"control"`, the permission you ask for |

Result: `studioVersion`, `apiVersion` (`1`), `client` (client ID), `permission`, `events` (event names the studio
can send), and `key` right after approval.

### `studio.state` — what is on air (read)

Result: `scene` (`{id, name, model}` or `null`), `model` (display name or `null`), `modules` (worn modules,
`{id, name, slot}`), `expressions` (active expressions, in the order they were applied), `motion`
(`{name, priority}` or `null`), `tracking` (`{tracking, trackingPaused, lipSyncPaused}`).

### `scene.list` (read) / `scene.activate` (control)

`scene.list` returns `scenes` (`[{id, name, model}]`, in the control window's order; `model` is a file name, never a
path) and `live` (the scene on air, or `null`).

`scene.activate` takes `{scene}` (a scene ID) and switches to it. Scenes with the same model keep parameter values,
tracking and playing motions; a different model is loaded fresh. If the scene cannot be shown (missing file), the
previous scene stays on air and the call fails with `-32010`.

### Expressions

| Method | Params | Effect |
| --- | --- | --- |
| `expression.list` (read) | — | `{expressions, active}`: all expression names of the model, and the active ones |
| `expression.set` | `{expression}` | Use only this expression |
| `expression.add` | `{expression}` | Layer this expression on the active ones |
| `expression.remove` | `{expression}` | Fade this expression out |
| `expression.toggle` | `{expression}` | Add it if inactive, remove it if active (like a hotkey) |
| `expression.clear` | — | Fade all expressions out |

### Motions

| Method | Params | Effect |
| --- | --- | --- |
| `motion.list` (read) | — | `{motions: [{name, duration, looping}], current, idle}` |
| `motion.play` | `{motion, priority?}` | Start a motion. `priority` is `idle`, `normal` (default) or `force`. Result `{started}` is `false` when a motion with a higher priority is playing |
| `motion.stop` | — | Fade out the playing motion and return to the idle motion |

### Parameters

`parameter.list` (read) returns `parameters`: `[{id, name, min, max, default, value, internal}]` in the model's
order. `value` is the value sent in the latest frame. `internal` marks relay parameters that UIs usually hide.

`parameter.set` (control) overrides parameter values for a limited time:

```json
{"jsonrpc":"2.0","id":7,"method":"parameter.set",
 "params":{"values":[{"id":"AngleX","value":20},{"id":"MouthOpen","value":1,"weight":0.5}],"duration":1}}
```

- `values`: up to 256 `{id, value, weight?}`. `value` is clamped to the parameter's range. `weight` (0–1, default 1)
  blends from the studio's own value (`0`) to `value` (`1`).
- `duration`: seconds, more than 0 and at most 10, default **1**. When it runs out, the override ends and the
  parameter returns to what tracking and motions say. To hold a value, send it again before it expires
  (for example ten times a second for smooth control). Sending a parameter again replaces its override.
- The override is applied after motions, tracking, expressions, eye blinking and breathing, so it wins over them.
  Physics then follows the overridden values. Lip sync from the microphone is still applied on top of the mouth
  parameters.
- Result: `{unknown}`, the IDs the model does not have (they are ignored).

### Dress-up

`dressup.list` (read) returns `slots` (`[{slot, module}]`, the model's slots and the module ID worn in each, or
`null`) and `modules` (`[{id, name, slot}]`, the modules you may attach).

For safety the API can only attach modules the user already uses: modules of any scene in the current profile and
modules assigned to dress-up hotkeys. Module IDs are opaque strings (currently the file path); pass them back as
they are.

`dressup.set` (control) takes `{slot, module}` and puts `module` (an ID from `dressup.list`) in `slot` of the scene on
air, or empties the slot when `module` is `null`. The change is saved in the scene, like a dress-up hotkey.

### Hotkey actions

These do what the studio's hotkeys of the same kind do, so tools can replace global hotkeys (for example on Wayland,
where global hotkeys only work while the studio is in front).

| Method | Params | Effect |
| --- | --- | --- |
| `tracking.pause` | `{paused?}` | Pause or resume face tracking (omit `paused` to toggle). Result: `{tracking, trackingPaused, lipSyncPaused}` |
| `lipsync.pause` | `{paused?}` | Pause or resume microphone lip sync (omit to toggle). Same result |
| `tracking.calibrate` | — | Make the current face pose neutral (done by the control window's tracker) |

Scene switching, expressions, motions and dress-up have their own methods above.

### `events.subscribe` / `events.unsubscribe` (read)

Take `{events: [names]}` and return `{events}`, the events this connection now receives. A new connection receives
no events. Unknown names fail with `-32602`.

## Events

Notifications have the form `{"jsonrpc":"2.0","method":"<event>","params":{…}}`.

| Event | Params | When |
| --- | --- | --- |
| `scene.changed` | `{scene}` (`{id, name, model}` or `null`) | Another scene went on air |
| `scene.listChanged` | `{scenes}` (as in `scene.list`) | Scenes were added, removed, renamed, reordered or got another model |
| `expression.changed` | `{active}` | The active expressions changed |
| `motion.started` | `{motion, priority}` | A motion started (including the idle motion) |
| `motion.finished` | `{motion}` | A motion ended or was replaced |
| `motion.event` | `{motion, value, time}` | A motion reached one of its events (`value` as authored, `time` in seconds) |
| `dressup.changed` | `{modules}` (`[{id, name, slot}]`) | The worn modules of the scene on air changed |
| `tracking.changed` | `{tracking, trackingPaused, lipSyncPaused}` | Face tracking started or stopped, or tracking / lip sync was paused or resumed |

## Example session

```text
→ {"jsonrpc":"2.0","id":1,"method":"session.hello","params":{"name":"Deck","key":"<key>"}}
← {"jsonrpc":"2.0","id":1,"result":{"studioVersion":"1.0.0","apiVersion":1,"client":"5f0c…","permission":"control","events":[…]}}
→ {"jsonrpc":"2.0","id":2,"method":"events.subscribe","params":{"events":["scene.changed"]}}
← {"jsonrpc":"2.0","id":2,"result":{"events":["scene.changed"]}}
→ {"jsonrpc":"2.0","id":3,"method":"scene.activate","params":{"scene":"scene-2"}}
← {"jsonrpc":"2.0","method":"scene.changed","params":{"scene":{"id":"scene-2","name":"Game","model":"avatar.ugoe"}}}
← {"jsonrpc":"2.0","id":3,"result":{}}
→ {"jsonrpc":"2.0","method":"expression.set","params":{"expression":"Smile"}}
```

## Versioning

`apiVersion` is `1`. New methods, new optional parameters, new result fields and new events may be added without
changing it; clients should ignore fields and events they do not know. Incompatible changes raise `apiVersion`.
