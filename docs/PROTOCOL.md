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
