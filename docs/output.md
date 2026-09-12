# Machine-Readable Output

animsmith's native JSON is the stable source of truth for pipeline adapters.
Text and Markdown lint output are presentation views over the same evaluation
results. The HTML report renders the same typed findings, coverage gaps, and
prediction facets beside its sampled-motion view.

## Skeleton compatibility

`animsmith skeleton compare` emits one immutable
[`skeleton-compatibility-v1.schema.json`](schemas/skeleton-compatibility-v1.schema.json)
(`urn:animsmith:schema:skeleton-compatibility:1`) result. Its control plane is
the exact raw correspondence TOML bytes, recorded as `correspondence.input`.
That TOML pins the exact source and target primary identities, their explicit
selected node-name sets (including each selected root), the declared matching
mode, the resolved explicit mapping (empty for exact-name mode), and every
tolerance. It does not infer skeleton membership from scene
graph reachability. Invalid or stale
control input, unreadable input, or an input beyond the 64 MiB primary-source
bound is an operator error (exit 2) and emits no result.

Each subject records `selected_skeleton_identity`: the exact identity of a
canonical, name-ordered projection of the selected source-node measurements
and selector. This is provenance derived from the loaded asset, not another
authored rest-pose profile.

The result keeps structural topology/rest evidence separate from optional
source skin-membership, inverse-bind, and deformation-model facets. A result
is `compatible` only when every required topology/rest row was available and
within its declared tolerances. Complete mismatches are `incompatible`; mixed
available/unavailable required topology/rest evidence is `partial` even when
another retained field also mismatches; and no evaluable correspondence is
`not_evaluated`. The last three exit 1. Neither the outcome
nor any optional facet establishes retargeting, runtime deformation, masking,
contact, gameplay, or artistic acceptance.

`rows` are deterministic: correspondence-covered source rows are emitted in
source-name order, followed by unmapped source names and then unmapped target
names in their respective name orders. A matched row
contains separate local and rest-world translation, rotation, and scale deltas;
the world rotation value exists only where the existing source measurement
proves a right-handed unit-orthonormal rest-world linear transform. A
child-bone ratio value exists only when both corresponding parent/child world
positions permit an independent nonzero length comparison. Parent
correspondence and child length
use the nearest selected ancestor, walking across unselected intermediate
nodes; they compare the declared selector projection rather than claiming that
the complete raw hierarchies are identical. `missing_source`, `missing_target`,
and `parent_mismatch` rows are explicit rather than folded into a score.

Non-pass rows identify a deterministic `owner_surface` and `remedy_class`.
These route work to the correspondence, selected skeleton authority, selected
skin/bind evidence, or a documented measurement boundary; they do not infer
project, artist, or vendor blame from structure alone. Unavailable source and
target evidence sides carry the same routing fields. The text view renders the
exact source, target, selected-skeleton, and correspondence identities; matching
mode and explicit pairs; tolerances; every retained row delta; facet details;
and remediation routing. JSON remains the stable adapter contract.

## Transition-pose evaluation

`animsmith evaluate-transition-poses INPUT --format json` emits exactly one
immutable `urn:animsmith:schema:transition-pose-evaluation:1` result described
by [`transition-pose-evaluation-v1.schema.json`](schemas/transition-pose-evaluation-v1.schema.json).
It is neither an output-v11 measure/lint envelope nor a check stream. The
result binds the exact input document bytes as `subject_input`, the complete
exact config source as `declaration_input`, and the declaration's independent
normalized identity as `declaration_normalized`. For configured document
families it also binds the complete same-load dependency-closure identity at
the subject and each member, so a changed external glTF animation buffer cannot
be masked by unchanged primary JSON bytes. With no selected or ambient config,
the exact declaration source is the defined zero-byte TOML sequence; an
explicitly empty config is intentionally identical.

An absent or empty `[transition_families]` table is a complete passing result
with `reason: "no_configured_families"`. Complete configured passes exit 0;
findings and every retained incomplete family exit 1 with this same result.
If same-load dependency-closure capture is incomplete, configured families are
retained as `incomplete/not_evaluated` with
`dependency_closure_incomplete`; an empty declaration remains the complete
`no_configured_families` pass because it evaluates no source data. Invalid
config/declaration, contradictory witnesses, input/load failures, and
serialization failures emit no result and exit 2. Because this result has no
sidecar or previously published artifact, failed stdout delivery is also an
operator error (exit 2), rather than a successful result with a best-effort
diagnostic. V1 evaluates one loaded document and does not imply collection
source reload, graph generation, engine runtime behavior, or input mutation.

`animsmith collection evaluate-transition-poses COLLECTION.toml --families
TRANSITION_FAMILIES.toml --format json` uses the same immutable result schema,
with `subject_input` bound to the exact collection manifest bytes. It verifies
the envelope's manifest identity and every logical/source/take witness before
source work. Missing, unreadable, oversized, digest-mismatched, malformed, or
otherwise unusable source input makes its whole family
`incomplete/not_evaluated` with `member_unavailable`; it never compares a
surviving subset. A readable raw source retains its exact `source_input` even
when unusable; only a source for which no complete bytes were available uses
`source_input: null`, and only in a `member_unavailable` family. Per-family
members may span documents but must have one exact normalized skeleton basis.

## Collection lint

`animsmith collection lint COLLECTION.toml --format json` emits the separate
current `urn:animsmith:schema:collection-output:11` envelope. Historical
`urn:animsmith:schema:collection-output:9`,
`urn:animsmith:schema:collection-output:8` (see
[`collection-output-v8.schema.json`](schemas/collection-output-v8.schema.json)),
`urn:animsmith:schema:collection-output:7`,
`urn:animsmith:schema:collection-output:6`,
`urn:animsmith:schema:collection-output:5`, and
`urn:animsmith:schema:collection-output:4` envelopes remain immutable.
It binds the exact manifest bytes to canonically ordered source, logical clip,
and runtime-set records while preserving each set's declared member order.
Every available source embeds its ordinary one-file output-v19 lint result;
each established logical clip separately carries the existing
`ClipMeasurements` value selected by raw source take index and exact authored
take name, then mapped through the loader's observed normalized clip index.
This duplicate-safe indexed projection does not revise the historical,
immutable measurements-v15 name-keyed wire contract; collection-output-v11
embeds measurements-v18 in its nested output-v19 documents. The strict reader
also preserves collection-output-v10 (`urn:animsmith:schema:collection-output:10`)
with its exact nested output-v18 (`urn:animsmith:schema:output:18`) and
measurements-v17 (`urn:animsmith:schema:measurements:17`) pair.
The strict reader
continues to bind historical collection-output-v9 only to output-v17 with
measurements-v16, historical collection-output-v8 only to output-v16,
historical collection-output-v7 only to output-v15,
historical collection-output-v6 only to output-v14, and historical
collection-output-v5 only to output-v13.

Source input, digest pin, config, loader, take inventory, and document-result
states stay orthogonal. A readable digest mismatch can therefore retain its
observed identity, loader result, and take drift without authorizing the clip
binding. Missing/unreadable sources, rejected readable bytes, digest/take
mismatches, and unestablished members produce a complete typed envelope and
exit 1; manifest, rooted-path, selected-config, serialization, and tool errors
exit 2 with no envelope. The frontend preflights the complete control plane
before source execution and reads sources sequentially.

## Collection dashboard

`animsmith collection dashboard --collection COLLECTION-OUTPUT.json -o
DASHBOARD.html --authority DASHBOARD.json` consumes only a strict current
collection-output V11 document. It writes a self-contained offline HTML
presentation and the separate
[`urn:animsmith:schema:collection-dashboard:1`](schemas/collection-dashboard-v1.schema.json)
machine authority. That authority records the exact raw collection-output
identity, bounded source/take/logical-clip and ordered runtime-set projection,
and optional compatible transition-pose evaluation input/result. The optional
result must be transition-pose-evaluation V1 bound to the same manifest input;
other evaluation contracts remain separate rather than being guessed into a
per-member dashboard projection.

Every declared source has its own inventory row even when no logical clip
binds to it. The row retains its safe locator, optional observed input identity,
availability, loader state, dependency-closure state, and bounded observed-take
inventory. Each physical take preserves its exact source-owned take index/name,
normalized index/name state, evidence, and outcome without inventing a logical
ID. Summary evidence totals count physical rows plus unscoped source findings
and unscoped source-level `required_prediction_unavailable` facets; the
logical-clip outcome partition remains a separate declared-row view. Valid
nested findings that have no matching clip are retained as source-level
unscoped findings and included in the dashboard finding total; the projection
never assigns them to an arbitrary clip. Required-unavailable prediction facets
without one exact physical witness are likewise retained once at source scope
with their reason set rather than being guessed onto a take or logical row.

Nested check evidence is normalized-name addressed. If one source has that same
normalized name at multiple physical indices, neither index is a unique witness:
the physical rows carry `duplicate_normalized_clip_name`, name-addressed findings
are counted once at source scope, and no findings, gaps, or predictions are
guessed onto either physical or logical row. An `established` logical row must
instead identify exactly one source-owned established physical row by source,
source-take index, and take name, and its evidence/coverage/outcome must match
that row during strict readback.

When transition evidence is present, a member's optional `logical_clip` is
required exactly when its `source_input`, `take_index`, and `take_name` resolve
uniquely through the dashboard source identities and logical declarations.
Strict authority readback rederives the match rather than trusting the stored
logical ID; absent, mismatched, and ambiguous witnesses remain unresolved.
Runtime-set `gaps` and `lifecycle` are also rederived during strict readback
from the declared member rows: each non-`established` member contributes one
`member_unavailable` gap in member order, and the set is `complete` only when
that derived gap list is empty.

The dashboard is inventory and evidence navigation only. It has no quality
score and does not establish engine loading/playback, retargeting, contacts,
visual or artistic quality, or gameplay acceptance. Caller-supplied per-asset
report links must be exact logical-id keyed safe relative paths; source paths,
licensed assets, and motion samples are never embedded.

## Collection directional-speed evaluation

`animsmith collection evaluate-directional-speed --policy POLICY.toml --evidence
COLLECTION-OUTPUT.json --format json` consumes only strict bounded
`collection-directional-speed-policy:1` TOML and collection-output V7,
historical V6, or historical V5 JSON.
It emits the JSON-only immutable
`urn:animsmith:schema:collection-directional-speed-evaluation:1` result
described by
[`collection-directional-speed-evaluation-v1.schema.json`](schemas/collection-directional-speed-evaluation-v1.schema.json).
The result identifies the raw policy and evidence bytes, retains all declared
members in manifest order, and records incomplete/not-evaluated outcomes or
declared-policy findings without evaluating a subset. Control-plane failures
write no result and exit 2; result-bearing incomplete, not-evaluable, and
finding outcomes exit 1; a complete passing policy exits 0.

