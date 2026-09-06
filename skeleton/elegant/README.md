# E2 typed composition experiment

This standalone, zero-dependency Rust crate tests [the architecture](../../docs/architecture.md). It is an executable
model outside the production workspace, not a second supported API.

```sh
cargo test --manifest-path skeleton/elegant/Cargo.toml
cargo clippy --manifest-path skeleton/elegant/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path skeleton/elegant/Cargo.toml -- --check
```

[Composition consumers](tests/composition.rs) build two counters and a picker with typed children,
commands, and locally mapped outputs. They exercise delivery, removal, foreign handles, bounded
admission, frame coalescing, and independent timers. Activation is a semantic command; there is no
native mouse routing. A compile-fail doctest checks command typing.

The queued picker regression preserves two query/selection pairs while yielding after every
envelope. Programmatic search updates are silent; child edits emit query changes. This avoids
an asynchronous echo that let selection overtake filtering. Three additional compile-fail checks
reject type coercions that would violate the runtime's private payload invariant. The bounded
retry example keeps one pending output when admission fails and reaches idle after delivering it.

[Layout checks](tests/layout.rs) exercise intrinsic decoration, baseline alignment, width-dependent
scroll extents, open custom measurement, shared appearance, and local overrides. The paragraph is a
deterministic wrapping stand-in. Layout is not connected to the composition runtime yet.

One mailbox owns queue capacity. Type erasure and non-reused identities stay private. Callback
effects apply after the widget borrow ends. Failed sends return the payload; removal can cancel
successful sends. Widget-local state does not roll back. Test assertions about sufficient queue
capacity are not an overload policy for real components.

The model omits C7's input/lifecycle router, dynamic insertion, tasks, shaping, rendering, accessibility,
and mount/visibility notifications. Timer and visibility scans are naive. Queue capacity bounds
envelopes, not bytes, tree size, or scheduler storage. Layout assumes valid child metrics and is
not a full constraint solver. These limits are not production resource guarantees.
