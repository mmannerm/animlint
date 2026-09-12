# Collection contract extensions

This document records versioned extensions to the file-scoped collection
decision in [DESIGN.md Appendix F](../DESIGN.md#appendix-f--decision-record-file-scoped-clip-identity-and-collections).
Contact-fragment V1 includes its strict standalone and manifest-selected
producer/CLI/publication boundary. The directional-speed policy/evaluation V1
is a separate ordered 0.6.0 slice with one JSON-only evaluator command.
The contact-transform result and foot-cycle map planner are format-neutral
contracts; animation rewriting, collection publication, and runtime systems
remain separate work.

## Directional-speed policy V1 (#552, ordered slice)

The strict `collection-directional-speed-policy:1` reader is an ordered,
manifest-bound declaration for the directional-speed evaluator. It is a
separate TOML envelope: it does not add fields to collection-manifest V1,
revise collection-output V2, or infer membership from filenames or paths.
`animsmith collection evaluate-directional-speed --policy POLICY.toml --evidence
COLLECTION-OUTPUT.json --format json` binds its raw bounded inputs to the
typed reader, strict current collection-output V3 adapter, pure evaluator, and
immutable result. There is no output file, text/Markdown, subset, or inference
mode; publication remains a consumer responsibility.

The envelope repeats the exact manifest identity (`collection_id` plus
`{sha256, bytes}`), one directional-blend `runtime_set_id`, and every existing
logical member exactly once in manifest order. Each member has one nonzero,
unique semantic `[x, z]` coordinate. `source_basis.x` and `source_basis.z`
are orientation witnesses for raw collection-output V3 +X/+Z endpoint
displacement in that semantic plane; their magnitudes are nonsemantic, and the
evaluator uses unit axes for heading. They must be finite, bounded,
nonzero, and perpendicular. `diagonal_behavior` is closed and applies to
unit-input/base targets: for coordinate `c`, `preserve` uses gain
`g(c) = hypot(c)` while `normalize` uses `g(c) = 1`. Thus uniform expected
speed is `base * g(c)`, authored expected speed is the member's base
`speed_mps * g(c)`, and ratios compare
`expected_measured_ratio_i_to_ref = declared_expected_ratio_i * g(c_i) /
g(c_ref)`, with the reference declaration fixed at `1.0`. Direction comparison
is unaffected by this gain.

The declaration also carries finite bounded `direction_tolerance_deg` in the
inclusive range `0..=180` degrees. The pure evaluator normalizes raw endpoint
displacement before source-basis projection and uses published `speed_mps` for
magnitude (not travel). Its immutable result binds raw policy and evidence
`InputIdentity` values and retains a manifest-ordered row for every member:
policy coordinate, raw evidence, projected heading, comparison values,
tolerances, deviations, and pass/violation outcome. Incomplete root travel,
zero endpoint, zero ratio reference, and numeric range are typed
not-evaluated outcomes, never implicit passes, failures, or subsets.

`uniform` requires `uniform_speed_mps` and `speed_tolerance_mps`; `authored`
requires each member's `speed_mps` plus `speed_tolerance_mps`; and `ratios`
requires a declared `reference_member`, each member's `expected_ratio`, and a
dimensionless `ratio_tolerance`. Mode-inapplicable fields are rejected. The
strict reader rejects parse/declaration failures including unknown or
duplicate fields, unknown tokens, malformed identities, duplicate member
ids or coordinates, invalid coordinates or basis, nonfinite/out-of-range
values, and N+1 members before retaining them. Separate binding validation
rejects stale manifest identity, wrong runtime-set id or kind, and missing,
extra, or reordered member sequences. No licensed asset data belongs in this
declaration or its tests.

## Foot-cycle parameterization V1 (#18, planner slice)

`urn:animsmith:schema:foot-cycle-parameterization:1` is a strict bounded TOML
declaration for one existing collection-manifest `gait-group`. It repeats the
exact manifest identity, runtime-set id, exact ordered members, one explicit
reference member, one safe contact-fragment path per member, finite positive
minimum/maximum segment slopes whose inclusive interval contains `1.0`, and one
safe future output-directory path.
The reference owns canonical contact-boundary phases. Membership, ordering,
contact paths, and reference ownership are never inferred from filenames or
clip names.

```toml
schema = "urn:animsmith:schema:foot-cycle-parameterization:1"
schema_version = 1
runtime_set_id = "com.example/sets/walk"
reference_member = "com.example/walk-forward"
output_directory = "generated/walk-aligned"
minimum_segment_slope = 0.5
maximum_segment_slope = 2.0

[proof]
max_gait_phase_spread = 0.08
min_lr_amplitude_m = 0.05
max_contact_boundary_phase_error = 0.01

[manifest]
schema = "urn:animsmith:schema:collection-manifest:1"
schema_version = 1
collection_id = "com.example"

[manifest.input]
sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
bytes = 1234

[[members]]
id = "com.example/walk-forward"
contact_fragment = "contacts/walk-forward.json"

[[members]]
id = "com.example/walk-right"
contact_fragment = "contacts/walk-right.json"
```

The reader is closed and bounded at 8 MiB and 4,096 members. Planning admits at
most 16,384 contact events and 32 MiB of canonical contact-fragment bytes
across the whole declared ring, so independent per-fragment limits cannot
multiply into unbounded retained topology or canonicalization work. Exact
supplied byte counts are preflighted before canonicalization; a false count is
then caught by exact canonical byte-count and digest comparison on the first
such row.
Safe locators use the same lexical relative-path contract as collection
manifests. It rejects unknown fields, unsupported identities/versions, unsafe
or duplicate paths, duplicate/missing members, an absent reference, invalid
manifest identities, and invalid slope ranges. The required closed `[proof]`
table declares finite `max_gait_phase_spread` and
`max_contact_boundary_phase_error` values in the inclusive range `[0, 0.5]`
plus a finite non-negative `min_lr_amplitude_m`. These exact values are bound
by the parameterization byte identity and retained in the plan; V1 has no
defaults, gait-group/config lookup, filename inference, or cross-member policy
merge. The published
[`foot-cycle-parameterization-v1.schema.json`](schemas/foot-cycle-parameterization-v1.schema.json)
describes the decoded TOML shape. Host canonicalization,
symlink/alias detection, and the guarantee that `output_directory` is a
previously absent sibling generation destination are enforced by the private
source-preparation adapter described below; this parser performs no filesystem
I/O.

The pure core planner additionally requires each supplied sidecar byte identity
to equal its canonical `contact-fragment:1` identity and its collection clip
witness to equal the manifest's exact logical/source/take-index/take-name row.
Each member also supplies independently measured typed Root/Hips evidence bound
to the fragment's exact artifact, dependency closure, and collection clip
source/take witness. Cross-wired or stale measurements refuse before their
values are used. A
missing, ambiguous, malformed, or non-finite witness refuses. Evidence retains
signed endpoint X/Z displacement and signed accumulated unwrapped yaw. The
unchanged admission derives horizontal displacement with binary64 `hypot(X, Z)`
and uses absolute yaw; values above 0.01 m or 1 degree refuse, and both
thresholds are inclusive. The planner consumes and retains those facts but does
not sample an animation asset itself.
V1 recognizes exactly AnimSmith's `contact-support-detector:1` extension and
validates its closed algorithm, sampling, frame-cap, threshold, and selected
left/right foot-or-toe role provenance. Every member must carry the exact same
finite non-negative `contact_height_m` binary64 value because that policy
determines the compared window boundaries. Other extensions refuse because no
operation-specific transform handler has run.

The private CLI-crate preparation adapter now bounded-reads the exact manifest
and parameterization, resolves the exact member-reachable source/config subset
in canonical manifest source-key order plus every declared contact fragment and
absent output directory under its declaration root, and rejects symlinks,
canonical collisions, and (where the platform exposes file identity) hardlink
aliases before source parsing. It first validates the pure exact runtime-set
binding, then loads only its member-reachable manifest sources/configs in
manifest order; unrelated source rows do not gate this scoped operation. It
captures each distinct canonical config once and shares that immutable
normalized snapshot and explicit byte identity across its sources for later
proof without a control-file reread. Distinct retained config snapshots share
an inclusive 32 MiB invocation ceiling; repeated use of the same canonical
config is counted and captured once. It requires complete
dependency-closure identity and exact raw take index/name witnesses,
counts primary, every retained external dependency identity, and normalized
document semantic payload against per-source and invocation byte ceilings. Each
loaded document passes strict structural shape validation before sampling or
candidate work. The adapter then preflights both `frames * bones` pose cells
and `frames * tracks` sampling work for every selected member and admits the
complete multi-member totals against the inclusive one-million-cell/work
ceiling before constructing the first metric grid. The prepared in-memory
collection retains those exact source pose-cell and sample-evaluation totals so
a later output proof can share one invocation budget without resampling or
rereading controls. After planning, it runs the
core clip-candidate preflight for every member and checks invocation-wide
candidate-key, retained-value, exact candidate-storage-byte (including name),
and work totals before constructing the first candidate.
It then
derives Root/Hips endpoint X/Z `hypot` and absolute unwrapped yaw through the
existing metric grid/role/trajectory authority, and cross-checks every returned
plan binding and duration before calling `time_warp_clip_v1`. Any refusal drops
the whole in-memory batch and creates no output or partial result.

Preparation deliberately stops contact transformation at a typed continuation.
It validates and reconstructs the closed stance-detector payload through the
operation-specific core handler, but a fresh output artifact identity and its
dependency closure do not exist until the candidate document is serialized.
Only that later serialization step may supply those exact identities and call
`transform_contact_fragment_v1`; preparation does not hash an in-memory `Clip`,
claim a generated artifact identity, serialize, reread/prove, or publish. No
user-visible command is added by this slice. A freshly serialized exact-copy
candidate may truthfully retain the input content identity; freshness comes
from capturing the serialization result, not from requiring unequal digests.

Each member must contain complete positive left/right support windows, each
with exactly one same-side marker. Fragment rows stay physically linear:
windows are partitioned by side, and same-side runs must be strictly disjoint
(even touching same-side runs refuse). When one side has both a first run
starting at `0` and a distinct last run ending at `1`, those two physical rows
form one logical cyclic stance. Their artificial seam edges are omitted from
topology correspondence, while both windows and both markers remain in the
transformed fragment. A single full-cycle `[0,1]` stance refuses. Opposite-side
windows may overlap or touch. After seam coalescing, both sides must have the
same nonzero logical-run count and their onsets must alternate around the
cycle.

The planner derives true logical onsets/releases in deterministic cyclic
chronological order, including a fixed kind order for simultaneous phases. It
rotates only the correspondence sequence to a true left-support onset; it does
not rotate numeric clip phases. Therefore the reference boundary phase remains
its authored normalized phase and a member that would require moving phase
zero, including incompatible wrap/non-wrap correspondence, refuses as
non-monotone.

Independent output proof first requires the expected and freshly detected
`(side, edge)` sequence in absolute phase-zero chronological order to match.
It then compares those phase-zero-paired boundary phases with circular numeric
distance, but a complete left/right role swap is a topology failure even at
the maximum allowed boundary-error tolerance.

For matching signatures, corresponding source onsets/releases map to the
reference phases. Simultaneous source boundaries are admitted only when all
corresponding reference boundaries are simultaneous at one phase, and vice
versa. Identical `(source, output)` pairs collapse to one map knot; a group
split on either axis refuses. `(0,0)` and `(1,1)` are then included in that same
deduplication, so the planner preflights the 4,096-point contact-transform cap
from the retained knots rather than raw boundary rows. Every resulting segment
must be finite, strictly increasing, continuous, and within the inclusive
declared slope range. The planner returns one duration-preserving
`ContactTransformOperationV1::TimeWarp` plus exact
`ContactTransformBindingV1` per member.

The separate pure `time_warp_clip_v1` seam consumes one such member plan only
after its caller has selected and bound the corresponding loaded clip. It
strictly validates track-local clip/track shape; because this seam has no
skeleton input, its host must already have bound every track bone index to the
selected, validated skeleton. Fixed aggregate name-byte, track,
authored/generated key/value, and work caps refuse before candidate allocation;
candidate storage has a derived public upper bound from those admitted rows.
LINEAR tracks map every authored key and insert interior map-knot
samples; STEP tracks map only authored breakpoints. A map knot and an authored
key that are the same instant in the emitted binary32 domain — equal, or one
representable place apart, since that domain holds nothing between them — are
one emitted key: it keeps the authored time and the authored value and carries
the control point's own output, because the control point is the definition of
the map at that instant. `FootCycleClipWarpKnotV1` is the one definition of that
question and `time_warp_rows_v1` the one answer to which keys a track emits and
in what order; the independent output proof consumes those rows for the
sequence alone and derives every expected time and value itself, a knot's own
times included.
That holds for a track's own first and last authored key too: the span test is
inclusive of one place on either side, so which side of a rounding step the
reconstructed instant lands on does not decide whether that key takes its
control point's output. Where two authored keys are one place apart, a knot
binds to the key it is bit-equal to rather than the key it is merely beside,
and both keys are retained. The one exception is a map knot that is the same
instant as a source endpoint, `0` or the duration: the map's exact endpoint
rows define the output there and a candidate retains those two instants as
themselves, so such a knot contributes nothing. Two knots that name one instant
refuse: each is compared against both instants of the knot before it, its own
resolved source time and the instant it named, because a knot that coalesced
into an authored key answers to a time one place from its own. Distinct output
instants that collide after binary32 narrowing refuse too. One-key CUBICSPLINE
tracks are retained, and multi-key cubic tracks are retained only for bit-exact
constant values with zero tangents. The value caps are the shape-derived
three-values-per-cubic-key maxima, so malformed N+1 storage refuses at shape or
key bounds before it can become a separate valid value-only case.

Neither pure seam transforms contact fragments, derives root measurements,
binds the selected asset to the plan, serializes output artifacts, proves
reread results, or publishes a generation directory; those remain later #18
transaction seams described below.

## Foot-cycle generation V1 (#18)

The public producer is deliberately one collection command with no destination
override:

```console
animsmith collection transform-foot-cycle COLLECTION.toml \
  --parameterization FOOT-CYCLE.toml --format json
```

The parameterization's `output_directory` is resolved relative to that
document and must name a previously absent directory below an existing safe
parent. The command is available in the glTF-only build; `.fbx` sources remain
available only when the `fbx` feature is enabled. Every admitted source,
including `.gltf`, is emitted as a self-contained GLB.

For `N` members, the generation has exactly `3N+1` files in parameterization
member order:

```text
members/000000/artifact.glb
members/000000/contact-fragment.json
members/000000/evidence.json
...
aggregate-evidence.json
```

Logical ids and source keys are evidence fields, never filesystem names. The
member evidence identity is
`urn:animsmith:schema:foot-cycle-member-evidence:1`; it binds exact manifest
and parameterization inputs, source artifact/config/complete closure, output
artifact/complete closure, transformed and independently redetected contact
identities, canonical operation identity, declared proof policy, and reread
proof residuals. The aggregate identity is
`urn:animsmith:schema:foot-cycle-aggregate-evidence:1`; it repeats the control
bindings and policy, preserves ordered member paths and identities, and records
checked `3N+1`, component bytes, exact total, retained GLB bytes, and
source/output/combined metric work. See
[`foot-cycle-member-evidence-v1.schema.json`](schemas/foot-cycle-member-evidence-v1.schema.json)
and
[`foot-cycle-aggregate-evidence-v1.schema.json`](schemas/foot-cycle-aggregate-evidence-v1.schema.json).

All GLBs, canonical transformed fragments, member evidence, and aggregate
evidence share one 256 MiB retained/staged cap in addition to their stricter
component limits. Every candidate, contact transform, independent reread
proof, and bounded serialization finishes before the first filesystem write.
The publisher flushes the fixed file set beneath one sibling temporary
directory and performs one no-replace directory rename. It does not replace an
existing generation or claim power-loss, network-filesystem, or abandoned-temp
cleanup guarantees.

Success exits 0 and writes the exact published `aggregate-evidence.json` bytes
to stdout only after publication. A broken stdout is diagnosed best effort and
cannot undo or relabel the generation. Asset/capability/map/contact/proof
refusal exits 1 with one `producer-refusal:1` JSON record and no destination.
Malformed or stale controls and bindings, unsafe paths or aliases, I/O,
serialization, or publication/race failures exit 2 with empty stdout and no
published destination.

## Contact fragments (#147)

The contact-fragment V1 identity is
`urn:animsmith:schema:contact-fragment:1`.
AnimSmith 0.6.0 ships #152's format-neutral strict core reader, bounded
canonicalization seam, and strict one-clip producer. `generate
contact-fragment` includes a manifest-selected collection form that reloads
the declared source rather than reading collection-output evidence.
Transforms and runtime systems remain out of scope.

It is an importable envelope that
can be merged into a host's one authoritative measured sidecar. It binds
contact facts to the exact primary source bytes and complete versioned
dependency closure, plus producer/tool version and an unambiguous clip
reference. `artifact` is the existing primary `InputIdentity`;
`dependency_closure_identity` is the existing complete
`DependencyClosureIdentityV1` rather than the full output-v10 closure record.
Both serialize as `{sha256, bytes}`: `sha256` makes the algorithm and digest
explicit rather than adding separate `algorithm` and `digest` fields. Partial
or unavailable dependency coverage refuses fragment generation, and a
mismatch in either identity makes an existing fragment stale.
The complete captured closure's `primary_input` must equal `artifact`; producer
and consumer validate that relationship against the captured closure.
V1 deliberately binds every dependency in the complete modeled closure,
including dependencies such as textures that may not affect contacts. This can
refuse generation for an unavailable unrelated dependency, but avoids a second
format-specific relevance policy.

Collection-owned clips use the [#409 logical identity](../DESIGN.md#f2-logical-and-physical-clip-identity)
plus its `source`, take-index, and exact take-name witness. Standalone
documents use an exact embedded clip/take name scoped by both input identities;
ambiguous or duplicate names are refused. No animation-array index, filename,
or engine asset handle is an identity.

Collection-manifest V1 rejects duplicate logical clip ids while parsing as a
collection control error, before `collection generate-contact-fragment` can
select a clip. A valid collection take-index addresses the raw source take row;
the producer resolves that row's complete normalized-index witness before
sampling its internal document clip.

Points and windows use normalized clip time `[0, 1]`, with the measured
positive duration recorded for validation. Events have stable opaque ids,
engine-neutral roles and phases, and deterministic ordering. V1's closed role
vocabulary is `left_foot`, `right_foot`, `left_hand`, `right_hand`, `left_toe`,
`right_toe`, `left_knee`, `right_knee`, `left_elbow`, `right_elbow`, `root`,
`prop`, and `body`; its phases are `begin`, `end`, and `marker`. Trim, slice, and
time-warp operations must transform the fragment with the same operation or
return structured per-event outcomes/refusal. Events outside the retained
interval are never silently clamped, dropped, or retained.

Confidence, when present, is finite and lies in `[0, 1]`. Core fields are
closed. Extensions use strict envelopes with exactly `schema`,
`schema_version`, and `payload`, where the extension's versioned schema owns
the payload object; an unsupported but well-formed extension may be preserved
as opaque data. Any transformer must understand and apply every
extension's operation contract or refuse the whole operation with
`unsupported_extension`; it may not copy opaque extension-owned times or event
references unchanged.

The strict V1 reader caps a contact-fragment source at 8,388,608 bytes and 32
JSON container levels, with at most 4,096 events and 256 extensions. Authored
strings are at most 4,096 UTF-8 bytes and identifiers at most 255 bytes; each
extension payload is at most 262,144 canonical bytes and 16 levels deep. A
complete fragment is at most 8,388,608 canonical bytes. A transform-result
source and complete canonical result are each at most 16,777,216 bytes, with at
most 4,096 control points and 4,096 event outcomes; its inline fragment obeys
the fragment limits. Depth counts every object/array on the root-to-value path,
including the root envelope as depth 1. A payload object also starts at depth 1
for its separate 16-level limit. Each exact maximum is accepted; N+1 is
rejected during bounded decoding before the excess value is retained or an
unbounded canonical buffer is allocated; canonical-byte limits use a bounded
JCS sink.
JSON Schema `maxLength` counts Unicode code points, while this reader's text
and identifier limits count UTF-8 bytes, so a non-ASCII value may satisfy the
schema and still be correctly refused by the reader. The reader accepts decoded
IEEE-754 numbers only within the safe magnitude
`[-9007199254740991, 9007199254740991]`; this includes identity byte counts
and opaque extension payload numbers. Extension payloads are stored from their
bounded canonical JCS bytes, so canonical serialization followed by strict
reread preserves the validated fragment value. An explicitly empty `extensions` array
is refused so omitted and present data do not canonicalize to the same value.

A minimal collection-scoped envelope is:

```json
{
  "schema": "urn:animsmith:schema:contact-fragment:1",
  "schema_version": 1,
  "producer": {"tool": "animsmith", "version": "0.6.0"},
  "artifact": {"sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", "bytes": 123456},
  "dependency_closure_identity": {"sha256": "1111111111111111111111111111111111111111111111111111111111111111", "bytes": 456},
  "clip": {"scope": "collection", "logical_id": "com.example.pack/locomotion/walk-forward-in-place", "source": "walk-forward", "take_index": 0, "take_name": "Take 001"},
  "duration_s": 1.2,
  "events": [{"event_id": "left-foot/0", "role": "left_foot", "phase": "marker", "time": 0.23, "confidence": 0.92}],
  "extensions": [{"schema": "urn:example:contact-quality:1", "schema_version": 1, "payload": {"quality": "high"}}]
}
```

Seconds and frame values, when shown, are derived display values rather than
identity or comparison coordinates. Canonical bytes use the complete
[RFC 8785 JSON Canonicalization Scheme (JCS)](https://www.rfc-editor.org/rfc/rfc8785),
including its object-key, string, and number rules; every extension payload
must also be JCS-canonicalizable. Before JCS serialization, events use the exact mixed-event sort tuple
`(start, kind_rank, end_key, role, phase, event_id)`: points have kind rank `0`
with `start = time` and a `null` end sentinel that sorts before every numeric
window end; windows use their declared `start`, have kind rank `1`, and use
their numeric end. This keeps point/window order stable when starts coincide.
Tuple strings, including opaque event ids, compare by
unsigned UTF-16 code units exactly like RFC 8785 property names, without
Unicode normalization.

Trim, slice, resample, and time-warp use the separate
`urn:animsmith:schema:contact-transform-result:1` result. It binds input and,
on success, output primary, dependency-closure, and canonical-fragment
identities. Its strict top-level fields are `schema`, `schema_version`,
`operation`, `input`, `outcome`, and `event_outcomes`, with `output` only on
success and `refusal { code, message }` only on refusal. `operation` is a
strict tagged object: trim/slice use `interval { start, end }`, resample uses
`mapping = "identity"`, and `time_warp` uses required finite positive
`output_duration_s` plus ordered `control_points` with `input_time`/`output_time`
from `(0,0)` to `(1,1)`. Between adjacent knots `(x0, y0)` and `(x1, y1)`, it
maps `t` by the exact piecewise-linear formula
`y0 + ((t - x0) / (x1 - x0)) * (y1 - y0)`; exact knots map exactly, and both
window endpoints use the same rule. A known V1 tag with an invalid numeric
domain or ordering remains representable for an `invalid_mapping` refusal and
uses an empty pre-inventory `event_outcomes` list. An unknown kind, version,
field, mapping token, missing field, or malformed field type is a strict
request/reader error and produces no V1 result or event outcomes. `input` has
exactly
`{artifact:{sha256,bytes},dependency_closure_identity:{sha256,bytes},fragment:{sha256,bytes}}`
and refers to a separately
supplied input fragment. Successful `output` has exactly
`{artifact:{sha256,bytes},dependency_closure_identity:{sha256,bytes},fragment:{sha256,bytes},contact_fragment}`;
all three identities must match the inline complete `contact-fragment:1` and
its two input bindings. The output closure identity is freshly captured rather
than copied, and its closure `primary_input` must equal output `artifact`. For
trim/slice `[a,b]`, a point is
outside exactly when `t < a || t > b`; endpoints are included. A window is
outside exactly when `end < a || start > b`, contained exactly when
`a <= start && end <= b`, and otherwise boundary-crossing. A crossing window
receives `refused` with code `partial_window` and refuses the whole operation.
Known-operation mapping failures, binding mismatches, unsupported extensions,
or partial windows produce a typed `refused` result with no `output` field.
After binding and identity validation, `event_outcomes` has exactly one
`{event_id, outcome}` object per input event in canonical input order. It adds
`value` only for transformed exact point/window values and `code` only for
refused events; pre-inventory binding, identity, mapping-validation, or
extension-support refusal uses an empty list.
Global success requires all events to be transformed or outside and requires
`output`; global refusal requires top-level `refusal` and omits `output`.
The inline output duration is `input_duration_s * (b - a)` for trim/slice,
the input duration for resample, and the required `output_duration_s` for
`time_warp`; the inline fragment and operation must agree exactly. A rounded
trim/slice duration that is not finite and positive refuses before event
inventory with `invalid_value` and an empty `event_outcomes` list.
All time/duration inputs and results are finite IEEE 754 binary64. The
normative sequence is `dx = rn(x1 - x0)`,
`alpha = rn(rn(t - x0) / dx)`, `dy = rn(y1 - y0)`, and
`mapped = rn(y0 + rn(alpha * dy))`; an exact knot bypasses interpolation.
Trim/slice uses `span = rn(b - a)`, `mapped = rn(rn(t - a) / span)`, and
`output_duration_s = rn(input_duration_s * span)`. Here `rn` is IEEE 754
binary64 round-to-nearest, ties-to-even, with no fused or extended-precision
intermediate, so independent producers feed the same numeric results to JCS.
Refusal codes are `partial_window`, `invalid_mapping`, `invalid_binding`,
`invalid_value`, and `unsupported_extension`. Malformed fragments/results and
duplicate, missing, or unknown event-outcome identities are strict reader
errors, not refusal results.

`animsmith-core` implements this mapping and strict result reader. The reader
requires the separately supplied input fragment plus an external transform
context containing the current input/output artifacts, their complete captured
closures, the expected producer, and any handler-produced extension outputs.
It validates each closure against its artifact and independently rederives the
operation, event outcomes, and successful inline fragment. Opaque extensions
are never copied merely because a caller lists their schema/version: an exact
handler-produced output is required at every input extension position. Its
published JSON Schema is
[`contact-transform-result-v1.schema.json`](schemas/contact-transform-result-v1.schema.json).
Asset time mutation and generation-directory publication are separate
collection-producer responsibilities.

The success shape is an illustrative frozen 0.5.0 transform-contract snapshot:

```json
{
  "schema": "urn:animsmith:schema:contact-transform-result:1",
  "schema_version": 1,
  "operation": {"kind": "trim", "version": 1, "interval": {"start": 0.1, "end": 0.9}},
  "input": {"artifact": {"sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", "bytes": 123456}, "dependency_closure_identity": {"sha256": "1111111111111111111111111111111111111111111111111111111111111111", "bytes": 456}, "fragment": {"sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789", "bytes": 789}},
  "outcome": "transformed",
  "event_outcomes": [
    {"event_id": "left-foot/0", "outcome": "transformed", "value": {"time": 0.1625}},
    {"event_id": "right-foot/0", "outcome": "transformed", "value": {"window": {"start": 0.7625, "end": 0.8625}}}
  ],
  "output": {
    "artifact": {"sha256": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210", "bytes": 123456},
    "dependency_closure_identity": {"sha256": "2222222222222222222222222222222222222222222222222222222222222222", "bytes": 654},
    "fragment": {"sha256": "a1d63b5e10381d2aa635cfd74218a4e5d42f3d61c1451765e25d47a23e02d633", "bytes": 729},
    "contact_fragment": {
      "schema": "urn:animsmith:schema:contact-fragment:1",
      "schema_version": 1,
      "producer": {"tool": "animsmith", "version": "0.5.0"},
      "artifact": {"sha256": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210", "bytes": 123456},
      "dependency_closure_identity": {"sha256": "2222222222222222222222222222222222222222222222222222222222222222", "bytes": 654},
      "clip": {"scope": "collection", "logical_id": "com.example.pack/locomotion/walk-forward-in-place", "source": "walk-forward", "take_index": 0, "take_name": "Take 001"},
      "duration_s": 0.96,
      "events": [
        {"event_id": "left-foot/0", "role": "left_foot", "phase": "marker", "time": 0.1625, "confidence": 0.92},
        {"event_id": "right-foot/0", "role": "right_foot", "phase": "begin", "window": {"start": 0.7625, "end": 0.8625}}
      ]
    }
  }
}
```

The delivered contact-fragment producer samples strict stance-support contacts
and writes its own canonical fragment sidecar. Before constructing its metric
grid, the producer requires finite positive timing and checked
`frames * bones` pose cells plus `frames * tracks` sampling work, each at or
below the inclusive one-million V1 CLI-producer ceiling. A refusal preserves
the destination. These resource bounds are not engine or artistic evidence.
The producer does not transform contacts, validate foot
placement, map engine-native event types, merge into a host's final sidecar,
or establish runtime behavior, gameplay meaning, or engine correctness.

## Transition families (#148)

The transition-family V1 identity is
`urn:animsmith:schema:transition-family:1`. A declaration has a stable family
id, an explicit ordered member list, a boundary selection (`entry`, `exit`, or
`both`), and closed typed tolerances with explicit units and basis.

The document-local `transition_families` tables are admitted by AnimSmith's
strict config loader, which retains the exact whole-config input identity with
the normalized declaration. `animsmith evaluate-transition-poses INPUT
--format json` evaluates one document against that authority. The separate
collection envelope is evaluated by `animsmith collection
evaluate-transition-poses COLLECTION.toml --families TRANSITION_FAMILIES.toml
--format json`: it first binds one exact manifest identity and verifies every
logical/source/take witness, then reloads only the source keys selected by the
declared members. A stale manifest or member witness is control (exit 2, no
result); unavailable member input makes its whole family incomplete rather
than comparing a survivor subset.

The format-neutral core provides the strict V1 skeleton-basis identity and
immutable transition-pose evaluation result contract at
`urn:animsmith:schema:transition-pose-evaluation:1`. The document and
manifest-bound collection adapters publish the JSON-only commands above. The
schema is checked in as
[`transition-pose-evaluation-v1.schema.json`](schemas/transition-pose-evaluation-v1.schema.json).
That docs schema is canonical; the core crate ships an exact checked snapshot
so packaged integration tests retain the same contract without a repository
path dependency.
Its `subject_input` field binds the exact raw subject selected by the
declaration scope: the document evaluator supplies document bytes and the
collection evaluator supplies manifest bytes without forking the V1 result
wire. The document evaluator accepts the loader's same-load
`DependencyClosureV1`, derives `subject_input` only from its `primary_input`,
and retains the complete `subject_dependency_closure_identity` both at the
subject and as each member's `source_dependency_closure_identity`. A configured
family cannot be complete unless closure coverage is complete and that identity
is present; otherwise it is `incomplete/not_evaluated` with
`dependency_closure_incomplete`. Empty document declarations still produce
`no_configured_families` without requiring a closure because no source data is
evaluated.

For collection scope, `subject_input` is the manifest identity and no manifest
dependency closure is invented. Each available member instead binds its own
raw `source_input` and complete `source_dependency_closure_identity`. A source
that could not be opened may retain a null `source_input`; an incomplete member
closure leaves its closure identity absent, makes the whole family
`dependency_closure_incomplete`, and does not erase closure identities retained
for other available members. This closure binding covers external glTF buffers
that can change sampled animation while the primary JSON identity stays equal.

Document-local family ids are one lowercase-ASCII token, 1–255 bytes, starting
with `[a-z0-9]` and continuing with `[a-z0-9._-]`. The table key itself is the
id, with no duplicate `family_id` field; quote the key whenever punctuation or
a dot is present (the canonical spelling quotes it always), such as
`[transition_families."walk_to_run"]` or
`[transition_families."combat.entry.v1"]`. Collection family ids retain
Appendix F's slash-qualified logical-id grammar and collection-id prefix.

The declaration distinguishes two ownership scopes:

- `document` families are placed in the existing `animsmith.toml` under
  `[transition_families."<family_id>"]` and resolve exact embedded clip/take
  identities. The reusable config carries no artifact digest; the document
  evaluator binds the primary document `InputIdentity` and complete
  dependency-closure identity in its output.
- `collection` families use a separate declaration envelope, not the
  collection-manifest V1 itself. The envelope binds the exact manifest
  `InputIdentity` `{sha256, bytes}`, then resolves declared logical clip ids
  plus their `source`, take-index, and take-name witnesses.

Members cannot cross scopes, point to paths or animation-array indices, or be
silently removed when missing or ambiguous. `stale_digest` is collection-only:
it means the declaration's manifest `InputIdentity` no longer matches. A
source digest pin in that manifest is enforced by manifest resolution; a
mismatch makes the member unavailable rather than creating a second identity
in this declaration. Reusable document-local config has no stale-digest state
at parse time; the document evaluator binds the exact primary document
`InputIdentity` plus its complete dependency-closure identity and reports
source/take resolution in its output. The existing `[clips]`,
`[gait_groups]`, and `[sync_groups]` sections remain document-local and are
not replaced by a second collection authority. Both placements share the
`transition-family:1` family-record semantics; only ownership and placement
differ. Family declarations are canonicalized by stable family id while
preserving member order.

Both placements cap the exact declaration and normalized envelope at
8,388,608 source or canonical bytes and 16 container levels. They permit at
most 4,096 families, 4,096 members per family, and 16,384 members in aggregate.
Authored strings are at most 4,096 UTF-8 bytes, with the 255-byte identifier
limits applied more strictly. Depth counts each table/array or JSON
object/array on the root-to-value path, including the root as depth 1. Each
exact maximum is accepted; N+1 is rejected during bounded decoding before
retention or unbounded canonical allocation; canonical-byte limits use a
bounded JCS sink.

A collection-scoped declaration uses the following typed TOML-like shape; each
logical id is accompanied by the #409 source/take witness and must agree with
the manifest:

```toml
schema = "urn:animsmith:schema:transition-family:1"
schema_version = 1
scope = "collection"
collection_id = "com.example.pack"
manifest_input_identity = { sha256 = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789", bytes = 9876 }

[[families]]
family_id = "com.example.pack/transitions/walk-to-run"
boundary = "both"

[families.basis]
translation = "skeleton-local-metres"
rotation = "skeleton-local-degrees"
time = "normalized-clip"

[families.tolerances]
translation_m = 0.05
rotation_deg = 5.0
time_normalized = 0.02

[[families.members]]
logical_id = "com.example.pack/locomotion/walk-forward-in-place"
source = "walk-forward"
take_index = 0
take_name = "Take 001"

[[families.members]]
logical_id = "com.example.pack/locomotion/run-forward-in-place"
source = "run-forward"
take_index = 0
take_name = "Take 001"
```

The document-local placement uses the existing config basis and the same
family-record semantics:

```toml
[transition_families."walk_to_run"]
schema = "urn:animsmith:schema:transition-family:1"
schema_version = 1
scope = "document"
boundary = "entry"

[transition_families."walk_to_run".basis]
translation = "skeleton-local-metres"
rotation = "skeleton-local-degrees"
time = "normalized-clip"

[transition_families."walk_to_run".tolerances]
translation_m = 0.05
rotation_deg = 5.0
time_normalized = 0.02

[[transition_families."walk_to_run".members]]
take_index = 0
take_name = "Walk"

[[transition_families."walk_to_run".members]]
take_index = 1
take_name = "Run"
```

The collection envelope's manifest digest/bytes binding makes a manifest edit
or reorder stale even when collection and logical ids are unchanged. No
runtime check or transition graph is implied by either placement.

Both placements normalize to the exact closed JSON envelopes shown in
[DESIGN.md §F.11](../DESIGN.md#f11-transition-family-declaration-v1-148): one
collection form retaining `collection_id` and `manifest_input_identity`, and
one document form in which each quoted table key becomes `family_id`. Families
sort by id, member order is retained, and RFC 8785 JCS produces the normalized
declaration identity. V1 evidence separately binds the exact declaration
source, normalized declaration, and evaluated document or manifest identities.

AnimSmith 0.6.0 ships both the document and manifest-bound collection
transition-pose commands from #153. They emit the V1 result but do not infer
graph edges, add gameplay metadata, or generate engine state-machine or
blend-tree data. Those runtime and gameplay consumers remain follow-up work,
including [#164](https://github.com/mmannerm/animsmith/issues/164).

The complete normative contract, including ownership, strict resolution, and
canonical serialization rules, is in [DESIGN.md §F.10](../DESIGN.md#f10-contact-fragment-v1-147)
and [§F.11](../DESIGN.md#f11-transition-family-declaration-v1-148).
