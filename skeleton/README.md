# Architecture experiments

These standalone crates preserve behavioral counterexamples for the replacement framework.
They are not supported framework APIs or native performance proofs. The chosen design is in
[the architecture](../docs/architecture.md); project policy is in [AGENTS.md](../AGENTS.md).

```sh
cargo test --manifest-path skeleton/Cargo.toml
cargo test --manifest-path skeleton/elegant/Cargo.toml
```

The C7 model has 43 tests for ownership, retirement, mount eligibility, focus/capture, modal
restoration, shared transforms, anchor publication, bounded delivery, stale task results,
virtual rows, text revisions, and host scheduling. Its maps and scans prioritize inspectable
behavior. Its public routing API and multiple frame slots are superseded.

The [E2 model](elegant/README.md) has 15 integration tests and four compile-fail checks for typed
composition, queue overload, removal, frame coalescing, independent timers, layout and appearance.
Independent review found and resolved invariant-type and queued-picker-ordering failures. Their
regressions remain in code. FIFO envelopes do not make a chain of descendant effects atomic.

Neither model integrates native input, shaping, GPU rendering, IME, or accessibility. E2 layout
is not connected to composition yet. Re-express useful tests through the replacement framework
as these features are implemented. Delete superseded models once their unique coverage is ported.

## Behavioral reference

The following records C7 ordering and failure behavior for implementation work. The architecture
supersedes public API mechanics, particularly receipt-based static composition and frame slots.

## Event turn

1. Host translates OS input into Input, retaining pointer/button or physical/logical key identity.
2. Consecutive move samples may replace older samples for the same pointer. Flush before any
   non-move input, geometry change, or frame. Raw-sample consumers may disable replacement.
3. Runtime commits pending structural edits and resolves dirty geometry before hit testing.
4. Modal scope filters candidates before hover changes. Capture takes priority within scope.
5. Route preview root-to-target, then target, then unhandled bubble target-parent-to-root.
   StopPropagation stops later phases. Frame, lifecycle and text-session events target only.
6. After each callback, validate/commit its queued requests. Keep independent wake updates.
   Event route keys are snapshotted; removed keys are skipped, inserted keys wait for new input.
7. Child actions enter the queue at their logical parent. An owner consumes, forwards unchanged,
   or emits a replacement action with its own source. Unconsumed root actions reach the app.
8. Drain at most the configured delivery budget before giving input/presentation another turn.
   Remaining work requests an immediate OS wake; it is never silently discarded as "idle".
9. Resolve layout and publish geometry; paint only if dirty or OS requests exposure repair.

(D/I3,I6) Request batches use a sequence of tagged requests, not one mutable slot per event.
Within one owner/slot the latest request wins; different owners and slots accumulate.
Each update changes a slot epoch. Revalidate the epoch immediately before a callback,
including work already selected as due; an earlier callback may cancel or replace it.
Monotonic deadlines and frame requests are distinct. NextFrame fires once at the next allowed
presentation opportunity with current time and elapsed time; a callback must request another.
Deadline callbacks do not force repaint. Hidden visible-only work is cancelled; visibility
notifications permit rescheduling. Explicit mounted-lifetime timers may continue while hidden.

## Structural and state changes

Insert(parent, local_token, widget), Remove(key), Reorder(parent, child_keys), Place(key, placement).
Only an owner can edit its direct children; the app can edit roots. A callback never receives
an unrestricted mutable arena. Each structural command validates before mutation and is
atomic individually. There is no implicit transaction over all work emitted by a callback.
Insert returns an owner-targeted Committed(token, Result<Key, EditError>) after child creation;
this both configures new children immediately and exposes duplicate-token/anchor failures.
Rejected target commands report failure to their owner; successful inserts earlier in the
same callback remain mounted. Callers needing all-or-nothing grouping can create a detached
composition specification and insert it as one validated subtree in a later API extension.
Property updates use target messages or an update guard that conservatively invalidates layout;
widget code can request narrower Paint or Semantics invalidation for its own state changes.
Target messages and task results never bubble into application actions. An unhandled target
payload returns Unhandled. Upward actions reserve a delivery slot before calling an owner;
new emissions use only remaining capacity and return their payload on Full. Capacity remains
reserved across callback completion and nested lifecycle notification. A consumed action's
reserved slot is released after its callback; no speculative slot reuse is required initially.

Dirty layout marks ancestors and affected descendants. `layout_child` reuses a child's result
only when constraints, widget content revision, and text/font revision match. Changing placement
rebuilds world geometry, old/new damage, and semantics bounds without forcing text measurement.
There is no unconditional layout of every widget on every event.

## Geometry / paint

