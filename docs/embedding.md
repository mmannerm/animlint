# Embedding animsmith in a pipeline

The CLI is one frontend. Asset pipelines that already own their contract
format, importer, build graph, and gate can call the same loaders,
measurements, and checks in process.

This guide covers integration decisions. Symbol-level contracts belong in
rustdoc, the [pipeline scenario guide](pipeline-scenarios.md) owns the larger
raw-to-game-ready process, and the [examples cookbook](../examples/README.md)
owns runnable CLI transcripts.

## Choose the crates

| Crate | Use it for |
|---|---|
| `animsmith-core` | Required embedding boundary: data model, rig roles, config, sampling, measurements, diffs, checks, and findings. No file I/O or format dependency. |
| `animsmith-gltf` | Load glTF/GLB, write glTF/GLB, or apply byte-surgical glTF repairs. |
| `animsmith-fbx` | Load FBX through `ufbx`; adds a bundled C build. Omit it from glTF-only pipelines. |
| `animsmith-engine` | Resolve one exact built-in consumer/importer profile and fully materialized typed importer settings. No TOML, file I/O, format crate, or engine SDK dependency. |
| `animsmith-report` | Render self-contained HTML from the same sampled grids and findings. |

The `animsmith` crate is the CLI binary, not a library facade.

```toml
[dependencies]
animsmith-core = "0.13"
animsmith-gltf = "0.13"
# Optional:
animsmith-fbx = "0.13"
animsmith-engine = "0.13"
animsmith-report = "0.13"
```