Runtime-set evidence concludes only whether every declared membership and required
indexed measurement was established. Its decision is always `not_evaluated`;
it does not infer blending, synchronization, retargeting, controller policy,
engine behavior, or artistic/gameplay readiness. A `gait-group` additionally
projects each member's raw `gait_phase` availability (and phase when measured)
onto that existing manifest-ordered member row. Its set-level
`evidence.gait_phase` records `lifecycle` and `members_measured`; only a
complete group (every declared member established and phase-measured) carries
`phase_spread` and the explicit basis
`max_circular_deviation_from_mean`. The scalar is the existing
`circular_phase_spread` value, not a smallest covering arc, and is calculated
from logical-ID-sorted phases even though member rows retain manifest order.
Incomplete groups keep every member visible and omit both scalar fields;
non-gait sets omit gait-phase evidence. Every declared member also carries raw
`root_travel`: duration, translation availability, signed horizontal X/Z
displacement, sampled horizontal travel, and speed availability/value.
Set-level `evidence.root_travel` counts only members with every required raw
fact and is complete only when all declared members are fully measured; it
never reduces the set or adds direction, ratios, thresholds, or policy.
The strict reader applies a 256 MiB N+1 cap before JSON decoding, validates
current nested output-v19 plus historical collection-output-v9/output-v17,
collection-output-v8/output-v16, collection-output-v7/output-v15,
collection-output-v6/output-v14, and
historical collection-output-v5/output-v13 through their existing reader,
recomputes all summaries/work/set lifecycles, and rejects
unknown fields or contradictory identities and states. Producer and reader
also freeze 1 GiB per primary source, 16 GiB aggregate primary reads, and the
collection-manifest V1 row/member/work limits.
Derived normalized clip names allow at most 4,101 bytes: the 4,096-byte
authored-name bound plus `#` and the largest duplicate ordinal permitted by the
4,096-clip manifest bound. Available nested measurement keys retain
output-v19's 4,096-byte bound.
If such a derived name cannot fit the immutable 4,096-byte text bound of the
nested output-v19 contract, indexed clip measurements and physical binding are
retained, but the nested document and its name-addressed check reference are
`nested_output_unavailable`; the collection exits 1 instead of publishing
schema-invalid nested JSON.
After the aggregate N+1 witness is retained, later declared sources remain in
the envelope as `aggregate_exhausted` with zero inspected bytes; they are not
opened and cannot increase the terminal counter beyond N+1.

## glTF animation addressability

`animsmith generate addressability INPUT` emits a separate, one-file V1
contract with immutable identity
`urn:animsmith:schema:gltf-animation-addressability:1`. Its retrievable schema
is
[`gltf-animation-addressability-v1.schema.json`](schemas/gltf-animation-addressability-v1.schema.json).
It is not an output-v11 measure/lint/diff envelope, and the two roots reject
one another on readback. The staged reader accepts at most 256 MiB and applies
that byte cap before UTF-8 or JSON decoding.

The root contains the normal bounded tool identity, the exact primary input
identity, an engine-neutral inventory, and a nullable `bevy` adapter. The
inventory retains the complete existing dependency-closure record plus raw
source-order animation and channel observations. Source names are optional,
non-unique metadata; source indices remain the identity authority. The
inventory's canonical identity excludes tool metadata and the optional adapter,
so selecting a profile does not change neutral evidence.

With no engine profile or another supported non-Bevy profile, `bevy` is null.
For the exact Bevy revision 1 / 0.19.0 / `gltf-asset-loader` profile, it embeds
the same-load prediction-provenance V1 record and the unchanged
`engine-addressability` check evaluation. The evaluation's available facet
subjects are the authoritative `Animation{source_clip_index}` display
selectors. A required-unavailable facet remains blocking and makes the command
exit 1. Incomplete dependency closure is retained honestly but does not, by
itself, become a claim about runtime load success.

The contract is animation-only. It makes no scene, default-scene, skin,
target-path or UUID, Bevy named-map-winner, extension-support, successful-load,
target-survival, or animation-graph claim. JSON is canonical; `--format text`
and `--format markdown` escape and render the same typed value without adding
conclusions.

### Rich glTF addressability v2

The richer producer is a separate immutable contract,
`urn:animsmith:schema:gltf-addressability:2`; its schema is
[`gltf-addressability-v2.schema.json`](schemas/gltf-addressability-v2.schema.json).
It preserves the V1 animation inventory and adds bounded same-load scene,
node, skin, attachment, scene-path, default-scene, named-map, and animation-
target evidence. V1 readers and the V1 Bevy revision-1 profile remain
unchanged; selecting the richer exact Bevy path selects V2 rather than widening
the historical root.

The raw inventory has independent coverage for scenes, nodes, skins,
attachments, and path candidates. Complete empty domains prove absence;
partial prefixes and unavailable domains never prove absence. Rows retain
source-array indices, authored names, parents and child order, ordered scene
roots, ordered skin joints, node-to-skin attachments, and all-scene
root-to-node path candidates. An authored `skin.skeleton` is retained as an
explicit source observation. Bevy 0.19 ignores that member, so V2 does not
replace it with an inferred Bevy root. The report does not claim scene
instantiation or `SkinnedMesh` attachment.

The exact Bevy adapter is paired with the existing one `engine-addressability`
evaluation and reuses the V1 `Animation{i}` selector primitive. It is pinned
to profile revision 3, Bevy `v0.19.0`, commit
`c6f634ca9f406d68ba5109d921247b654cb42c10`, `bevy_gltf 0.19.0`, locked
`gltf 1.4.1`, and commit-pinned label, loader, path, animation-target, feature,
and root `Cargo.lock` sources. `Scene{i}` is emitted for each declared source
scene;
`Gltf.default_scene` is only a route to an existing `Scene{i}`. There is no
`DefaultScene` label and no fabricated `Scene0`.

Bevy creates `Skin{i}/InverseBindMatrices` eagerly for every declared source
skin, including unreferenced skins; an absent inverse-bind accessor uses the
identity fallback. `Skin{i}` is materialized when any source node references
the skin during Bevy's all-source-node construction pass. Source skin indices,
not the order of Bevy's collected skin values, are identity. Named scene,
animation, and skin maps are separate from typed labels and use source-order
last-write-wins semantics (the skin map follows lazily created skins in first-
reference order).

Target projections are per unique source animation target node and retain
contributing animation/channel identities. Paths use authored node names,
including an authored empty string exactly, or
`GltfNode{source_index}` fallbacks, exclude the scene world-root name, and are
projected only when reachability, hierarchy, dependency closure, feature
settings, and collision checks are complete. A duplicate full path, multiple
scene candidates, target-ID collision, unreachable target, missing
`bevy_animation`, disabled `load_animations`, missing pointer width, or
incomplete evidence is typed `required_unavailable`; no guessed path or UUID
is published. Bevy's target-ID reproduction requires an explicit 32- or
64-bit pointer width because its segment lengths use target-width little-endian
encoding; the host width is never inferred.

The projection also carries `target_coverage`, independently of the retained
target rows. A target domain is complete, including when empty, only when the
raw node/scene/path and animation/channel inventories are exhaustive. Partial
or unavailable evidence makes coverage `required_unavailable`; a domain beyond
the 4,096 row limit additionally reports `target_domain_truncated`, and an
aggregate rich-projection limit uses `projection_bounds_exceeded`. This keeps
positive-prefix target rows from being mistaken for proof of complete,
collision-free coverage.

Each new rich projection domain is capped at 4,096 rows. Its aggregate
structural references are capped at 65,536 and its dynamic projection text at
1 MiB; the sealed embedded V1 animation inventory and `CheckEvaluation` scopes
retain their own bounds. Names and path segments are capped at 1,024 UTF-8
bytes, paths at 4,096 bytes and 256 segments, and the staged report reader at
256 MiB. Target coverage is an explicit projection: a retained canonical
prefix is `target_domain_truncated`, while any new rich projection bound
overflow is `projection_bounds_exceeded`; no retained target is treated as
collision-free after truncation. Strict readback rejects N+1 collections and contradictory identities or states. A second
bounded parse of already captured primary bytes is allowed within the same
loader invocation, but no primary/dependency reopen is allowed. V2 remains
prediction evidence only: it does not certify runtime loading, target
survival, graph wiring, scene spawning, or playback. Required-unavailable
prediction retains the existing exit-1 convention; malformed tuple,
configuration, or input errors remain exit 2.

If the resulting single-check prediction would exceed core's per-file facet
budget, the adapter compacts its unavailable facets to one subjectless
`engine-addressability:facet-budget` scope with `facet_budget_exceeded`. This
is a user-visible compaction marker, not a new check or a claim that omitted
facets were available.

## Engine import advice

`animsmith generate import-advice INPUT` emits a separate one-file V1
contract with immutable identity
`urn:animsmith:schema:engine-import-advice:1`. Its retrievable schema is
[`engine-import-advice-v1.schema.json`](schemas/engine-import-advice-v1.schema.json).
It is neither output-v11 nor the glTF addressability root; strict readers
reject the other shapes. The staged reader applies a 256 MiB byte cap before
UTF-8 or JSON decoding.

Every record embeds exact `PredictionProvenanceV1`: the frozen engine-profile
fact bundle, fully materialized settings, authoritative source format, raw
source facts, primary identity, and dependency closure from the same load.
Each available clip row binds its source-order clip index to one normalized
document index and carries the original source-name observation separately
from the normalized name. It also retains explicit project loop/movement
intent plus duration, speed, loop-endpoint, and declared-frame-grid
measurement/status pairs. Advice identity is a domain-separated canonical
digest of provenance identity, lifecycle, clip rows, and importer payload;
tool build metadata is excluded.

For Unity 6000.3 Generic and Humanoid, the payload projects only settings
already materialized by the frozen profile: Model Importer `Convert Units`,
`Bake Axis Conversion`, Generic `Root Motion Source`, and each clip's
`lockRootRotation`, `lockRootHeightY`, and `lockRootPositionXZ`. `bake` maps to
`true`; `extract` maps to `false`. Unreal 5.8 and Godot 4.7 revision 1 have no
modeled setting vocabulary, so they emit `state: refused` with
`profile_settings_unmodeled` and exit 1 rather than guessing. Incomplete raw
clip inventory, missing source-to-normalized linkage, or mismatched
measurement/settings evidence likewise refuses without exposing a prefix.

V1 does not emit frame-number ranges, sample rates, unit conversions,
root-motion predictions, or project-file mutations. Source seconds multiplied
by a floating-point FPS are not authoritative authored frame coordinates.
Available advice exits 0, typed refusal exits 1, and configuration, profile,
format, input, or serialization errors exit 2. Text and Markdown are escaped
presentation views; JSON remains the contract.

### Revision-2 import-setting advice

The same command also emits the immutable V2 contract
`urn:animsmith:schema:engine-import-advice:2`, described by
[`engine-import-advice-v2.schema.json`](schemas/engine-import-advice-v2.schema.json).
It keeps the output-v15 `tool`, V4 prediction-provenance, and V4 prediction-basis
types by reference, while adding only the V2 `basis` and optional native
`projection` fields. The lifecycle is closed against the same-load dependency
closure: complete closure requires `available`, the exact projection, and no
`refusal_reason`; partial or unavailable closure requires `refused`,
`dependency_closure_incomplete`, and no projection. Unknown tuples, missing
settings, and unsupported formats are configuration errors rather than advice
documents, so they cannot be recast as a guessed projection.

Only these exact profile/input tuples can produce an available V2 projection:

| tuple | accepted input | projection |
|---|---|---|
| `godot` / `2` / `4.7` / `resource-importer-scene` | glTF JSON or GLB | `godot_params`: `animation/fps` (1..120; default 30) and `animation/trimming` (default false) |
| `unreal` / `2` / `5.8` / `fbx-importer` | FBX | `unreal_fbx_import_data`: explicit `sample_rate` as `default_30`, `source_determined`, or `custom_hz(1..48000)` |

The projection is bounded to those documented keys and retains each value's
`explicit_config` or `profile_default` origin. For Unreal, `default_30` maps
to `bUseDefaultSampleRate=true`; `source_determined` maps to `false` with
`CustomSampleRate=0`; a custom rate maps to `false` with its explicit hertz.
This is a deterministic same-load parameter projection only. It does not
execute an engine importer, read back an imported asset, infer frame ranges,
units, skeleton/retargeting, compression, root motion, runtime behavior, or
write project files. The historical V1 contract and its refusal semantics
remain immutable.

## Contract identities

Validation and comparison JSON commands emit output contract v19 with the
current protocol identity `urn:animsmith:schema:output:19`. Output-v17 retains
identity `urn:animsmith:schema:output:17` and remains
retrievable historical schema evidence at
[`output-v17.schema.json`](schemas/output-v17.schema.json), paired with
`urn:animsmith:schema:measurements:16`. Output-v16 remains
retrievable historical schema evidence at
[`output-v16.schema.json`](schemas/output-v16.schema.json); its repository URL
is a retrieval location, not the protocol identity
`urn:animsmith:schema:output:16`. Output-v11 remains
immutable historical schema evidence, and the measurement reader retains its
validation paths for existing reports while CLI producers emit output-v19.
`urn:animsmith:schema:output:15`, `urn:animsmith:schema:output:14`,
`urn:animsmith:schema:output:13`, `urn:animsmith:schema:output:12`, and
`urn:animsmith:schema:output:11` remain immutable historical contracts and are
never retargeted.

