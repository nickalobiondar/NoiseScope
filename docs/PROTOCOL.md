# The noisescope protocol & transcript schema

This document defines the two declarative document kinds that `noisescope`
consumes — the **protocol specification** and the **transcript** — together with
the exact semantics of the four structural invariants the engine enforces.

> **Scope note.** `noisescope` is a *structural* handshake analyzer. It reasons
> about message order, roles, and metadata counters. It does **not** model
> cryptography of any kind and makes **no security claim**. See
> [Limitations](#limitations).

---

## 1. Document envelope

Both documents are UTF-8 JSON. As a convenience, `//` line comments are accepted
by the parser (a non-standard extension used only in fixtures). An optional
`"kind"` field (`"protocol"` or `"transcript"`) may be present for readability;
it is not required, since the two kinds are distinguished structurally.

---

## 2. Protocol specification

A protocol specification is a finite state machine plus per-transition metadata
requirements.

### 2.1 Fields

| Field          | Type              | Required | Meaning                                             |
|----------------|-------------------|----------|-----------------------------------------------------|
| `name`         | string            | yes      | Unique spec identifier.                             |
| `description`  | string            | no       | Human notes (surfaced in reports).                  |
| `roles`        | string[]          | yes      | Declared participant roles.                         |
| `initial`      | string            | yes      | Starting state; must appear in `states`.            |
| `states`       | string[]          | yes      | All state names.                                    |
| `accepting`    | string[]          | yes      | Terminal/accepting states; each must be in `states`.|
| `transitions`  | Transition[]      | yes      | The edges of the machine (see below).               |

### 2.2 Transition

| Field                  | Type    | Required | Default | Meaning                                                       |
|------------------------|---------|----------|---------|---------------------------------------------------------------|
| `from`                 | string  | yes      | —       | Source state.                                                 |
| `to`                   | string  | yes      | —       | Target state.                                                 |
| `role`                 | string  | yes      | —       | Role expected to send this message.                           |
| `msg`                  | string  | yes      | —       | Message-type label triggering the transition.                 |
| `requires_fresh_nonce` | boolean | no       | `false` | Event must carry a nonce **not seen before** in this run.     |
| `requires_seq`         | boolean | no       | `false` | Event must carry `seq == previous_seq_for_role + 1` (from 0). |
| `note`                 | string  | no       | —       | Human note.                                                   |

### 2.3 Example

```json
{
  "kind": "protocol",
  "name": "noise-XX-abstract",
  "roles": ["initiator", "responder"],
  "initial": "await_e",
  "states": ["await_e", "await_ee", "await_se", "established"],
  "accepting": ["established"],
  "transitions": [
    {"from": "await_e",  "to": "await_ee", "role": "initiator", "msg": "e",         "requires_fresh_nonce": true},
    {"from": "await_ee", "to": "await_se", "role": "responder", "msg": "e_ee_s_es", "requires_fresh_nonce": true, "requires_seq": true},
    {"from": "await_se", "to": "established", "role": "initiator", "msg": "s_se",    "requires_seq": true}
  ]
}
```

### 2.4 Spec linting

`noisescope lint spec.json` reports internal inconsistencies **without** any
transcript: an undeclared `initial`/`accepting` state, transitions referencing
unknown states, or transitions using undeclared roles. A clean spec exits `0`.

---

## 3. Transcript

A transcript is an ordered list of observed events.

### 3.1 Fields

| Field      | Type                | Required | Meaning                                        |
|------------|---------------------|----------|------------------------------------------------|
| `protocol` | string              | yes      | Name of the spec this transcript targets.      |
| `events`   | Event[]             | yes      | Ordered events.                                |

### 3.2 Event

| Field   | Type                  | Required | Meaning                                             |
|---------|-----------------------|----------|-----------------------------------------------------|
| `id`    | string                | no       | Stable id (defaults to `e{index}`).                 |
| `role`  | string                | yes      | Sender role.                                        |
| `msg`   | string                | yes      | Message-type label.                                 |
| `nonce` | number (uint)         | no       | Freshness token (plain integer, **not** a key).     |
| `seq`   | number (uint)         | no       | Per-role sequence counter.                          |
| `meta`  | object<string,scalar> | no       | Free-form metadata; scalars are coerced to strings. |

### 3.3 Example

```json
{
  "kind": "transcript",
  "protocol": "noise-XX-abstract",
  "events": [
    {"id": "m1", "role": "initiator", "msg": "e",         "nonce": 1001},
    {"id": "m2", "role": "responder", "msg": "e_ee_s_es", "nonce": 2002, "seq": 0},
    {"id": "m3", "role": "initiator", "msg": "s_se",      "seq": 0}
  ]
}
```

---

## 4. Replay semantics & invariants

The engine walks events in order from `initial`. For each event it looks up
transitions matching `(current_state, role, msg)`:

1. **Order** — if **no** transition matches the `msg` from the current state,
   the event is an `unexpected_message`.
2. **Role** — if a transition exists for that `msg` from the current state but
   only for a *different* role, it is a `role_mismatch`.
3. **Nonce** — when the chosen transition has `requires_fresh_nonce`, the event
   must carry a `nonce` that has not been observed before in this run;
   otherwise `nonce_replay` (missing nonce is also a `nonce_replay`).
4. **Sequence** — when the chosen transition has `requires_seq`, the event's
   `seq` must equal the previous sequence value for **that role** plus one, with
   the first sequenced event for a role expected to be `0`; otherwise
   `sequence_violation`.