docs.rs is the canonical reference for published APIs. The stable package
URLs are:
[animsmith-core](https://docs.rs/animsmith-core),
[animsmith-gltf](https://docs.rs/animsmith-gltf),
[animsmith-fbx](https://docs.rs/animsmith-fbx),
[animsmith-engine](https://docs.rs/animsmith-engine), and
[animsmith-report](https://docs.rs/animsmith-report). For the current workspace
state, build the same rustdocs locally with `just doc`.

Rustdoc owns signatures, type invariants, lifetimes, and the `Errors` and
`Panics` contracts. In particular, start at the `animsmith-core` crate root;
it is the compact API map rather than another copy of this guide.

The compiling end-to-end example is
[`crates/animsmith/examples/embed.rs`](../crates/animsmith/examples/embed.rs):

```console
cargo run -p animsmith --example embed
```

## Integration flow

1. **Load a source.** Use `animsmith_gltf::load_source` or
   `animsmith_fbx::load_source` when importer-sensitive evidence matters. The
   immutable result binds the exact primary-file identity and bounded raw
   facts plus dependency-closure evidence to one normalized `Document`; borrow
   them through `document()`, `source_facts()`, and `dependency_closure()`.
   Calling `into_document()` explicitly discards both sidecars. Byte-owning
   hosts with sibling resources use the format crate's explicit-resource-root
   byte API. The compatibility `load_source_bytes`/`load_bytes` entry points do
   not fall back to the process working directory. The `load` and `load_bytes`
   entry points still return only a `Document`.

   glTF animation values remain authored; FBX scenes are normalized to metres,
   right-handed +Y-up coordinates and baked into linear TRS tracks. Raw FBX
   facts describe parser-projected source declarations and the AnimSmith
   loader's treatment, not authored key/tangent/interpolation values reconstructed
   from those baked tracks. Structural failures are loader errors. Semantic
   defects load and become findings. The same document carries meshes, skins,
   factor-only materials, and available base-color and normal textures for
   scene round-trips.

   Raw-fact row sets have explicit complete, partial, or unavailable coverage;
   a partial prefix proves presence only. Resource rows inventory bounded
   declarations. The separate closure view maps them to the primary identity,
   one-read identities for safe rooted sidecars, or typed refusal/unavailability.
   Each row also exposes its kind-derived loader-essential, nonessential, or
   target-only purpose; that classification is not an engine-import verdict.
   Only complete declaration coverage with an identity for every row yields a
   complete closure identity. The result is exact for that versioned resource
   domain, not a claim that an unsupported extension or unmodelled FBX domain
   cannot introduce another target-importer dependency. `SourceInfo.path` is
   diagnostic metadata, not identity or raw-fact authority.

   External capture is local and bounded: unsafe, absolute, remote, escaping,
   oversized, and symlink-mediated locators are not opened or reproduced.
   Aliases of one normalized source-relative key are read and hashed once;
   equal bytes under different keys remain distinct dependencies. Path-based
   loading supplies the trusted root from the primary file. Hosts loading
   captured primary bytes must explicitly supply a trusted root to resolve
   sidecars; without one, a loader-essential resource is an error and an
   optional resource leaves conservative closure coverage.

   An FBX host that needs auditable scale-boundary facts uses
   `animsmith_fbx::load_scale_source` instead: the returned wrapper retains the
   document and `FbxScaleCapabilityInventory` from one parse. The inventory
   and normalized source-skeleton sidecar enable only the CLI's narrow
   rest/bind re-encode path when
   `rest_bind_capability_facts_for_source` accepts them; the source-aware form
   can distinguish known, non-scale-bearing custom properties, resource
   linkage, and shader/binding-table metadata from an unknown source element.
   It also admits a BindPose only when it covers every joint of each skin it
   touches and every exposed converted row is finite, unambiguous, and agrees
   with the cluster bind or node rest-world matrix the bridge consumes; a
   source with no Pose remains valid.
   The same-load source can also prove that enumerated payload omission,
   influence projection with complete effective coverage, triangulation, and
   exact-bit welding are conversion-fidelity facts rather than scale-bearing
   blockers. The inventory retains them, and unknown payload or missing
   effective geometry/influence coverage still fails closed.
   The inventory-only
   `rest_bind_capability_facts` remains conservative where its frozen aggregate
   cannot make that distinction. Whole-document FBX scaling remains refused.
   `Complete`
   means the documented ufbx projection covers every representable node/skin
   identity and joint slot; a missing cluster bone downgrades the sidecar to
   `Unavailable` instead of dropping that slot. Unreadable ordered bind
   declarations retain no finite prefix. The local rests and bind matrices are
   adjusted/compensated or derived target-
   coordinate values, not exact authored FBX members. The inventory explicitly
   records baked curves, rebuilt payloads, omitted authored face/edge members,
   uninstanced, non-polygon-only, or zero-face mesh definitions (including
   stable identities for zero-face omissions), and unavailable raw span
   proof. Stackless source curves are present but unsupported. Retained mesh
   definitions and source-skin attachments share the stable ufbx mesh identity
   even when an earlier source definition emits no normalized primitive. A
   shared FBX geometry remains one normalized definition with multiple compact
   node instances, so that stable identity stays unique in measurements.
2. **Resolve an optional engine profile.** Construct an
   `animsmith_engine::EngineDeclaration` with the exact family, profile
   revision, engine version, importer, and typed document/selector settings.
   `resolve_static` validates every declaration without file I/O; then call
   `StaticResolution::resolve_input` with the loader-owned `SourceFormatV1`
   and actual clip names. The result contains the immutable facts identity and
   fully materialized settings identity. Pass that result and the same
   `LoadedSource` to `project_prediction_provenance_v1`; do not reopen or
   reconstruct either input. There is no generic/fallback profile,
   caller-supplied source-unit override, or TOML type in this API. If the host
   has no engine contract, omit projection and retain `None`; core measurements
   and checks remain engine-neutral.
3. **Compose one check catalog.** Start with `animsmith_core::all_checks()` and
   append one borrowed
   `animsmith_engine::EngineAddressabilityCheck::new(&source,
   provenance.as_ref())?`. Pass `None` when no profile was resolved so the
   stable engine-owned record remains not applicable. Validate selection
   against the core ids plus `animsmith_engine::ENGINE_CHECK_IDS_V1` before
   asset I/O, then call `evaluate_checks` exactly once for the combined
   per-file catalog. The borrowed catalog intentionally cannot outlive its
   same-load `LoadedSource` and provenance. `measure_document` does not consume
   either and remains profile-neutral. V1 materializes at most 4,096 actual
   clip-setting rows; an N+1 document is a typed bounds error rather than a
   truncated prediction.
4. **Resolve rig roles.** Use `resolve_configured_roles` to apply the same
   named/auto profile plus inline-override policy as the CLI. Lower-level
   `detect_profile`, `profile::resolve_named`, and
   `ResolvedRoles::from_names` remain available when a host intentionally
   owns a different policy. Checks consume roles, never project-specific bone
   names.
5. **Build `Config`.** The CLI's TOML is only one constructor. Deserialize
   the types from your schema or build them programmatically. Deserialization
   validates numeric check tolerances and per-clip loop caps immediately.
   `MovementOwner` represents independent XZ, Y, and yaw intent in core;
   `Config::expectations_for` returns canonical effective fields after
   exact/glob layering, while `Config::loop_continuity_tolerances` resolves the
   four effective loop pose and seam-velocity caps through the same per-clip,
   check-setting, and built-in-default precedence used by the checks. The
   legacy `in_place` input maps only to XZ and is cleared from that effective
   result. A selector that declares both spellings is rejected by
   `Config::validate`.
   Programmatic callers must call `Config::validate` before passing a directly
   constructed config to measurement-only APIs; `evaluate_checks` performs the
   same validation before inspecting or running the check catalog and returns
   invalid configuration as a typed `EvaluationError::InvalidConfiguration`.
   `Config::runtime_nodes` is the shared attachment/socket/IK selector policy;
   the legacy `rest-world-scale` check field is normalized into it only when
   the shared field is absent. Supplying both is a typed conflict. This added
   public field is an intentional pre-1.0 struct-literal break: exhaustive
   literals must add `runtime_nodes` or use `..Config::default()`.
6. **Create one `MetricGrids`.** Share it by reference with
   `measure_document`, `CheckCtx::new`, `evaluate_checks`, and optional report
   rendering so each clip is sampled once. `animsmith_report::render` takes
   one `ReportInputs` carrying those grids, the roles, the evaluations and the
   same `Config` the checks ran under; the configuration is what lets it draw
   a declared gait group's members against each other, since a coherent group
   emits no finding naming them.
7. **Map results into the host.** `Finding` carries a stable check id,
   severity, optional clip/bone/time, measured and expected values, and a
   message. The host decides whether warnings fail its gate.

Call `evaluate_checks` with the full catalog and a `CheckSelection`. It
returns one `CheckEvaluation` per
catalog check, including disabled, unselected, not-applicable, partial, and
not-evaluated work. `CoverageGap::code` and `EvaluationScope::code` are the
machine fields; never reconstruct coverage by parsing a message. Content
findings are nested under their owning check and coverage gaps are never
encoded as findings.

Role resolution remains an explicit frontend step. Use
`resolve_configured_roles`, `CheckCtx`, and `Config::rig` rustdocs for the
exact profile, override, and unresolved-role contracts.

## Compose the outputs you need

An embedded gate does not need to reproduce every CLI output. It can emit
clip measurements, add static asset measurements, run findings, render HTML, or
combine those results with host-owned checks. Share the same `MetricGrids`
within the limits documented by its rustdoc so those consumers judge one
sampled representation.

The `MetricGrids`, `measure_document`, and `measure_assets` rustdocs own cache
thread-safety, sampling, static-domain scope, and identity details.
`Primitive::additional_influence_sets` and the corresponding mesh measurement
records expose secondary glTF skin-attribute presence only. Mesh records also
preserve whether either side was unpaired on an individual primitive; they do
not retain or evaluate secondary per-vertex payloads.

For glTF/GLB, asset measurements additionally provide source-order material,
texture, and image inventory records plus explicit
`material_resource_coverage`. Complete glTF/GLB coverage is scoped to the five
documented core bindings, in order: `base_color`, `normal`,
`metallic_roughness`, `occlusion`, and `emissive`. It does not cover
extension-defined texture slots. These are loader observations, not a portable
image-management API: a host must handle `"unavailable"` coverage from other
loaders and must not infer writer preservation, image acceptance, repair,
resizing, transcode, color-space policy, or engine-import behavior from them.
Use the typed image metadata to inspect declared MIME separately from the
byte-detected container and decoded color/channel data; unavailable images
carry a reason instead of decoded metadata.

When the host needs to exchange the same JSON as the CLI, construct
`MeasurementContract`, `MeasureFileReport`/`LintFileReport`, and
`MeasureEnvelope`/`LintEnvelope` from `animsmith-core::contract`. That module
owns both immutable URNs and derives the lint/measure summary from the supplied
records.
It also exposes the typed `MeasurementReportInput` subset and
`MeasurementReportFile` records for consumers that need to validate and recover
every file's exact display path and full measurement contract from a current
measure or lint report. The core boundary preserves file order and accepts
empty or multi-file reports; each consumer owns any cardinality rule for its
workflow. `MeasurementReportInput::file_count` lets an adapter retain the raw
record count before `into_files` consumes and validates the report, while
`MeasurementReportError::File` carries a typed `MeasurementFileError` and an
index (also available through `file_index`) without adding consumer-specific
prose.
The compiling example emits a full schema-valid lint envelope; embedded
producers do not need to copy private CLI structs or hard-code protocol
identities. Host-specific sidecars remain appropriate when CLI interoperability
is not needed.

## Gate and stability contracts

The CLI convention is a useful default for an embedded gate:

- no error findings and no required-unavailable prediction facets: success
  (warnings and ordinary coverage gaps may remain visible);
- any `Severity::Error`: content rejection;
- any `required_prediction_unavailable` facet: prediction-evidence rejection,
  regardless of severity or allow-list policy;
- loader/config/I/O error: operator failure, kept separate from findings.

Missing prerequisites are typed coverage gaps, and disabled/unselected checks
remain visible without executing. Severity overrides apply only to content
findings. Ordinary coverage gaps are nonblocking by default; the embedding host
owns any stricter required-check or release-lane policy. To match the CLI's
finding threshold, allow-list, and unsuppressible prediction policy, call
`animsmith_core::lint_requires_failure` on the completed evaluations rather
than deciding from findings alone.

Built-in scope and gap codes may be emitted only by the checks declared in the
core evidence-code authority; `CheckEvaluation::evaluated` returns a typed
error when a different check claims one. Embedded checks retain an open
vocabulary by using namespaced custom codes such as `acme:input_unavailable`.
The public built-in-code slices are available to consumers that need to
enumerate or allow-list animsmith-owned codes.

For the current pre-1.0 API, prefer the crate-root flow: loader → role
resolution → `Config` → `MetricGrids` → measurements/checks → findings. The
durable automation contracts are deliberately narrower than the pre-1.0 Rust
API:

- built-in check ids used by config and findings;
- CLI exit codes and the versioned
  [JSON envelope](output.md), when the host interoperates with the CLI.

The `animsmith-core` crate root owns the full
[API status](https://docs.rs/animsmith-core) contract. Rust symbols, model and
transform types, metric formulas, and diff thresholds may still be refined
before 1.0. Match `#[non_exhaustive]` result types with a fallback arm. The
`Check` trait supports experiments with custom checks, including opt-in checks
that override `enabled_by_default`; an explicit "note", "warn", or "error"
setting activates one. There is no stable plugin registry yet; wrapping
animsmith findings with host-owned checks keeps that boundary explicit.

## Migrating an existing pipeline

For a command-by-command migration plan, use the
[pipeline scenarios](pipeline-scenarios.md) for marketplace intake, mocap
cleanup, outsourced acceptance, CI gating, and raw/generated artifact
storage, then use the [cookbook](../examples/README.md) for exact commands.

For the library cutover itself:

1. Capture accepted measurements from the old pipeline as golden values.
2. Run the CLI and embedded path from the same `Config` until their findings
   agree.
3. Compare old and new measurement maps with `diff_measurements`; judge
   motion deltas rather than binary file differences.
4. Keep project-specific sidecars, hashes, provenance, and storage policy in
   the host pipeline.

## Foot-cycle map planning

`animsmith_core::plan_foot_cycle_parameterization_v1` is the pure planning
boundary for a manifest-declared in-place gait ring. A host supplies the strict
typed parameterization, its exact byte identity, the already validated
collection manifest and exact byte identity, plus canonical contact fragments
and independently measured typed Root/Hips in-place evidence bound to each
fragment's exact artifact, dependency closure, and selected collection take in declared
member/path order. The planner validates manifest clip witnesses,
the known stance-detector provenance and exact common detector threshold,
bounded aggregate canonical-fragment work, alternating bilateral support
topology, positional boundary correspondence, and every declared segment-slope bound.
The strict declaration also carries a required exact proof policy: finite
maximum gait-phase spread and contact-boundary phase error in `[0, 0.5]`, plus
a finite non-negative minimum left/right amplitude. The plan retains that
policy together with the identity of the exact parameterization bytes; it does
not obtain defaults from config or merge values across members.
It returns one exact `ContactTransformBindingV1` and duration-preserving
`ContactTransformOperationV1::TimeWarp` per member.

After the host has independently bound the selected loaded clip to that exact
member plan and bound every track bone index to its selected, validated
skeleton, `animsmith_core::time_warp_clip_v1` can build a pure cloned clip
candidate. The map is source-to-output: LINEAR authored keys move forward and
interior map knots are sampled with AnimSmith's normal runtime semantics; STEP
tracks move only their authored breakpoints. Unsafe nonconstant CUBICSPLINE
tracks and binary32 time collisions refuse the whole candidate. A successful
candidate still carries no emitted-artifact identity or reread proof.
`animsmith_core::FootCycleClipWarpKnotV1` and `time_warp_knots_v1` expose the
one rule that decides when a control point and an authored key are the same
emitted instant and what the resulting key carries, so an embedder proving a
candidate independently can ask that question instead of restating it.
`animsmith_core::preflight_time_warp_clip_v1` runs the same validation and
returns exact candidate name/track/key/value/storage-byte counts plus a
conservative V1 inspection-work charge without allocating the candidate,
allowing embedders to enforce their own checked aggregate batch budget before
retaining multiple outputs.

The planner does not open paths. The CLI crate has a private
adapter that validates the closed bounded TOML grammar, resolves the manifest
and parameterization path sets with symlink/canonical and
platform-observable hardlink alias refusal, validates the pure runtime-set
binding before host source access, reloads only its exact member-reachable
sources/configs, retains their normalized config semantics and byte identities
   for proof without rereading controls, derives the typed in-place root evidence,
cross-checks plans against exact selected takes, and prepares bounded clip
candidates. Root evidence retains signed endpoint X/Z and signed accumulated
yaw even though the unchanged in-place gate applies horizontal `hypot(X, Z)`
and absolute yaw. The prepared collection also retains exact aggregate source
   metric pose-cell and sample-evaluation totals for the shared
source/output proof budget. It also runs the closed operation-aware stance-detector extension
handler, but retains contact transformation as a typed continuation: only a
   serializer supplies a truthful fresh output artifact identity and dependency
   closure. The private in-memory continuation now serializes every candidate
   as a strict self-contained GLB, rereads the exact bytes, and independently
   proves the transformed clips and contacts as one all-or-nothing batch.
   The public CLI then bounded-serializes member and aggregate evidence and
   publishes the complete fixed generation through one no-replace directory
   rename. Embedders must not
treat a successful plan or prepared candidate as an output-artifact or
gameplay-acceptance proof. See the
[collection contract](collection-contracts.md#foot-cycle-parameterization-v1-18-planner-slice)
for the declaration and exact topology grammar.

   The format-edge candidate writer is also a private continuation, not a
producer: `animsmith_gltf::write::preflight_glb_bytes` records one exact
in-memory projection receipt and `write_glb_bytes` reruns it before retaining
bytes. `StrictFootCycleV1` is deliberately narrower than legacy conversion and
refuses data it cannot represent exactly enough for the foot-cycle candidate
   contract, including source scene collapse, incomplete skin bindings, primary
skin-slot reinterpretation, unsupported primitive/texture/material domains and
   non-empty complete source material-resource sidecars,
and unrepresentable complete source-node or source-skin facts. The receipt
binds emitted JSON/BIN projection bytes, not deliberately excluded source-only
evidence. It preserves no
source-only names, stable source indices, or image-inspection/provenance
   sidecars as output facts. An exact complete-empty material resource graph is
   admitted only when the writer-facing material list is also empty. The
   collection proof clones each prepared source document, replaces only its
   selected clip, preflights all candidate bytes and shared metric-grid work
   before writing, rereads the exact GLB bytes, requires a complete closure with
   no external resources (embedded-buffer references may target `Primary`),
   finishes the contact transform with the reread identities, and independently
   checks duration, map/contact correspondence, gait, signed root trajectory,
   and configured loop pose/velocity tolerances. The CLI-only transaction owns
   generation staging/publication; the format crate does not. Neither path
   claims game-engine acceptance.

## Scale plan and proof contracts

The [scale workflow](scale.md) is the operator-facing guide. Normative algebra,
tolerances, refusal boundaries, and ownership live in
[DESIGN.md Appendix D](../DESIGN.md#appendix-d--decision-record-skinned-restbind-scale-canonicalization);
the [output reference](output.md) owns the evidence wire contract.

For an embedded producer:

1. Preflight the exact source format with
   `animsmith_gltf::preflight_scale_source`, then project it with
   `animsmith_gltf::capability_facts` into `ScaleCapabilityFacts`.
2. Call `plan_scale` once with an explicit `ScaleOperation`. The opaque plan
   owns the operation parameters and a numeric-free structural ledger.
3. Apply that same plan to the exact source representation. For glTF/GLB,
   `rewrite_scale_plan` cross-checks raw hierarchy and container identities
   against the plan before changing JSON or accessor bytes.
4. Reload the emitted artifact, wrap the document with
   `ScaleCandidate::from_document`, and call `prove_scale` with the original
   source and same plan.
5. Run the format artifact proof as well. Core proof covers normalized semantic
   claims; artifact proof covers exact raw preservation, container structure,
   declared bounds, aliasing, deterministic bytes, and the actual write set.
6. Publish only the proved artifact and matching evidence as one coordinated
   pair, preserving the CLI's rollback and documented process-crash semantics.

Those producer steps apply directly to glTF/GLB. With the default FBX feature,
the narrow `rest-bind` producer first requires a complete normalized FBX
inventory, stages a private GLB, and then runs the same raw-GLB plan/rewrite/
proof transaction; `whole-document` remains unsupported for FBX. The FBX
inventory is not authority to claim raw FBX spans, object properties, curve
keys, material/texture assignments, or vertex identity were preserved. The
source-aware gate may admit bounded texture/video declarations and discarded
user-defined properties while retaining their raw facts and closure evidence;
all incomplete or unknown scale-bearing domains still refuse with the exact
failed fact or counter.

`ScaleCandidate` grants no authority: proof independently validates the
source, plan inventory, candidate structure, and numeric claims. A frontend
must not substitute a normalized writer for exact-source rewrite when the
operation promises preservation of unknown or format-only payload.

`ScalePlan::ledger()` exposes read-only field, payload-shape, source-topology,
and proof-claim views for adapters. These rows name structural ownership and
identity, not resolved multipliers or expected numeric values. Writer and proof
may share this numeric-free binding vocabulary, but each must derive its own
numeric expectations. Under unavailable source coverage, best-effort raw locals
remain compatibility output rather than replay identity.

Each `ScaleProof` claim is a read-only `ScaleProofResidual` carrying its
maximum and comparison count together. `evaluated()` distinguishes a measured
zero from an obligation that walked nothing. The fixed
`ScaleTolerancePolicy::APPENDIX_D_V6` identity also owns the sampled-work
budget; proof refuses over-budget input rather than proving a subset.

The glTF convenience functions `rewrite_linear_units` and
`rewrite_rest_bind` compile and delegate to the same plan-taking adapter.
Use the plan-taking boundary when the host already owns the plan. The format
frontend retains raw capability, hierarchy, component, type/count/range,
alias/overlap, and complement checks that core cannot express.

See the public rustdoc for exact selectors, row variants, errors, and proof
fields. Calibration populations and historical timing notes are intentionally
separate in [scale-calibration.md](scale-calibration.md).

## What the libraries do not own

- Parsing the host's contract files into `Config`.
- Hashing assets, tracking provenance, or deciding staleness.
- Choosing raw/generated artifact paths or retention policy.
- Artistic retargeting, contact cleanup, or motion editing.

Those decisions surround animsmith in the
[raw-to-game-ready pipeline](pipeline-scenarios.md); they are not API
responsibilities.