The current CLI emits output-v19. The Bevy revision-3 track-support slice
continues to use V5 prediction provenance/readback with the bounded raw
animation/channel inventory and gate outcomes described below. Unity Generic
root-motion uses V6 prediction provenance. This does not retarget or invalidate
output-v16, output-v15, or output-v9; all historical readers remain readable
and their behavior remains preserved.

Those immutable prediction contracts consume measurements-v16. In output-v19,
their basis values are validated against a deterministic measurements-v18-to-v16
evidence view: fully measured loop-continuity rows omit the V17-only per-bone
status, while any unavailable bone projects to the historical unavailable outer
fact; every V18-only `rotation_xyzw` fact is removed from skeleton and skin
linear measurements. The output-v19 report still retains the complete
measurements-v18 rows. This projection cannot turn unavailable evidence into
measured evidence, and a projection or basis mismatch is rejected rather than
weakening the prediction.

Output-v15 retains the V3 provenance/prediction path for revision-1 profiles
and adds `prediction-provenance:4`, `engine-profile-facts:2`,
`resolved-engine-settings:3`, raw scene/attachment inventory V1, machine
results V1, and `engine-prediction:4` for revision-2 profiles. One file cannot
mix the two revisions. Settings retain at most 4,096
canonical clip rows. A 4,097th clip records partial coverage and bounded work;
the identity commits to that state, and lint emits required-unavailable evidence
(`resolved_settings_overflow`) instead of treating the retained prefix as a
complete V1 inventory.

Measurement evidence is nested and independently versioned as
`urn:animsmith:schema:measurements:18`. Its retrievable schema is
[`measurements-v18.schema.json`](schemas/measurements-v18.schema.json). Version
18 adds `rotation_xyzw` to every `LinearTransformMeasurements` surface exactly
when its `classification` is `unit_orthonormal`. It is the canonical XYZW
quaternion for the linear matrix's proper-orthogonal polar factor (the nearest
proper rotation): the classifier deliberately admits small scale and shear
residuals, which a quaternion does not encode. Exact rotations therefore
recompose directly; tolerance-admitted residuals retain their matrix evidence
alongside this orientation fact. Extraction uses all matrix branches near half
turns, and the largest absolute component (XYZW tie order) is always
non-negative, so equivalent `q` and `-q` rotations compare identically.
The field is absent for uniform-scaled, non-uniform, sheared, reflected,
singular, and non-finite transforms. Version 17 adds explicit availability to
every per-bone loop-continuity row. Version
16 (`urn:animsmith:schema:measurements:16`), preserved at
[`measurements-v16.schema.json`](schemas/measurements-v16.schema.json),
added per-primitive measurements in source primitive order. Each primitive
records its nullable source material index, total decoded `POSITION` row
count, and finite row count. Indexed primitives count each stored position
once, not once per index reference. Primitive AABBs and centroids are
finite-only and omitted when the finite count is zero. Version 16 also records
`leading_magic_hex` only for unsupported, nonempty image payloads: at most the
first 16 bytes as lowercase hex. It is bounded evidence, not a guessed format
classification. `urn:animsmith:schema:measurements:15` remains immutable
historical evidence and
has no primitive measurements or leading magic field.
Version 15 added canonical per-bone TRS channel coverage and root-trajectory evidence.
Version 14 introduced a sibling
`_availability` status (`measured`, `not_applicable`, or `unavailable`)
alongside its optional value field, so a consumer can distinguish "this clip
has no subject for the fact" from "the fact applies, but derivation failed" —
a distinction a bare optional value cannot express. Version 13 retains each
per-joint source-declaration inverse-bind matrix beside the observations
derived from it, refuses non-affine sources, and publishes a scale-free
reciprocal infinity-norm condition number before trusting an inversion.
Measurements v14 and earlier, and output v10 and earlier, remain immutable
historical contracts.
The historical output-v10 identity is `urn:animsmith:schema:output:10`.
Output v9 first paired measurements-v15. Output v10 retained it and added the
prediction-provenance substrate described below. Output v11 retains those
contracts and adds
role-resolution provenance: each delivered resolved bone name remains in
`rig.resolved_roles`, while the matching policy appears in the parallel
`rig.resolved_role_policies` map. Its `rig.resolution_outcome` is one of
`resolved`, `coverage`, `ambiguous_exact_match`, `ambiguous_folded_match`,
`role_collision`, or `ambiguous_profile`. Built-in profile fallback is only
unique ASCII case-insensitive matching; explicit `[rig.roles]` entries remain
exact and use `explicit`. Any future nested measurement revision will likewise
require a new outer identity.

`convert --format json` is deliberately a separate conversion-evidence
contract, not another command in the output-v11 envelope. Its immutable
identity is `urn:animsmith:schema:conversion-evidence:2`; its retrievable
schema is
[`conversion-evidence-v2.schema.json`](schemas/conversion-evidence-v2.schema.json).
This lets producers pin conversion provenance independently of measurement
and lint evidence.

An asset-property refusal from `convert`, `assemble`, `generate
contact-fragment`, or `collection transform-foot-cycle` is not success
evidence. Under `--format json` it uses the separate immutable
`urn:animsmith:schema:producer-refusal:1` contract, whose retrievable schema is
[`producer-refusal-v1.schema.json`](schemas/producer-refusal-v1.schema.json).

This keeps conversion evidence v1/v2, assembly evidence v1-v7, output v1-v9,
and scale evidence v1-v5 immutable. The record has `outcome: "rejected"`, a
null `result`, the command, and a typed `{stage, kind, detail}` rejection.

`assemble` writes a separate character-assembly-evidence v7 document to its
required `--evidence` path, and prints the same record to stdout under
`--format json`. Its immutable identity is
`urn:animsmith:schema:character-assembly-evidence:7`; its retrievable schema is
[`character-assembly-evidence-v7.schema.json`](schemas/character-assembly-evidence-v7.schema.json).
The paired GLB and evidence are prepared before publication, so an operator
failure emits neither new destination and restores any prior pair.

`scale` writes scale-evidence v4 for glTF/GLB and the separate v5 FBX
rest/bind record for the narrow FBX path to its required
`--evidence` path, and prints the same record to stdout under
`--format json`. Their immutable identities are
`urn:animsmith:schema:scale-evidence:4` and
`urn:animsmith:schema:scale-evidence:5`; their retrievable schemas are
[`scale-evidence-v4.schema.json`](schemas/scale-evidence-v4.schema.json) and
[`scale-evidence-v5.schema.json`](schemas/scale-evidence-v5.schema.json).
The artifact and its evidence are prepared as temporaries and published as one
pair, so a refusal or an operator failure emits neither destination and
restores any prior pair.

Publication promotes the two temporaries with two renames, which are
individually atomic but not atomic together. Only the artifact destination is
moved aside first, so a process killed between the renames leaves the new
artifact beside the *previous* evidence — a complete pair whose members
disagree, which the evidence's own record of the artifact digest makes
detectable. Backing the evidence up as well would turn that same window into
a new artifact with no evidence at all. Both members are promoted from
temporary files and therefore land with mode `0600` rather than the `0644` a
plain create under the process umask would produce; this is shared by every
producer that publishes a pair.

The `--format json` copy on stdout is a **view of the pair, never a member of
it**. Both producers serialize their record exactly once and hand those same
bytes to the evidence temporary and to stdout, so the file and the stream are
identical by construction rather than by two serializers agreeing. Because the
stream is not a destination, a stdout that cannot take it — a closed pipe, a
full filesystem — changes nothing about publication and does not change the
exit code: the failure is diagnosed on stderr and the outcome stands. Read the
`--evidence` file, not stdout, when the record must be durable.

Conversion evidence v1 remains a historical immutable contract at
`urn:animsmith:schema:conversion-evidence:1`. The current CLI emits v2
exclusively; regenerate v1 evidence when a v2 consumer is required.

[`Output-v2`](schemas/output-v2.schema.json),
[`output-v3`](schemas/output-v3.schema.json),
[`output-v4`](schemas/output-v4.schema.json),
[`output-v5`](schemas/output-v5.schema.json),
[`output-v6`](schemas/output-v6.schema.json),
[`output-v7`](schemas/output-v7.schema.json),
[`output-v8`](schemas/output-v8.schema.json),
[`output-v9`](schemas/output-v9.schema.json), and
[`output-v10`](schemas/output-v10.schema.json) remain historical immutable
contracts, as do output-v11 through output-v16. The current CLI emits
output-v17; `diff` also retains strict version-matched historical readers,
including output-v9.
Regenerate an output-v14 report with a historical producer when a
historical v14 artifact is required.
Regenerate a current output-v19 report from the original asset with
`animsmith measure --format json` when a current artifact is required.

## Contact fragments

[`contact-fragment-v1.schema.json`](schemas/contact-fragment-v1.schema.json)
(`urn:animsmith:schema:contact-fragment:1`) is the strict, portable envelope
for one selected clip's normalized contact facts. Its reader and RFC 8785
canonical serializer live in `animsmith-core`. `animsmith generate
contact-fragment` is the strict V1 producer: it publishes a source-bound
sidecar only after finite bilateral stance-support preflight and a complete
dependency closure; typed refusals publish nothing. It reports sampled support,
not physical contact or gameplay meaning.

The format-neutral core also implements the separately versioned
[`contact-transform-result-v1.schema.json`](schemas/contact-transform-result-v1.schema.json)
(`urn:animsmith:schema:contact-transform-result:1`) reader, canonical serializer,
and exact trim/slice/resample/time-warp contact mapping. A result reader is
given the separately supplied input fragment and an external context containing
the current input/output artifacts, their complete captured closures, the
expected producer, and any handler-produced extension outputs. It validates the
closure-to-artifact bindings and independently rederives every event outcome
and successful inline fragment instead of trusting copied result rows.
Extension transformation requires exact schema/version-preserving output from
an operation-specific handler; opaque payloads are never automatically copied,
and unsupported extensions refuse the whole operation. This library contract
does not mutate an animation asset or publish a collection generation
directory.

## Foot-cycle generation evidence

`collection transform-foot-cycle` publishes two strict success-evidence
contracts outside the ordinary output envelope:

- each `members/NNNNNN/evidence.json` is
  `urn:animsmith:schema:foot-cycle-member-evidence:1`;
- `aggregate-evidence.json` is
  `urn:animsmith:schema:foot-cycle-aggregate-evidence:1`.

The aggregate is serialized once, staged as the generation member, and those
same exact bytes are written to stdout after the no-replace directory publish.
Its resource record includes itself: bounded serialization iterates the byte
count to a stable exact value before publication, then verifies that the four
component totals equal `total_bytes` and do not exceed 256 MiB. Member evidence
binds the exact source/output/closure/contact identities and independent proof
facts; the aggregate binds every deterministic relative path and member
evidence identity in declared order. The retrievable schemas are
[`foot-cycle-member-evidence-v1.schema.json`](schemas/foot-cycle-member-evidence-v1.schema.json)
and
[`foot-cycle-aggregate-evidence-v1.schema.json`](schemas/foot-cycle-aggregate-evidence-v1.schema.json).

A valid asset or capability refusal uses
[`producer-refusal-v1.schema.json`](schemas/producer-refusal-v1.schema.json)
with command `collection-transform-foot-cycle` and exits 1. An operator failure
emits no JSON and exits 2. The transformed contact fragments themselves remain
the existing canonical `contact-fragment:1` contract; the contact transform
result is proof input and is not an extra generation file.

## Common envelope

```json
{
  "schema_version": 15,
  "schema": "urn:animsmith:schema:output:15",
  "tool": {
    "name": "animsmith",
    "version": "0.13.2",
    "source": {
      "revision": "0123456789abcdef0123456789abcdef01234567",
      "dirty": false
    }
  },
  "command": "measure",
  "summary": { "files": 1 },
  "files": []
}
```