Layout provides local bounds, local-to-placement-parent affine transform, clip, and hit policy.
Geometry computes inverse world matrices and clip ancestry once per changed branch. Hit testing
uses inverse world coordinates and the same clips. Overlay placement uses viewport clipping.
Paint traversal isolates widget state. A widget's internal drawing transforms do not leak to
children; transformed children use placement metadata. Paint bounds declare effects outside
layout bounds. Unknown/unbounded custom effects force full damage in that surface.

Painter supports paths, fills, strokes, image leases, immutable text layouts, and save/clip state.
Custom backend drawing returns Supported or Unsupported. The host isolates its render target,
transform and clip state and restores them after the hook. The portable fallback remains valid.
Partial repaint is only legal when the host proves pixel preservation and repairs all damage.

## Text and semantics

Document owns text/revision and accepts replace(range, text) -> Change or InvalidRange.
TextService shapes a paragraph request into a versioned immutable LayoutLease containing
glyph runs, visual caret stops with affinity, line boxes, and selection geometry. Painter draws
that lease; it does not independently reshape its source string. Layout caches are byte bounded.
An editor stores document positions, selection and IME preedit separately. A replacement edit
advances revision once; undo is a document operation. IME events carry the receiving host's
focus-session token. Queued preedit/commit for an obsolete token is discarded. The native host
must reset its IME session on focus changes; actual OS behavior is a native acceptance gate.
Semantics declares role/name/value/actions and optional virtual item count. Bounds and focus
come from runtime geometry. Actions include a widget key and snapshot revision; stale actions
return Stale. A virtual item's Show action realizes its row before requesting focus.

## Host embedding and bounds

Core exposes next_work() -> {ready, earliest_deadline, frame_requested} together. Host chooses
the earlier of a timer deadline and its next allowed frame opportunity; ready work wakes now.
An empty summary means Idle. Zero-size/occluded surfaces stop visible frame delivery and retain damage.
Foreground resumption publishes visibility and requests full exposure repair.
Mailbox capacity and per-turn work budgets are host configuration, with finite defaults.
The event/action budget yields fairly; mailbox Full is explicit backpressure. No model-specific
worker pool, automatic async runtime, reconciler, or unbounded display-list cache is required.

Pointer-up bookkeeping runs even when a handler stops propagation; default capture ends
when the pointer's last pressed button releases. Explicit release/cancel may end it earlier.
Ordinary focus changes cancel key/composition state, preserving unrelated pointer captures.
Effective visibility changes notify all affected descendants and anchor-dependent overlays,
so visible-only animation can restart when shown. Placement/anchor dependencies must be acyclic.

## Lifecycle ordering (C3)

Allocated keys are available to their owner immediately, but input/paint eligibility begins
only when mount runs. Hosts may yield a delivery budget with unmounted children pending.
Focus/capture state commits synchronously after an ordinary input callback. Notifications
finish that transition before any commands they emit can run. Those commands enter the same
bounded queue and consume the same delivery budget; capacity reservations include downstream
insert receipts. Repeated focus reclamation therefore cannot recurse or monopolize a turn.
Blur blocks reacquisition until the host activates the window again.
CaptureLost(pointer) targets the previous capturer on transfer, release, or invalidation.
CancelKeys clears key/composition state on focus loss without changing other pointers' buttons.
Modal scopes form an ownership-nested stack. Closing a scope restores its saved eligible focus,
or the enclosing scope default. Removed/hidden scopes are discarded and saved keys revalidated.
Geometry publication compares effective visibility, including singular transforms and anchor
ancestors; disappearing anchors notify the overlay owner, and recovery permits animation restart.

The model accepts a host-supplied unique Ui identity; the production constructor must allocate
one that is never reused during the process. Test arena scans and recursive geometry evaluation
are deliberately simple and do not establish the target traversal costs or memory footprint.

Allocated-but-unmounted nodes retain pending mounted-lifetime deadlines, but receive no timer
or frame callbacks until mount has run. Removing one simply drops its state; Unmount pairs
only with an earlier Mount. Insert receipts report committed allocation, not readiness of
an arbitrarily deep child subtree. An enclosing modal supplies focus fallback on every scope
exit, including hide/removal, when the saved key is no longer eligible.

A keyboard route also snapshots its focus-session token. A focus/session change during any
phase invalidates the remaining route, even without StopPropagation. The old focus has already
received CancelKeys and must not receive a subsequent stale press whose release goes elsewhere.
Pointer routes retain their separate capture and phase rules.

Anchor availability is independent of whether its popup is currently visible. Existing owners
receive AnchorUnavailable when an anchor becomes unavailable or is removed, including hidden
popups. Mount publication updates already-mounted dependents so a pending anchor's arrival can
restore their visibility. A newly mounted widget uses Mount for initialization and does not
receive an extra initial Hidden(false).

Visibility/anchor publication compares against the last state published for each node, and
records the new state before invoking callbacks. Nested edits can publish subsequent changes
but cannot replay a transition already delivered by an inner edit. Initialization records a
new widget's starting state without synthesizing an initial visibility event.
