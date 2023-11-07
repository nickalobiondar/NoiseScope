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