`tool.version` is the package's plain semantic version. Source revision and
dirty state are separate fields so automation never has to parse a decorated
version string. Packaged source records its Cargo VCS revision and leaves
`dirty` as `null`; builds without trustworthy VCS metadata may leave both
fields `null`.

Every `measure` and `lint` file record also includes `input`, with the exact
primary-file byte count and lowercase SHA-256 digest of the bytes parsed for
that row. Retain this identity with the JSON evidence when a pipeline promotes
or publishes an asset: it proves which primary payload the recorded result
describes. For multi-file invocations, rows stay in argument order and each
has its own independently calculated identity.

The `input` identity covers only the named primary input file. For profiled
lint output, the sibling prediction provenance carries the bounded same-load
dependency closure, including identities for captured external resources when
coverage is complete. A primary-file digest alone remains insufficient closure
evidence for text glTF or another source with external dependencies.

Operator failures do not emit a JSON envelope. They exit 2, write a diagnostic
to stderr, and leave stdout empty. Content findings exit 1 at the configured
threshold. Any `required_prediction_unavailable` facet also exits 1 and cannot
be suppressed by severity or `--allow`; ordinary engine-neutral coverage gaps
remain nonblocking.

`convert` and `assemble` asset refusals exit 1. JSON mode emits exactly one
producer-refusal v1 document on stdout and leaves stderr empty; text mode
leaves stdout empty and emits one escaped stderr line carrying the same stable
kind. A consumer can therefore branch on exit plus stdout without parsing
English. Failure to serialize a truthful refusal record is an operator error;
failure to deliver already serialized bytes keeps exit 1 and is diagnosed
best-effort on stderr.

## `convert`

`convert --format json` emits one conversion-evidence v2 document. It records
the input and output paths, the requested options, and counts derived from the
written artifact. It is producer evidence: consumers should use the stable
field names and schema identity rather than parsing the text write summary.

That contract is success-only. A rejected source instead emits the shared
producer-refusal v1 record:

```json
{
  "schema_version": 1,
  "schema": "urn:animsmith:schema:producer-refusal:1",
  "tool": {
    "name": "animsmith",
    "version": "0.13.2",
    "source": { "revision": null, "dirty": null }
  },
  "command": "convert",
  "outcome": "rejected",
  "result": null,
  "rejection": {
    "stage": "transform",
    "kind": "transform-refused",
    "detail": "cannot bake static transforms: document has no mesh instances"
  }
}
```

```json
{
  "schema_version": 2,
  "schema": "urn:animsmith:schema:conversion-evidence:2",
  "tool": {
    "name": "animsmith",
    "version": "0.13.2",
    "source": { "revision": null, "dirty": null }
  },
  "command": "convert",
  "input": "prop.fbx",
  "output": "prop.glb",
  "options": {
    "animation_only": false,
    "bake_static_mesh_transforms": true,
    "material_texture_recipe": null
  },
  "artifact": {
    "nodes": 2,
    "animations": 0,
    "meshes": 1,
    "primitive_positions": 3,
    "materials": 1,
    "clips_without_writable_tracks": 0
  },
  "static_mesh_bake": {
    "entries": [
      {
        "source_node_index": 4,
        "source_node_name": "prop",
        "source_mesh_ordinal": 0,
        "source_mesh_index": 7,
        "source_mesh_name": "prop_mesh",
        "output_node_index": 1,
        "output_mesh_index": 0,
        "world_transform": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        "linear_determinant": 1,
        "primitive_count": 1,
        "position_count": 3,
        "normal_count": 3
      }
    ]
  }
}
```

`static_mesh_bake` is absent unless
`--bake-static-mesh-transforms` was requested. Its entries are in deterministic
source-node order. `source_node_index`, `source_mesh_ordinal`, and
`source_mesh_index` identify the source record; names are display data and
need not be unique. `world_transform` is the 16-element column-major rest
world matrix applied to the source positions. `linear_determinant` records the
accepted transform's linear determinant. The output node and mesh indices are
indices in the generated artifact.

The static bake is opt-in and conflicts with `--animation-only`. It only
accepts unanimated, unskinned, singly-instanced static geometry with finite,
non-reflecting, non-singular (including near-singular) transforms. It bakes
positions and inverse-transpose normalized normals into a canonical
identity-root output while retaining indices, UVs, model-supported material
assignments, and embedded base-color and normal textures. Unsupported input is
an operator error, not partial evidence. Repeated same-platform conversion
with the same input and options emits a byte-identical artifact.

When `options.material_texture_recipe` is a path, the top-level
`material_texture_recipe` object is required. When it is `null`, that object is
prohibited. This separates ordinary linked or embedded texture conversion from
recipe provenance. The object records the recipe identity, declared recipe path
and optional root, dimension cap, locked processor policy, and deterministic
consumed/emitted image records:

```json
{
  "schema_version": 1,
  "schema": "urn:animsmith:schema:material-texture-recipe:1",
  "path": "recipes/materials.toml",
  "texture_root": "textures",
  "max_dimension": 1024,
  "processor": {
    "image_crate": "image@0.25.10",
    "png_crate": "png@0.18.1",
    "jpeg_crate": "zune-jpeg@0.5.15",
    "base_color_algorithm": "sRGB-to-linear premultiplied-alpha Lanczos3",
    "normal_algorithm": "tangent-vector Triangle renormalize +Z fallback",
    "data_algorithm": "linear-channel Triangle",
    "output_encoding": "PNG RGBA8 compression=Best filter=NoFilter"
  },
  "consumed_inputs": [
    { "material_index": 0, "material_name": "surface", "slot": "base_color", "declared_path": "surface-base.png", "mime": "image/png", "dimensions": [2048, 1024] }
  ],
  "emitted_textures": [
    { "material_index": 0, "material_name": "surface", "slot": "base_color", "declared_path": "surface-base.png", "mime": "image/png", "dimensions": [1024, 512], "resized": true, "emitted_bytes": 1234 }
  ]
}
```

Each array has the required BaseColor and normal pair plus any declared
metallic-roughness and occlusion slots for every recipe material. Records are
ordered by source-material index, then BaseColor, normal, metallic-roughness,
and occlusion; recipe declaration order does not affect them.
`material_index` is the source-material identity; `material_name` is display
data. `emitted_bytes` is the encoded byte count. See [material texture
recipes](material-texture-recipes.md) for containment and image semantics.

## `assemble`

`assemble` writes character-assembly evidence v7 beside its GLB. The evidence
binds the effective recipe and its SHA-256, every base/clip/recipe/texture input
and digest, selected source takes and windows, exact track-operation counts,
removed named-bone translation deltas, mesh and skin canonicalization, tool
identity, and the final artifact digest and counts. When `prune_constant_tracks`
is enabled, each clip also records the exact removed tracks in
`pruned_constant_tracks`, including each track's index in the completed,
normalized output clip immediately before pruning (the pre-prune authored
order). In assembly evidence v3 through v7, `bone_index` is the BoneId in the
post-canonicalization/pre-node-removal skeleton; `removed_nodes` provides the
stable compaction ledger needed to derive a surviving final index. The array
is empty when pruning is disabled or no track is removed.
`transforms.removed_nodes` records the final structural projection in original
pre-removal node order, with each node's name, original index, nullable
original parent index, and whether it was selected directly. It is empty when
the recipe selects no nodes.
Paths remain
operator-declared; canonical host paths used for containment checks are not
serialized.

Assembly evidence remains success-only. A valid recipe that does not fit the
loaded assets exits 1 before publication and uses producer-refusal v1 in JSON
mode; invalid recipe syntax/schema/values or an input/path/publication I/O
failure remains exit 2 with empty stdout.

When the optional recipe-v7 `rest_bind_scale` operation is active, evidence
also pins the declared root name, every input's resolved root name/source-node
index and applicable source-skin index, the effective staged source selectors and factor, the exact digest
and versioned basis fingerprint for the base and every clip input, each
semantic compatibility result, its `rest-bind` or `skinless-clip-tracks`
application, and the shared scale proof over the exact staged artifact bytes.
Full rest/bind rows name
`urn:animsmith:character-assembly-scale-basis:1`; animation-only clip
projections name
`urn:animsmith:character-assembly-skinless-clip-scale-basis:1` and omit the
inapplicable resolved skin index. A captured clip source may carry a skin or
mesh; its `source_projection` record remains raw-source evidence while the
track-only scale projection excludes geometry, deformation, materials, and
bind state before rebasing. For glTF/GLB, role-specific preflight retains
framing, dependency, raw-coverage, named-skeleton, and animation
accessor/layout safety. The stable `skinless-clip-tracks` value names the
projected operation domain, not the raw source contents. Base and full
rest/bind rows remain strict.
`residual_comparison_counts` uses the same twelve stable
field names as `proof.residuals`, keeping every measured maximum paired with
the count from the shared proof API without changing immutable scale-evidence
v4. V7 retains v6's captured input container and either its raw
glTF preservation boundary or its normalized/baked FBX capability inventory
and private staged-GLB identity. For FBX, that private stage may exclude an
unskinned mesh instance inside a declared `remove_nodes` closure while the
input digest and capability inventory continue to describe the captured raw
source. Omitting the block omits this operation evidence and retains ordinary
assembly behavior under the v7 envelope.

The normative recipe and evidence contracts are
[`character-assembly-recipe-v7.schema.json`](schemas/character-assembly-recipe-v7.schema.json)
and
[`character-assembly-evidence-v7.schema.json`](schemas/character-assembly-evidence-v7.schema.json).
The recipe identity is
`urn:animsmith:schema:character-assembly-recipe:7`; v1 through v6 remain
immutable historical contracts.
See [multi-source character assembly](character-assembly.md) for operation and
consumer-boundary semantics. Migrate from v6 by selecting recipe/evidence v7
and replacing its two source indices with `root_node_name`;
an omitted `rest_bind_scale` block retains the existing behavior, while v3
continues to reject that block as unknown.

## `scale`

The [scale workflow](scale.md) explains when to choose each operation and how
the rewrite/reload/proof/publication transaction produces this record. This
section is the wire-format authority.

glTF/GLB `scale` writes scale evidence v4 beside its artifact. One record serves both
outcomes, discriminated by `outcome`:

| `outcome` | `result` | `rejection` | Published | Exit |
|---|---|---|---|---|
| `"published"` | the record | `null` | the pair, atomically | 0 |
| `"rejected"` | `null` | the typed reason | nothing | 1 |

A refused run's record is printed by `--format json` and is never written to
the `--evidence` path: publication is an artifact/evidence *pair*, and a
refusal has no artifact. This follows `lint --format json`, which prints its
machine-readable result to stdout and exits 1 when the asset has a problem.
For example, a finite negative primary skin weight is rejected during planning
with `rejection.kind: "negative-skin-weight"`; the record has `result: null`
and neither destination is written.
Shared structural refusals likewise keep their specific existing kinds (such
as `invalid-parent`, `invalid-track-shape`, or `invalid-mesh-instance`) rather
than collapsing into a generic document-shape kind.

The record binds the operation and its declared selectors, the operator's
declared paths verbatim, the input digest and byte count, and the complete raw
capability manifest of the source. `capability` is `null` only for a refusal
raised before an inventory existed — bytes that never parsed. A published
record always carries the manifest, because publication is reachable only
through a preflight that built one. In v4 that manifest includes the sorted
JSON-pointer inventory of every static mesh/node and animated morph-weight
source accepted for preservation. Static JSON weights retain their numeric
values, while animated weight accessor payloads remain byte-exact. A published
run adds the fixed tolerance policy by identity and in full, both observed-factor witnesses with the
divergence between them and the ceiling the design expects of it, the affected
node and skin identities in the raw source index space the selectors use, the
rewritten model domains, proof coverage and results, the artifact-level
residuals, and the artifact's own digest, byte count, rewritten accessors and
rewritten JSON pointers.

**Residuals are never published flat.** Each is `{ "evaluated": bool, "max":
number|null }`, with `max` null exactly when the plan's obligations and the
source's own payloads gave that claim nothing to check. A residual reported as
`0.0` for a claim nothing evaluated would be a false record rather than a
missing one, which Appendix D §D.6 forbids.

Every number in the record is guarded for finiteness before serialization.
`serde_json` renders `NaN` and both infinities as `null`, which in a residual
field would read as a checked-but-unmeasurable claim; the producer fails
instead, and publishes nothing.

`proof.read_back_digest_matches` records the producer's own third artifact
check: the staged artifact was re-read from disk and its digest compared with
the bytes that were proved. The two frontend proofs already reload the
candidate, re-run the shared core proof, and re-run the whole rewrite to
byte-compare for determinism; what they cannot see is the write path, which is
what this closes.

Paths are recorded exactly as the operator wrote them. Canonical host paths are
computed only for publication-safety comparisons among input, output,
evidence, and retained external dependency keys. They are deliberately not
serialized, so identical arguments produce byte-identical evidence. Nothing in
the record carries a timestamp.

The normative glTF/GLB contract is
[`scale-evidence-v4.schema.json`](schemas/scale-evidence-v4.schema.json).
The CLI emits v4 for glTF/GLB; immutable
[`scale-evidence-v1.schema.json`](schemas/scale-evidence-v1.schema.json),
[`scale-evidence-v2.schema.json`](schemas/scale-evidence-v2.schema.json), and
[`scale-evidence-v3.schema.json`](schemas/scale-evidence-v3.schema.json)
remain available for historical records.

The narrow FBX `scale rest-bind` path emits v5,
[`scale-evidence-v5.schema.json`](schemas/scale-evidence-v5.schema.json), and
always writes a new `.glb` artifact. Its `capability` is the complete ufbx
normalized-domain inventory, not a raw FBX accessor manifest. `result` binds
the private staged GLB identity and then nests the v4-shaped exact GLB rewrite,
reload/proof, and read-back record for the published candidate. This is a
re-encoding proof, not a claim that FBX bytes, object properties, or authored
curve keys were preserved. Source-aware admission can therefore allow
user-defined properties and bounded external texture/video declarations when
same-load evidence proves they do not affect rest/bind state. The frozen
v5 record retains the inventory counts; the same-load loader source separately
retains raw facts and dependency-closure coverage/identities, but v5 does not
serialize those sidecars. Supported linked texture bytes are captured before
staging so the operation does not silently remove them, without claiming raw
FBX material/texture-assignment preservation. The inventory projection is
frozen with v5: a new FBX fact requires a new evidence identity rather than
silently changing this wire record. Whole-document FBX scaling remains
refused.

A whole-document factor of one has no raw write set. Its v4
`result.artifact.rewritten_accessors`, `rewritten_json_pointers`, and
`reencoded_buffers` arrays are empty, and
`result.proof.artifact.rewritten_accessor_count` is zero. This factor-one
behavior is carried forward unchanged from v3.

For rest/bind, `result.domain_rewrites.scale_animation` is `true`: every
stored scale VEC3 is rebased by its topology multiplier — `1 / s` at the
selected root, `1` at strict affected descendants and unaffected nodes —
including both cubic-spline tangent elements. It is `false` for whole-document
conversion, which preserves dimensionless scale channels. The raw preflight
also refuses an animation channel targeting a node authored with `matrix` as
`animated_matrix_node`; no artifact or evidence file is written for that
refusal.

For an `artifact-proof-failed` refusal from the exact raw-JSON-preservation
walk, v2 added
`rejection.artifact_proof_differences`: the artifact proof's bounded,
deterministically ordered raw-JSON difference sample. `items` has one to 16
entries, each with a JSON-pointer `location` and an
`artifact_added`, `artifact_removed`, or `value_changed` `kind`. `omitted`
counts entries outside the prefix, so the full difference count is
`items.length + omitted`. The field is `null` when that exact-preservation
walk did not supply locations for the refusal. It does not replace
`rejection.violations`, which continues to describe unsupported source
capabilities.

## `measure` and `lint`

Both commands put evidence under `files[].measurements`:

```json
{
  "schema_version": 18,
  "schema": "urn:animsmith:schema:measurements:18",
  "clips": {},
  "mesh_definitions": [],
  "node_instances": [],
  "scenes": [],
  "skeleton_source_coverage": "unavailable",
  "skeleton_nodes": [],
  "skins": [],
  "material_resource_coverage": "complete",
  "material_definitions": [],
  "textures": [],
  "images": []
}
```

`clips` maps clip names to duration, frame count, animated bones, exact local
TRS channel coverage, rotation ranges, optional per-bone loop continuity, and
optional role-dependent gait, foot-seam, root-trajectory, and speed metrics. `bone_channels`
is the canonical set of non-empty local channels present in the measured
document:

```json
"bone_channels": [
  { "bone_index": 0, "bone_name": "hips",
    "properties": ["translation", "rotation"] },
  { "bone_index": 3, "bone_name": "weapon_socket",
    "properties": ["scale"] }
]
```

Rows use skeleton-index order; properties use translation, rotation, scale
order. Bone index is identity and the name is display metadata, so duplicate
bone names remain distinguishable. Duplicate tracks for one `(bone, property)`
pair collapse to one presence fact. This describes surviving normalized TRS
coverage in the artifact regardless of which transforms were requested; it is
not pruning evidence, track multiplicity, morph-weight coverage, or a raw
source-channel inventory. `animated_bones` remains the sorted unique name
projection for compatibility. Empty or structurally malformed channels do not
contribute to `bone_channels`, `animated_bones`, or
`bone_rotation_range_deg`; every rotation-range key is therefore also present
in `animated_bones`.

`mesh_definitions[].primitives` is a source-order array. Each row retains the
nullable source `material_index`, counts decoded base `POSITION` rows in
`vertex_count`, and counts finite rows separately in `finite_vertex_count`.
`primitive_index` retains the original zero-based source slot, so gaps are
valid when a loader omits unsupported source primitives; normalized documents
without source identity use contiguous retained-order fallback values.
Indexed primitives count each stored position once; index references are not
expanded or counted repeatedly. Primitive `geometry_aabb` and
`geometry_centroid` use finite positions only and are absent when there are no
finite positions. Mesh-level totals use the same finite-only geometry domain.
For unsupported nonempty images, `leading_magic_hex` records at most the first
16 payload bytes as lowercase hex. It is evidence of the unsupported bytes,
not an inferred or guessed image format; it is absent for supported images and
other unavailable cases.

Each optional clip fact — `loop_continuity`, `loop_endpoint_mode`,
`frame_grid`, `loop_seam_ratio`, `gait` (and its own `gait.phase`),
`root_trajectory` (with independent nested `translation` and `yaw` facts), and
`speed_mps` — carries a required sibling `<field>_availability` status
(`gait.phase` uses `phase_availability`, nested beside `phase` inside the
`gait` object) with one of three values:

- `measured`: the fact was derived; the sibling value field is present.
- `not_applicable`: this clip has no subject for the fact (for example, no
  declared loop, no resolvable gait roles, or no root/hips travel quantity);
  the sibling value field is absent.
- `unavailable`: the fact applies to this clip, but it could not be derived
  (evidence was insufficient or a producer stopped emitting it); the sibling
  value field is absent.

The value field is present if and only if its status is `measured`; the
schema rejects a `measured` status without a value and a `not_applicable` or
`unavailable` status with one. This distinction matters because the two
absences require opposite handling: legitimate non-applicability should
remain acceptable, while an applicable-but-`unavailable` metric must not
silently pass a consumer threshold that only checks for a present value.

Field-by-field applicability:

| Field | `not_applicable` when | `unavailable` when |
| --- | --- | --- |
| `loop_continuity` | the skeleton has no bones | present bones exist, but the clip has fewer than three samples or the shared seam sampling grid is unusable; individual non-finite bones remain explicit unavailable rows when the shared grid is usable |
| `loop_endpoint_mode` | the clip is not declared `loop = true` | the clip is declared `loop = true`, but neither the strict duplicate-endpoint predicate nor sampled continuity evidence can classify it |
| `frame_grid` | the clip has no declared/configured FPS expectation | an FPS expectation is declared, but the duration or an authored key does not land on that grid |
| `loop_seam_ratio` | the Hips role or every foot role is unresolved, or the Hips + at least one foot role resolved but no real stride was found between the seam-adjacent frames (feet did not move relative to the hips by at least the configured stride floor) — a planted/idle clip has no stride subject to normalize the seam against | the Hips + at least one foot role resolved and a real stride was found, but the ratio itself still could not be derived |
| `gait` | the Hips role or every foot role is unresolved | the Hips + at least one foot role resolved, but the per-clip cycle sample failed |
| `gait.phase` (nested `phase_availability`) | only one side (left or right) resolved a foot role, or both sides resolved but the L-R foot-height signal has exactly zero peak-to-peak swing (so it has no phase subject) | both sides resolved and the L-R foot-height swing was nonzero, but the fundamental-harmonic trough still could not be derived; current finite sampled production data has no known route to this defensive state |
| `root_trajectory` | neither the Root nor the Hips role is resolved | Root (preferred) or the Hips fallback resolved to an index outside the measured skeleton, or its captured name no longer matches that index |
| `root_trajectory.translation` (nested `translation_availability`) | never, once a trajectory bone is selected | the shared metric grid is unavailable or any sampled selected-bone position is non-finite |
| `root_trajectory.yaw` (nested `yaw_availability`) | never, once a trajectory bone is selected | the shared metric grid is unavailable, sampled rotation is non-finite/zero, the fixed heading witness becomes vertical, or an adjacent sampled step is an ambiguous half turn |
| `speed_mps` | neither the Root nor the Hips role is resolved | the Root or Hips role resolved, but root-motion speed could not be derived from the sampled grid |

`root_trajectory` selects the resolved Root role whenever it exists. Hips is
used only as `source_role: "hips_fallback"` when Root is unresolved; a bad or
unmeasurable resolved Root never causes a silent Hips fallback. A valid
selection keeps its `bone_index`, `bone_name`, and source role even when one
or both nested metric domains are unavailable. Translation and yaw are
derived independently, so bad positions do not erase usable yaw and bad
rotations do not erase usable translation.

Collection lint projects the existing per-document `duration_s`, root
trajectory translation availability and X/Z/travel values, plus `speed_mps`
availability/value, into each declared runtime-set member's `root_travel`
object. `evidence.root_travel.members_measured` counts only rows that have all
of those raw facts, while its lifecycle is complete only when that count equals
the declared member count. It retains manifest member order and does not infer
a direction, reference member, ratio, threshold, finding, or controller policy.

The translation domain is normalized right-handed, +Y-up model space in
metres: +X is right and -Z is forward. Signed X/Z and Y displacement compare
the final sample to sample zero. `horizontal_travel_m` sums the XZ length of
every sampled step, so horizontal out-and-back motion does not collapse to a
zero fact. Vertical min/max are sampled signed displacements from sample zero,
include the initial zero, and contain the endpoint displacement.

Yaw chooses at sample zero the positive local Z, Y, or X axis with the largest
finite horizontal projection (ties prefer Z, then Y, then X), records that
`heading_axis`, and holds the witness for the clip. `net_yaw_deg` is the
endpoint-equivalent signed result in `[-180, 180]`; `unwrapped_yaw_deg` is the
signed sum after shortest-step wrap handling; and `yaw_travel_deg` sums the
absolute sampled steps so reversals do not cancel. An adjacent sampled step
within `0.0001` degrees of exactly 180 degrees has no recoverable direction,
so yaw is unavailable rather than guessed. Positive yaw increases
`atan2(x, z)`; for a +Z-aligned witness this rotates +Z toward +X, the positive
right-handed direction about normalized +Y. A multi-step endpoint-equivalent result within the same
`0.0001`-degree tolerance of a half turn is canonicalized to signed `+180` or
`-180` using the sign of the unwrapped result.

These are bounded regression facts from the same inclusive uniform
`MetricGrids` grid used by checks and reports. The grid has the longest
authored channel's key count and needs at least three keys and positive clip
duration. It can alias fast or irregular motion between samples, and adding an
unrelated denser track can change its resolution. Consequently travel,
vertical extrema, winding, and yaw travel are sampled observations, not proof
of continuous-curve extrema or of a transform's authored-data correctness.
Legacy `speed_mps` retains its existing calculation and status.

`diff` reports every change to a clip fact's availability status — not only
the `not_applicable` <-> `unavailable` transition that a bare optional value
would otherwise compare as unchanged (both are an absent value), but also a
`measured` <-> either absence transition, reported under the field's own
`<field>_availability` metric name (for example
`"loop_seam_ratio_availability"`) so it never collides with that field's
ordinary value-movement delta (`"loop_seam_ratio"`, reported `"appeared"` /
`"disappeared"` / `"moved"` as before). This matters most for
`loop_endpoint_mode` and `frame_grid`, which carry an enum and a small object
rather than a plain number: they have no value-movement delta of their own,
so without the status delta a `measured` -> `unavailable` transition on
either would be completely silent.

For clips declared with `loop = true`, `loop_endpoint_mode` is present when
AnimSmith can distinguish a strict mechanically removable
`duplicate_endpoint`, a non-duplicate `unique_cycle` within the effective
position/rotation closure caps, or a `non_closing` cycle beyond either cap.
Clips not declared as loops report `loop_endpoint_mode_availability:
"not_applicable"`; loops without enough finite evidence report
`"unavailable"`. When a positive declared FPS places the duration and every
authored key on its frame grid, `frame_grid` records that `fps` and the
rounded number of `frame_intervals`; `frame_count` remains the longest
authored channel's key count and is not relabeled as an authored FPS grid.

`loop_continuity.bones[]` is present when a clip has at least three samples and
the shared seam sampling grid is usable. Rows stay in skeleton order and carry
`bone_index`, `bone_name`, and `availability`; the numeric index is identity,
while the name is display context and need not be unique. `measured` rows carry
all four numeric fields below. `unavailable` rows carry none of them, so one
bone's non-finite transform evidence cannot suppress usable evidence for other
bones. `not_applicable` is not valid for an existing bone. Each measured row
reports:

- `position_delta_m`: last-to-first model-space position distance (C0);
- `rotation_delta_deg`: last-to-first shortest-path model-space rotation
  difference (C0);
- `seam_velocity_delta_mps`: difference between the model-space linear
  velocities entering the last sample and leaving frame 0 (C1).
- `seam_angular_velocity_delta_degps`: shortest-path model-space angular
  velocity difference, in degrees per second, between those same incoming and
  outgoing steps (rotational C1).

The velocity comparison deliberately uses the two in-clip steps adjacent to
the wrap. The uniform grid contains both `t=0` and `t=duration`, so treating
the duplicate last-to-first endpoint chord as a velocity would report zero on
a perfectly closed loop. Model-space values include ancestor motion; the
rotation chain is composed independently of scale rather than decomposed from
a potentially sheared matrix. These measurements need no rig profile and are
emitted for measurable clips whether or not project configuration declares
them as loops. The `loop-closure`, `loop-seam-vel`, and `loop-seam-rot` checks
judge them only where `[clips.<name>] loop = true`.

`mesh_definitions` contains one record per source mesh definition. Its
`geometry_aabb` reduces finite primitive `POSITION` values in the mesh's own
coordinates; it is independent of every node and scene. When finite positions
exist, optional `geometry_centroid` is their arithmetic mean in that same
mesh-local coordinate domain. Equivalently, it is the finite-count-weighted
mean of the available primitive centroids; primitives with zero finite rows
contribute neither weight nor position. It is not the AABB midpoint and does
not weight vertices by triangle area, volume, or skin influence. Both fields
omit non-finite positions and are absent when no finite positions remain.
Vertex and skin influence statistics are properties of that same definition.

Indexed primitives contribute each base `POSITION` accessor element once,
regardless of how many times the index stream references it. Unindexed
primitives contribute each base position-stream element. These are the same
counting semantics as `vertex_count`. In text output, the value appears as
`geometry centroid (x, y, z)` beside the geometry bounding-box size, or as
`geometry centroid unavailable` when no finite position exists. JSON carries
the optional three-number `geometry_centroid` array.

Each mesh definition has `additional_influence_sets`, an always-present array
of authored secondary glTF skin-attribute sets discovered across its
primitives. Each entry has a numeric `set_index` of at least 1 plus independent
`joints_present` and `weights_present` booleans. The accompanying
`joints_without_weights_present` and `weights_without_joints_present` booleans
preserve unpaired declarations on individual primitives, even when the mesh
also has the complementary side on another primitive. Entries are strictly
ascending by `set_index` and appear at most once. The array is empty when no
secondary set was authored.
It excludes set 0: `max_joints_per_vertex` and the weight-sum extrema retain
their paired primary `JOINTS_0` / `WEIGHTS_0` semantics and do not incorporate
additional sets.

This is source-presence evidence, not secondary-skinning evaluation. animsmith
does not decode the additional per-vertex values into the core skinning model,
include them in weight statistics, repair unpaired sets, or preserve their
payloads when writing a converted asset. Keep the raw source when this evidence
matters; a consuming pipeline decides whether a non-empty or unpaired set is
acceptable.

Material and image evidence is deliberately separate from mesh definitions.
`material_resource_coverage` is `"complete"` for glTF/GLB input and
`"unavailable"` when the loader cannot provide the source-resource sidecar.
For glTF/GLB, complete means the loader inspected the entire documented core
source-resource domain: material, texture, and image definitions plus exactly
the `base_color`, `normal`, `metallic_roughness`, `occlusion`, and `emissive`
material-texture slots. It does not claim that extension-defined texture slots
or every glTF material feature are modeled.

When coverage is complete, `material_definitions`, `textures`, and `images`
are source-indexed records in ascending source order. A material definition
has its optional display `name` and zero or more bindings
`{ "slot", "texture_index" }`. Bindings use the five-slot order listed above;
an emissive-only material therefore has one `emissive` binding rather than an
empty list. A texture record has `texture_index`, optional `name`, and its
`image_index`. This preserves shared images and textures without duplicating
metadata per material slot.

An image record has `image_index`, optional `name`, a `source_kind` of
`"embedded"`, `"data_uri"`, or `"external"`, and optional declared and
decoded metadata. `declared_mime_type` is the source's declared MIME type;
it is source-authored text, not proof of the payload. `detected_container` is
the byte-detected `png` or `jpeg` container. `decoded_color_type` is one of
`l8`, `la8`, `rgb8`, `rgba8`, `l16`, `la16`, `rgb16`, or `rgba16`; its
`channel_count` is respectively 1, 2, 3, or 4, while `width` and `height` are
decoded pixel dimensions. Thus MIME describes a declared media label,
container describes encoded bytes, and color type/channel count describe the
decoded pixel representation. These facts can differ without contradiction.
Available images always have decoded metadata and no `unavailable_reason`.
An unavailable image has a stable `unavailable_reason` and no decoded
dimensions, color type, or channel count; a recognizable corrupt PNG/JPEG may
still report its detected container. Reasons are `source_unavailable`,
`invalid_data_uri`, `unsupported_container`, `decode_failed`, and
`resource_limit`. This is inventory evidence, not an image
acceptance decision: animsmith does not repair, resize, transcode, or judge
color-space, normal-map, engine-import, or artistic suitability here.

The records describe what the loader observed, not authority for later writes
or conversion recipes. Complete coverage does not promise writer or `convert`
preservation of any observed binding or image payload, and a material-texture
recipe remains a separate explicit conversion input. It also makes no image
acceptance, repair, resize, transcode, color-space, or engine-import decision.
Preserve the raw source if those source details must be retained.

`node_instances` contains one record per mesh-bearing node. `node_index` and
`mesh_index` are stable source indices, so names need not be unique. Its
`static_node_world_aabb` transforms the definition's finite positions by that
node's default/rest world transform before reducing. It includes rotation and
negative or non-uniform scale, but explicitly excludes animation, skin
deformation, morph deformation, and runtime world placement. A missing box is
explained by `static_node_world_aabb_unavailable_reason`: no finite positions,
excluded skinned deformation, or a non-finite effective transform. The
producer must emit that reason exactly when the corresponding static box is
unavailable.

`scenes` contains every declared source scene. Each record counts its reachable
mesh-bearing nodes in `instance_count` and unions their available static
node-instance boxes in `static_scene_world_aabb`; `excluded_instance_count`
makes partial coverage explicit. A node can contribute to more than one scene.
Mesh-bearing nodes not reachable from a declared scene remain node instances
but contribute to no scene aggregate. `default_scene_index` is present only
when the source names a default scene; its absence does not select scene zero.

`skeleton_source_coverage` separates a loader that cannot expose source-node
and source-skin identity from a source that genuinely contains no nodes or
skins. When it is `"unavailable"`, both `skeleton_nodes` and `skins` are
empty. When it is `"complete"`, `skeleton_nodes` is the source node table in
ascending `node_index` order. `node_index`, `parent_node_index`, skin index,
and joint index are source identities; names are display data and need not be
unique. `scene_root_indices` lists the source scenes that directly declare the
node as a root, in ascending source-scene order. A joint can name any source
node as its parent, including a non-joint helper node.
Structurally inconsistent source identity tables, such as a missing parent or
parent cycle, downgrade the whole skeleton source domain to `"unavailable"`
instead of publishing a self-contradictory complete table.

Every source node records its source-projected `local_rest`, tagged as `"trs"`,
`"matrix"`, or `"unavailable"`. For glTF this is the exact authored node
member. FBX instead reports ufbx's documented metre/Y-up,
inheritance-compensated projection, which can include adjusted transforms and
generated helper nodes; it is not the raw FBX transform stack. A TRS has
`translation_parent_space_m`, `rotation_xyzw`, and `scale`; the quaternion is
`[x, y, z, w]`. The translation is expressed in the direct parent's coordinate
frame. Even though glTF linear distances use metres, its numeric value is not
directly comparable with a mesh-local or scene-world AABB until ancestor
transforms are composed. A matrix is a 16-element column-major local
transform. Measurements v10 and earlier called the TRS field `translation_m`;
v11 renamed it so adapters cannot accidentally erase the coordinate domain.

`rest_world_matrix` is the separately derived, column-major default/rest world
matrix. `rest_world_translation_m` repeats indices 12, 13, and 14 of that
matrix as a directly consumable world-domain translation. `rest_world_linear`
describes its upper-left 3x3 matrix with X/Y/Z column lengths, determinant,
orientation sign, an optional canonical `rotation_xyzw` for the proper
orthogonal polar factor of a unit-classified linear matrix, an optional common
orthogonal `uniform_scale`, and one stable
classification: `unit_orthonormal`, `uniform_scaled`, `non_uniform`,
`sheared`, `reflected`, `singular`, or `non_finite`. After singularity,
reflection takes classification precedence over shear and scale shape; axis
lengths and the negative orientation retain the remaining evidence.
Measurements v12 derives its finite `f64` axis,
determinant, and cross-axis facts through the shared affine facts, then applies
its tolerant measurement policy: every length is compared to the three-axis
mean using the longer operand, orthogonality is normalized by each axis pair,
and singular/reflected/sheared/unit-or-uniform/non-uniform precedence remains
measurement-specific. This reconciles the `1.0, 1.0, 1.000012` fixture
with shared uniform-affine facts and gives the same observation after an axis
permutation. v11 remains the immutable historical contract with its former
derivation. Relative `1e-5` orthogonality/equal-axis tolerances and a
scale-relative `1e-6` determinant tolerance make the result independent of a
uniform choice of source units. The derived numeric facts are calculated and
serialized with `f64` precision so scale-cubed determinants do not overflow or
underflow across the finite `f32` source-matrix range. A
non-finite or transitively unavailable world matrix carries its typed
unavailable reason and a `non_finite` linear classification without fabricated
numeric fields.

The local representation is never silently decomposed or replaced by the
world transform. A declared scene is membership only, not a transform domain:
`scene_root_indices` is membership evidence and adds no transform. A
transformed node selected as a scene root retains its own projected local-rest record,
and its ancestors determine the derived world matrix. These composition rules
follow the [glTF node hierarchy and transform
definition](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#nodes-and-hierarchy).

`skins` is a source-skin table in ascending `skin_index` order. Its `joints`
are in declared skin-slot order; each joint row owns its `joint_index`,
`node_index`, `joint_bind_to_mesh`, and `mesh_bind_world` observations.
`skeleton_root_node_index` is present only when the source explicitly declares
one. The wire field `inverse_bind_accessor` reports whether the source
inverse-bind declaration was absent, readable, empty, count-mismatched, or
unreadable. A readable finite declaration retains its column-major matrices in
slot order. For glTF these are exact accessor values. For FBX they are matrices
projected from ufbx's converted cluster binds, not raw FBX payloads. Absent,
malformed, and non-finite inverse-bind evidence is not inferred from node-local
rest data.

Each skin's `attachments` names every source node that declares use of that
skin, in source-node order. In each joint row, `joint_bind_to_mesh` is
`inverse(inverse_bind_matrix)`, so it maps joint bind-local coordinates into
the mesh-local bind domain declared by that skin; `mesh_bind_world` is
`joint_rest_world * inverse_bind_matrix`, mapping that mesh-local bind domain
into world bind coordinates when the projected rest and bind evidence agree. These
remain per-joint observations rather than claims that rows agree. Attachments
are identity evidence, not an extra transform folded into either calculation.
Each available derived field preserves its finite matrix and adds the same
`linear` facts used by rest-world nodes. Otherwise it carries a typed
unavailable reason. Both observations repeat the exact finite
`source_inverse_bind_matrix` for their retained declaration slot, even when a later
derivation is unavailable, so a consumer can verify the arithmetic without a
second source decoder. For glTF that matrix is the exact finite accessor
value; for FBX it is the finite matrix derived from ufbx's converted cluster
binds and is explicitly not a raw-payload preservation claim.
`joint_bind_to_mesh.inversion_quality` reports
`1 / (norm_inf(A) * norm_inf(inverse(A)))` for the source linear 3x3. This
scale-free value is zero for a singular source and approaches zero as forward
error amplification grows. Values at or below `1e-6` refuse as
`inverse_bind_matrix_ill_conditioned`; an exactly singular linear part uses
`inverse_bind_matrix_non_invertible`. A source bottom row must be affine within
absolute `1e-6` of `[0, 0, 0, 1]`, which admits ordinary binary32 round-trip
noise while refusing projective matrices as `inverse_bind_matrix_non_affine`.
`mesh_bind_world` has no inversion quality because it multiplies rather than
inverts the retained source-declaration matrix. `joint_bind_linear_summary` summarizes only
`joint_bind_to_mesh`: it reports joint/available/unavailable counts and
distinguishes a shared `consistent_uniform` factor, differing `mixed_uniform`
factors, `non_uniform_or_sheared`, `reflected_or_singular`, mixed groups, and
partial or total unavailability. The detailed joint rows remain authoritative.
These are descriptive calculations only. animsmith does not decide whether a
consumer requires a joint, whether a skin/rest comparison is close enough,
which root is canonical, or whether an unavailable matrix is acceptable.

For example, an exporter may put a `0.01` scale on an ancestor and author a
child socket translation of `11.5`. The local value remains
`11.5` parent-space metres while the accumulated rest-world contribution is
approximately `0.115 m`. A skin can still render at its intended size when an
inverse bind contributes the compensating factor: a joint may show effective
rest-world scale `0.01`, retained inverse-bind scale `100`, joint-bind-to-mesh scale
`0.01`, and mesh-bind-world scale `1`. That compensation applies to the
skinning equation; an ordinary weapon,
effect, or collision object parented to the socket generally inherits the
socket's non-unit affine scale. The measurements expose all three domains so a
consumer can apply its own attachment policy. They do not infer authored units
from character height, mesh size, or any other plausibility heuristic. The
matrix relationship follows the [glTF skinning
definition](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins).

These facts help an artist or engine integration diagnose common handoff
questions without reparsing the source: whether a joint is nested below a
helper, whether a transformed scene root participates, whether two mesh nodes
reuse a skin, and whether inverse-bind data is missing or singular. The node
and skin terminology follows the [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html).
They do not select a required bone list, validate an engine's tolerance,
retarget, normalize, repair, or rewrite the asset; those remain consuming
project policy.

Lint adds exactly one `files[].checks[]` record for every built-in catalog
check. Each record keeps these dimensions independent:

- `selection`: `selected` or `unselected`;
- `configuration`: `enabled` or `disabled`; a check is disabled when its
  severity is `off` or its built-in policy is opt-in and no enabling severity
  was configured;
- `applicability`: `applicable` or `not_applicable`;
- `evaluation`: `complete`, `partial`, or `not_evaluated`;
- content `findings`;
- completed `evaluated_scopes` and typed coverage `gaps`.

Gap and scope `code` fields are the machine contract; `message` is display
text and must never be parsed. Disabled, unselected, and not-applicable checks
are not artificial gaps. A partial check has at least one completed scope and
at least one gap. Applicable work that completed nothing has a gap and no
content findings. A scope can appear as completed and also be named by a gap
when a group-level calculation covered some but not all members.

Built-in gap codes are:

| Gap code | Meaning | Emitted by |
|---|---|---|
| `roles_unresolved` | Required semantic rig roles were not resolved. | `loop-seam`, `root-motion-speed`, `in-place`, `foot-slide`, `gait-group`, `time-complement` |
| `measurement_unavailable` | A required numeric measurement could not be produced or did not meet its evidence floor. | `loop-closure`, `duplicate-loop-endpoint`, `loop-seam`, `loop-seam-vel`, `loop-seam-rot`, `root-motion-speed`, `in-place`, `foot-slide`, `gait-group`, `sync-group`, `time-complement`, `rest-world-scale` |
| `skeleton_unavailable` | Required skeleton presence work could not run because the file has no usable skeleton. | `required-bones` |
| `node_selector_no_match` | A configured source-node selector matched no named source node. | `rest-world-scale` |
| `node_selector_ambiguous` | A configured source-node selector matched more than one named source node. | `rest-world-scale` |
| `insufficient_measurable_members` | Fewer than two configured group members produced usable comparison evidence. | `gait-group`, `sync-group`, `time-complement` |
| `members_not_evaluated` | Some configured group members did not produce usable comparison evidence. | `gait-group`, `sync-group`, `time-complement` |
| `invalid_declared_fps` | A declared frame rate was zero, negative, or non-finite. | `fps` |
| `sync_frame_grid_unavailable` | A same-time sync-group member lacks usable declared frame-grid evidence. | `sync-group` |
| `insufficient_rotation_evidence` | Too few usable rotation tracks existed for a bind-pose comparison. | `bind-pose` |

Built-in completed/gap scope codes are:

| Scope code | Work unit | Emitted by |
|---|---|---|
| `loop_closure` | One named clip's per-bone model-space pose closure was measured. | `loop-closure` |
| `duplicate_loop_endpoint` | One named clip's authored tracks were analyzed for redundant closing endpoint keys. | `duplicate-loop-endpoint` |
| `member_existence` | Configured group members were checked for existence. | `gait-group`, `sync-group`, `time-complement` |
| `phase_measurement` | One named clip's gait phase was measured or lacked usable evidence. | `gait-group`, `time-complement` |
| `phase_coherence` | One named group's measurable gait phases were compared. | `gait-group`, `time-complement` |
| `sync_member_measurement` | One named same-time sync-group member's timing evidence was measured. | `sync-group` |
| `sync_compatibility` | One named same-time sync group had compatible member timing evidence compared. | `sync-group` |
| `loop_seam` | One named clip's positional loop seam was measured. | `loop-seam` |
| `loop_seam_velocity` | One named clip's per-bone model-space seam velocity continuity was measured. | `loop-seam-vel` |
| `loop_seam_rotation` | One named clip's per-bone model-space angular seam velocity continuity was measured. | `loop-seam-rot` |
| `required_bone_presence` | Configured structural skeleton-bone presence requirements were evaluated. | `required-bones` |
| `selected_node_rest_scale` | One configured source-node selector resolved and its effective rest-world linear scale was evaluated. | `rest-world-scale` |
| `root_motion_speed` | One named clip's root-motion speed was measured. | `root-motion-speed` |
| `travel_mode` | One named clip's XZ movement-owner declaration was judged. | `in-place` |
| `foot_stance` | Whole-clip prerequisites for stance analysis were evaluated. | `foot-slide` |
| `left_foot_stance` | The named clip's left foot/toe stance was evaluated. | `foot-slide` |
| `right_foot_stance` | The named clip's right foot/toe stance was evaluated. | `foot-slide` |
| `frame_grid` | The named clip's declared frame grid was evaluated. | `fps` |
| `first_frame_rest_delta` | The named clip's first-frame/rest-pose rotation evidence was evaluated. | `bind-pose` |
| `animation_asset_label` | One source animation index was projected to the selected engine profile's canonical asset-label selector. | `engine-addressability` |
| `animation_asset_label_inventory` | Complete source-animation inventory required for asset-label prediction was unavailable. | `engine-addressability` |
| `scene_asset_label` | One source scene index was projected to the selected engine profile's canonical asset-label selector. | `engine-addressability` |
| `default_scene_route` | The source default-scene observation was projected to the selected engine profile's route to an existing scene asset. | `engine-addressability` |
| `skin_asset_label` | One source skin index was projected to the selected engine profile's conditional canonical skin asset-label selector. | `engine-addressability` |
| `inverse_bind_matrices_asset_label` | One source skin index was projected to the selected engine profile's canonical inverse-bind-matrices asset-label selector. | `engine-addressability` |
| `named_addressability_map` | One selected engine profile named-addressability map and its duplicate-name policy were evaluated. | `engine-addressability` |
| `animation_target_id` | One unique source animation target node's exact path and target identifier were evaluated. | `engine-addressability` |
| `gltf_addressability_inventory` | Complete raw glTF scene, node, skin, attachment, and path evidence required for rich addressability prediction was unavailable. | `engine-addressability` |
| `engine_clip_boundary` | One source animation clip's exact end-frame boundary was evaluated. | `engine-clip-boundary` |
| `engine_clip_boundary_inventory` | Complete exact source-animation boundary inventory was unavailable. | `engine-clip-boundary` |

The built-in gap and scope declarations in `animsmith_core` are authoritative
for each code's identity, meaning, and allowed emitting check ids. Runtime
evaluation rejects a built-in code from an undeclared emitter, and the output
contract test derives this reference inventory from those same declarations.
The public code slices let consumers enumerate or allow-list animsmith's
built-in vocabulary; the meaning/emitter registry remains an implementation
detail. Custom checks may add namespaced gap codes and their own namespaced
scope vocabulary.

`summary.checks` reports a `total` and four independent partitions. Each of
`selection`, `configuration`, `applicability`, and `evaluation` sums to that
same total. `summary.checks.gaps` counts typed gaps, while
`summary.findings` counts content findings by severity.

### Engine-prediction provenance and scoped facets

Every output-v19 lint file has required nullable `prediction_provenance`. It is
`null` when no exact engine profile was resolved. Revision-1 profiles carry
immutable prediction-provenance v3 (`urn:animsmith:prediction-provenance:3`): the typed
profile facts and sources, authoritative input format, bounded resolved
document/per-clip settings, V2 raw-source facts including optional exact
source-timing observations, the same-load dependency closure, and consumed contract
identities. The header, profile, settings coverage/work, raw evidence, closure, and primary
input identities are cross-validated; host paths and arbitrary JSON are
forbidden.

Output v10 also carries the first bounded production rule. For the exact Bevy
revision 1 / 0.19.0 / `gltf-asset-loader` profile on glTF or GLB,
`engine-addressability` predicts the canonical
`GltfAssetLabel::Animation(source_clip_index)` display selector. Each available
`animation_asset_label` facet uses the exact `Animation{index}` selector as its
scope subject. Names do not control the selector, and the index may change when
the source animation order changes. Complete empty source-animation inventory
is not applicable. Partial or unavailable inventory emits one subjectless
`animation_asset_label_inventory` required-unavailable facet and no prefix
predictions.

Output v14 introduced the bounded `engine-clip-boundary` rule for the exact
Unreal revision 1 / 5.8 / `fbx-importer` profile. Its FBX adapter supplies the
parser-resolved absolute animation-stack end coordinate and exact frame period
as generic exact source-timing evidence. The rule reports only whether that end
coordinate lies on the exact frame lattice. Missing, partial, or unavailable
evidence becomes a required-unavailable facet; it does not infer frame ranges,
resampling, root motion, or other importer behavior.

Output v15 adds `engine-unit-scale` for the exact Bevy revision 2 / 0.19.0 /
`gltf-asset-loader` profile on glTF JSON or GLB. V4 provenance binds exact
profile facts, fully materialized settings with value origins, the complete
alias-normalized runtime-node selector declaration, raw source and dependency
evidence, and a bounded same-load scene/attachment/primitive inventory.
Available facets carry typed machine results for exact unit mapping,
loader-created scene entities, primitive children, and selected source nodes.
The caller-owned `WorldAssetRoot` and arbitrary application world-unit policy
are outside the result. Unsupported matrix ancestry, selector miss/ambiguity,
incomplete evidence, and exhausted facet work remain typed, unsuppressible
required-unavailable states; this prediction-only rule emits no content
findings and authorizes no rescaling.

The current successor adds the narrow Bevy revision-3
`engine-track-support` prediction through V5 provenance and the output-v19
contract. Its same-load raw animation inventory is bounded by both animation
coverage and independent per-animation channel coverage. With a complete
inventory, the two gate settings produce only negative outcomes for dropped
animation/channel rows; a dropped row is not a content finding. The compiled
feature gate takes precedence over `load_animations`. When both gates allow
loading, the result is a stable required-unavailable runtime-survival facet,
because this contract does not execute Bevy or claim positive runtime
survival.

Complete-empty animation inventory is not applicable. Partial or unavailable
inventory emits exactly one subjectless, unsuppressible inventory
`required_prediction_unavailable` facet and no retained-prefix prediction;
bounded N+1 work is therefore distinguishable from a complete inventory.
The ordinary lifecycle remains explicit: all available work is `complete`, a
mix of available and unavailable facets is `partial`, and all required-
unavailable work is `not_evaluated`. V5 readback validates result state,
facet scope, inventory-row basis, and provenance identity under the same
bounds before accepting an available result; malformed or contradictory
serialized claims are rejected rather than repaired into a prediction.
Extensions, unsupported animation constructs, and other positive runtime
claims remain outside this revision-3 slice. Output-v17, output-v16, and
output-v15, V4 provenance, and revision-2 behavior remain preserved and
readable; output-v19 carries the immutable V5 provenance for the Bevy slice,
while Unity Generic
root-motion uses V6 provenance.

For the exact Unity Generic revision-2 / 6000.3 / `fbx-model-importer` tuple,
the `engine-root-motion` check is serialized in output-v19 with immutable
`urn:animsmith:prediction-provenance:6` and `urn:animsmith:engine-prediction:6`
identities. V6 extends the same-load V5 evidence with the raw FBX transform-path
inventory and the normalized per-clip movement-owner intent. Each declared
clip/axis facet records a `RootMotionRouting` result: `gameplay` must pair with
`baked_into_pose`, and `animation` must pair with `stored_as_root_motion`.
Conflicts are ordinary error findings bound to the available facet's
`prediction_scope`.

The prediction lifecycle is independent of the ordinary measurement checks.
An all-available run is `complete`; mixed available and unavailable facets are
`partial`; all required-unavailable facets are `not_evaluated`. Missing or
ambiguous source-path resolution, a path that does not identify the explicitly
resolved `Root` role, incomplete raw-path or project-intent coverage, settings
overflow, duplicate clip names, or unavailable translation/yaw evidence yields
`required_prediction_unavailable`. Such a facet is not a content finding and
cannot be suppressed by `--allow`; it makes lint exit 1. A missing `Root` never
falls back to `Hips` for this rule. The consumer-neutral measurement layer may
still use `Hips` as its historical fallback, but root-motion prediction
requires the resolved Root source role.

Project-intent scans retain at most each bounded source/declaration prefix plus
one overflow witness. An ownerless unmapped tail is represented as `NPlusOne`
intent work and yields one atomic unavailable summary; only complete evidence
that no owner is declared preserves the check's not-applicable result. Raw path
evidence is required once declared work exists.

The numeric root trajectory is evidence, not a routing threshold. Translation
and yaw availability must be `measured` for the corresponding axis; zero
travel, nonzero travel, and any other finite magnitude follow the same
owner-versus-setting comparison. This check performs no Unity editor
execution, imported-asset readback, runtime playback, or engine certification.

Raw FBX paths use a closed, case-sensitive, byte-exact grammar: `/` is the only
separator and there is no escaping or Unicode normalization; segments are
nonempty, with no leading/trailing or doubled separators, `.`/`..`, backslashes,
controls, or Unicode format characters. Segment/path/depth limits are 1,024
UTF-8 bytes, 4,096 bytes, and 256 segments. The same input bytes are projected
through a raw-preserving ufbx load so source identities, parent chains, and
names remain auditable. The implicit ufbx root and generated geometry/scale
helpers are retained as evidence but cannot match. Only complete inventory
coverage makes zero matches a proven `NoMatch`; partial or unavailable
coverage is `CoverageIncomplete`.

Resolved engine settings and prediction provenance v3 use explicit bounded
N+1 work and coverage evidence. A 4,097th clip produces partial settings
coverage rather than an honest complete prefix; engine facets report the typed
`resolved_settings_overflow` reason. Current lint allocates the shared 4,096
facet budget before evaluation and emits one canonical rule-scoped
`facet_budget_exceeded` summary when candidates are omitted.

This is selector evidence only: it does not establish that Bevy loaded the
input, that animation loading was enabled, that an asset exists at runtime, or
that targets and graph wiring survived. Other exact profiles currently retain
nonnull provenance with no production prediction. Engine-backed checks attach
one `prediction` object to their existing check record. Its canonical `facets`
are keyed by the existing
`EvaluationScope` so one check can retain an available result for one clip or
selector and a required-unavailable result for another.

A facet state is exactly `available` or
`required_prediction_unavailable`. Available facets have nonempty typed basis
and no reasons. Required-unavailable facets keep any available basis prefix and
one or more stable reason codes; they are not content findings or generic
coverage gaps. Available scopes occur in `evaluated_scopes`; unavailable
scopes occur in neither completed scopes nor gaps. Consequently an
all-available check is `complete`, mixed available/unavailable work is
`partial`, and all-unavailable work is `not_evaluated` under the existing
single evaluation lifecycle.

Findings on a prediction-bearing check include `prediction_scope` and must bind
to exactly one available facet. Severity and `--allow` may filter or block a
content finding but never suppress required-unavailable prediction work.
`summary.prediction_facets` contains explicit `available` and
`required_prediction_unavailable` counts; any nonzero unavailable count makes
lint exit 1.

Basis rows are closed typed references to embedded profile facts, resolved
settings, project/config fields, raw-source facts, measurements-v16 scalars, or
primary sources. Measurement references use bounded canonical JSON pointers
and exact scalar values. A schema-valid measurement availability of
`unavailable` may make prediction work unavailable. A malformed or non-finite
present measurement remains a contract error and exit 2.

The report reader caps each serialized input at 256 MiB before UTF-8 or JSON
parsing. It validates provenance and measurement-independent prediction links,
then the complete version-matched measurements contract, then
measurement-pointer values, before `diff` can extract measurements. Output v10
and earlier inputs receive
the normal regeneration guidance.

`lint --format json` deliberately rejects `--allow` so machine evidence is
never deleted. `--allow` remains available for text and Markdown presentation
and their exit policy. Text and Markdown render coverage gaps separately from
findings and group repeated gaps by `(check_id, code)` for readability. Group
counts still reflect every underlying per-scope JSON gap.

## Findings and numeric values

Findings carry `check_id`, `severity`, optional `clip`, `bone`, `node`,
`time_s`, `measured`, and `expected` fields, plus a human message. `node` is a
source-node path whose components include stable source indices; it is
distinct from the normalized skeletal `bone` context used by animation
checks. Treat `check_id` and
the structured fields as automation data; treat `message` as display text.
Group-level findings may also carry `members`, a configured-order array whose
rows contain the member name and a key-sorted map of scalar measurements.
Missing or unavailable evidence stays explicit in those rows; consumers do
not need to parse the finding message to recover the comparison table.
The nested `check_id` intentionally repeats its owning check record so a
finding stays self-describing when extracted or consumed through the embedded
API; the evaluator rejects mismatched parent/child ids.
For `loop-closure` and `loop-seam-vel`, `expected` is the effective cap for
that finding's clip after exact-name and glob expectations are resolved, with
the corresponding global check setting or built-in default as fallback.

Numeric equality in the JSON contract means equality of decoded JSON numbers,
not byte-for-byte lexical spelling. For example, `1`, `1.0`, and `1e0` denote
the same numeric value to a conforming adapter.

## `diff`

`diff --format json` uses the current output-v19 header and emits `inputs`, a
delta count, and structured metric deltas:

```json
{
  "schema_version": 19,
  "schema": "urn:animsmith:schema:output:19",
  "tool": {
    "name": "animsmith",
    "version": "0.13.2",
    "source": { "revision": null, "dirty": null }
  },
  "command": "diff",
  "inputs": { "before": "old.glb", "after": "new.glb" },
  "summary": { "deltas": 1 },
  "deltas": [
    { "clip": "walk", "metric": "speed_mps", "before": 1.0, "after": 1.2, "note": "moved" }
  ]
}
```

`diff` accepts asset files, current output-v19 with measurements-v18, or
historical output-v17, output-v16, output-v15, output-v14, or output-v13
`measure`/`lint` reports with measurements-v16, and
version-matched historical output-v11/v12 reports with measurements-v15.
The output-v9 contract and its measurements-v9 pairing remain immutable
historical evidence; output-v10 and earlier reports are rejected with guidance to
regenerate them from the original asset. Multi-file reports and other
unsupported contract versions are also rejected as operator errors.
Before extracting the clip metrics it uses,
`diff` validates the complete measurement record, including mesh evidence, and
rejects malformed or non-finite payload values.

Loop-continuity rows compare by `bone_index`; availability changes are reported
at `loop_continuity.bones[N].availability`. Numeric fields are compared only
when present on each side. Re-export noise at or below
0.001 m for `position_delta_m`, 0.1 degree for `rotation_delta_deg`, and
0.01 m/s for `seam_velocity_delta_mps` is silent; the 0.5 degree/s floor
applies to `seam_angular_velocity_delta_degps`. Larger changes produce metric
paths such as `loop_continuity.bones[12].rotation_delta_deg`. These are diff
significance floors, not the lint caps configured under
`[checks.loop-closure]`, `[checks.loop-seam-vel]`, and `[checks.loop-seam-rot]`.

Channel-coverage membership compares the canonical `(bone_index, property)`
set, so a display-name-only change is not reported as lost/gained coverage.
Every root-trajectory status, selected-bone/source identity, and heading-axis
change is exact. Each translation field uses a 0.001 m significance floor;
each yaw field uses 0.1 degree, with circular 360-degree distance only for
`net_yaw_deg`. Unwrapped yaw and yaw travel compare ordinarily, so `0 -> 360`
remains visible even when net yaw is unchanged.
