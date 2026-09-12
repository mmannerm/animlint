# animsmith — design & requirements

Status: pre-1.0 publishing design. Intended to keep the public crate and
CLI surface aligned while the project is still willing to make breaking
changes.
Origin: extracted from a private game project's animation pipeline
(design session 2026-07-03); that project — "the incubating project"
below — is the first consumer, not the scope.

---

## 1. Mission & positioning

**animsmith is a linter for skeletal animation clips.** It answers the
question every game team answers by hand today: *does this clip have
game-engine-friendly characteristics?* — does the loop actually close, does
the walk cycle's declared speed match its root motion, do the feet slide
during stance, is the rig conformant, are the quaternions sane.

**The gap it fills.** Nothing open-source does game-semantics clip
validation:

- **Khronos glTF-Validator** checks *spec conformance* (accessor validity,
  NaN, quaternion norms at the container level) — it has no concept of a
  loop, a gait, or root motion.
- **ozz-animation** has a motion-extraction sample (a good root-motion
  measurement reference) but no lint pipeline.
- Academic metric code (foot-skate ratio, jitter, penetration) lives in
  ML-evaluation repos, not artist tools.
- Engine importers (Unreal Interchange + Data Validation, Unity
  AssetPostprocessor, Godot import sidecars) give teams a *place* to hang
  custom checks, but the checks themselves are always studio-custom and
  re-derived from scratch.

animsmith packages those checks as a standalone Rust library + CLI: glTF/GLB
native, FBX ingested via ufbx, engine-agnostic core, machine-readable
output, and a self-contained HTML report with a 3D preview.

**What it is not (scope guardrails):**

- **Not a spec validator.** Run glTF-Validator for container conformance;
  animsmith assumes a parseable file and judges its *content*.
- **Not an art exporter.** `convert` (FBX→glTF) exists so clips can enter
  the lint pipeline directly from a DCC export; it promises animation and
  skinning fidelity, not material/shading fidelity.
- **A transformer for pipeline-mechanical operations only** (scope
  widened 2026-07-03; see Appendix A). In scope: `fix` for lossless
  mechanical repairs (quaternion unit normalization and hemisphere
  normalization), frame-range
  slice/trim + hold-extend, fail-closed in-place gait-anchor rotation, opt-in
  pruning of provably constant multi-key tracks, and format conversion
  including a full mesh/skin FBX→glTF path (a maintained replacement
  for the archived FBX2glTF). Out of scope stays *artistic*
  transformation: retargeting, motion editing, procedural animation —
  that is DCC work. The rule of thumb: animsmith may rewrite a clip
  only in ways whose correctness its own checks can verify.
- **Gait anchoring is an explicitly in-place operation.** The core boundary
  requires a movement policy; the shipped CLI and assembly switches select
  only `InPlace`. Before any channel is cyclically reordered, AnimSmith samples
  the resolved Root role (falling back to Hips) and refuses missing/non-finite
  evidence, horizontal accumulation above 1 cm, or yaw accumulation above 1°.
  Every nonconstant channel the operation would rotate must contain exactly one
  key at each declared whole-frame sample over `[0, duration]`, at the exact
  representable f32 `key / fps` time and exact period endpoint. Sparse,
  differently framed, duplicate-time, or off-grid evidence refuses; duplicate
  `(bone, property)` channels, including constant channels, also refuse before
  sampling. A phase shift must bijectively permute authored values rather than
  synthesize values at omitted frames. The
  verifier samples those exact admitted f32 key times, and mutation is an
  integer-index permutation rather than a second floating-point resample.
  Constant channels need no grid because cyclic reordering cannot change them;
  their key times cannot influence the declared period or shift.
  Before allocating a pose grid, the public core boundary validates all track
  cardinalities, targets, finite values, all resolved metric-role indices, and
  the complete acyclic parents-before-children skeleton. Both declared-frame
  and maximum-authored-key work must independently satisfy the inclusive bounds
  `declared frames × skeleton bones <= 1,000,000`, `declared frames × tracks <=
  1,000,000`, and `maximum authored keys × skeleton bones <= 1,000,000`; every
  verifier and metric sample uses the one bounded declared grid. The 1 cm endpoint-displacement and
  1° accumulated-yaw caps are applied directly, without any sampled-step
  allowance an interior outlier could inflate. Yaw is derived from model-space
  f32 quaternions as binary64 headings. At sample zero, the verifier selects
  the local `+Z`, `+Y`, or `+X` basis axis with the greatest finite horizontal
  projection, breaking exact ties in that order, and retains that one axis for
  the entire proof. This admits equivalent source-axis conventions without
  switching witnesses later to hide yaw; a selected axis that later loses its
  horizontal projection refuses. Full-turn crossings are counted and the first
  heading is subtracted from the final unwrapped heading, so the result has no
  segment-count-dependent summation error. Only four f32 successors at each
  inclusive cap cover authored endpoint translation/quaternion quantization;
  this gait-local rule does not widen other checks. Standalone transform output
  is transactional too: per-clip success lines and the write summary remain
  buffered until every selected clip and the artifact write succeed. This
  admits tightly closed cyclic pelvis/root sway while refusing authored travel
  and turns. Root-motion phase offsets,
  root-motion extraction, and trajectory-preserving cyclic rebasing remain
  runtime or separately designed operations; gait anchoring must not improvise
  them.
- **Not a runtime.** It models how engines sample animation; it does not
  play games.

## 2. Users & use cases

1. **Artist inner loop** — `animsmith lint export.fbx` seconds after a DCC
   export, before any engine import or bake. Catches "the loop pops",
   "wrong rig", "cm instead of m" while the DCC session is still open.
   This is the highest-value loop: the alternative is discovering the
   problem after the slowest step of the pipeline.
2. **CI gate** — `animsmith lint --format json` in CI on committed assets;
   stable JSON schema, exit codes, per-check severity config, baseline
   file for adopting teams with a dirty back catalog.
3. **Pipeline library** — engine pipelines embed `animsmith-core` and build
   check sets programmatically. First consumer: the incubating
   project's asset gate replaces ~1000 LOC of measurement Python with
   library calls.
4. **PR-review artifact** — `animsmith report clip.glb -o report.html`
   produces a single offline HTML file with 3D skeleton playback and
   metric charts; attach it to a PR or CI artifacts so a reviewer can *see*
   the seam pop the numbers describe.

## 3. CLI surface

```
animsmith lint    <file...> [--config animsmith.toml] [--select ids] [--deny warn] [--format text|json]
animsmith measure <file...> --format json          # measurements only, no judgment
animsmith inspect <file>                           # clips, durations, tracks, bones, detected rig profile
animsmith report  <file> -o report.html [--clip name]
animsmith report  <before> -o comparison.html --compare-after <after> --before-clip <name> --after-clip <name>
animsmith transform <file> -o <out.glb> [--clip name] [--slice START:END] [--hold-extend SECONDS] [--gait-anchor] [--drop-duplicate-loop-endpoint] [--prune-constant-tracks]
animsmith fix     <file> (-o <out.glb>|--in-place|--dry-run) [--repair id[,id]]
animsmith convert <in.fbx|in.glb|in.gltf> -o <out.glb> [--material-texture-recipe recipe.toml] [--animation-only|--bake-static-mesh-transforms] [--format text|json]
animsmith assemble <recipe.toml> -o <out.glb> --evidence <out.json>
animsmith diff    <A> <B> [--format text|json]     # A/B: assets or one-file output-v13 measure/lint JSON
```

- `lint` = measure + judge against config. `measure` is lint minus
  judgment — both emit the independently versioned measurement contract
  that other pipelines can pin.
- **Exit codes**: `0` no failing findings and no required-unavailable engine
  prediction facets (warnings, notes, and ordinary nonblocking coverage gaps
  may remain), `1` at least one error-severity finding, any
  `required_prediction_unavailable` facet, pending repairs under
  `fix --dry-run`, or any producer refusal established as a fact about source
  asset bytes, `2` operator/tool error (unreadable declared file, bad config or
  recipe syntax, unsafe/unwritable path, or publication failure).
  `--deny-warnings` promotes warnings to errors; severity and `--allow` do not
  suppress an emitted required-unavailable facet.
- **Producer outcome authority is typed before prose exists.** `convert` and
  `assemble` carry `Published`, `Rejected`, and `Operator` through one shared
  boundary. Loader, selector, transform, proof, and representability failures
  reached after source bytes were read are asset refusals; invocation,
  recipe-validation, path, file-read, temporary-file, serialization, and
  publication failures are operator errors. Mixed error enums (notably the
  material-texture recipe) are matched by variant at the call site. No exit
  decision parses an error string. JSON refusals use the separately immutable
  `urn:animsmith:schema:producer-refusal:1` record; existing conversion,
  assembly, output, and scale evidence identities remain unchanged. Text
  refusals are one escaped stderr diagnostic carrying the same stable kind.
  Every refusal precedes publication, preserving a prior assembly pair and a
  prior convert artifact; this does not claim that convert's direct writer can
  roll back an operator I/O failure.
- **Stdout is reporting, not outcome authority.** Every JSON, text, and
  Markdown stdout write, including parser-rendered help and version text, uses
  a fallible checked write followed by a best-effort checked stderr diagnosis.
  A closed pipe or other write failure never panics or replaces the outcome
  already established by a stdout-bearing path: shipped success paths retain
  `0`, while finding and refusal paths retain `1`. Operator errors occur
  before stdout reporting, remain stderr-only, and retain `2`.
  This deliberately includes output-centric commands such as `inspect`: exit
  `0` states that inspection completed, not that an external consumer accepted
  every byte. Assigning a new outcome to text alone would make exit semantics
  depend on presentation format and would relabel real findings or completed
  publication as operator errors. JSON serialization failure remains an
  operator error because it means no truthful record could be formed; failure
  to deliver already-rendered bytes is only a reporting failure.
  Multi-part human output is attempted as one checked stream where one command
  can produce multiple records; in particular, all selected `fix` repair
  reports and all parts of one conversion summary yield at most one
  stdout-failure diagnostic.
  Parser output remains clap-owned: the checked boundary calls clap's
  fallible writer directly so its configured automatic/forced ANSI styling is
  preserved instead of being stripped while materializing a plain string.
- Human-readable command results are assembled by pure functions in the CLI's
  renderer module. Command dispatch keeps execution, file writes, and exit
  policy in `main.rs` and passes structured values to the renderer; the
  renderer is the single owner of escaping untrusted text for terminal-safe
  presentation. JSON result serialization remains a separate, unchanged path.
- `transform --prune-constant-tracks` is an opt-in, candidate-first mechanical
  edit: it shares the `constant-track` tolerances (vector components `1e-4`,
  sign-invariant rotations `1e-3` radians), verifies the resulting sampled
  local TRS and model-space position/rotation, and reports every original track
  index it removed or retained.
  It runs after selected transforms and never substitutes for DCC curve cleanup
  or key reduction. Per-clip `animates_bones` names protect motion evidence;
  `[rig] required_bones` remains a skeleton-presence-only contract.
- Inputs: `.glb`, `.gltf` (+ external buffers), `.fbx` (via the `fbx`
  feature, default-on in the released binary).
- **Malformation policy**: *structural* malformation — keyframe/value
  count mismatch, zero-key channels, absolute or escaping external
  buffer URIs, non-forest node graphs (cycles or a node with two
  parents), or a primitive accessor whose dense or sparse byte extent its
  declared buffer view or resolved buffer bytes cannot satisfy, an integer
  `TEXCOORD_0` or `WEIGHTS_0` accessor that omits `normalized: true`, or an
  animation sampler accessor whose declared element does not match its
  property-selected reader — is rejected
  at load. For `convert` and `assemble`, a rejection derived from readable
  source bytes is an asset refusal (exit 1); an unreadable primary file or a
  missing/unreadable declared external resource is still an operator error
  (exit 2). Commands outside that producer boundary retain their historical
  operator-error load policy. Run glTF-Validator for structural details.
  Recovering a non-forest graph would force an arbitrary parent choice or
  silently drop a cyclic subtree, so the loader rejects rather than repairs
  (decision recorded for #92). Likewise, substituting an empty position/index
  vector for a short accessor would make checks report on geometry the file did
  not author. Primitive shortfalls therefore use a located
  `PrimitiveAccessorLayout` refusal; inverse-bind shortfalls remain typed
  source evidence because that domain already models unavailable bind
  declarations (decision recorded for #329).
  Sampler input is `SCALAR`/`FLOAT`; translation and scale output are
  `VEC3`/`FLOAT`; rotation output retains all five glTF quaternion encodings
  (`BYTE`, `UNSIGNED_BYTE`, `SHORT`, `UNSIGNED_SHORT`, `FLOAT`) as `VEC4`,
  and morph-weight output retains those same component encodings as `SCALAR`.
  Declared lookalikes are never reinterpreted merely because their byte size
  happens to match the reader's element (decision recorded for #327).
  The same fail-closed rule keeps invariant-9 honest for integer UVs and
  weights: normalized `UNSIGNED_BYTE`/`UNSIGNED_SHORT` values are decoded to
  their declared float range, and `FLOAT` values pass through, but an
  unnormalized integer accessor is refused before decoding. The `gltf` reader
  rescales those integers whether or not the flag is set, and the loader has
  no finding channel in which to disclose that unauthorized reinterpretation;
  allowing it would make `measure` report derived values as authored data
  (decision recorded for #328).
  *Semantic* defects — NaN times or values, non-unit quaternions,
  hemisphere flips, seam pops — load fine and are judged by the checks;
  sampling is panic-free under them by construction.
- `fix` intentionally requires either `-o/--output` or `--in-place` for
  writes; `--dry-run` is the check mode — it inspects only and exits `1`
  when repairs are pending, mirroring `lint`. Repairs are addressed by
  stable ids; every repair must be safe, lossless, and idempotent.
  Repair taxonomy (risk-tier groups) is deliberately deferred until a
  repair exists that doesn't meet that bar.
- **`fix` stays byte-surgical — a product requirement, not an accident**
  (decision recorded for #33). It patches only the offending animation
  bytes in the original container and copies everything else through
  verbatim, so meshes, skins, materials, and textures survive a repair
  bit-for-bit. Folding hemisphere/norm repair into a core `transform`
  and re-emitting through the unified `Document` writer was considered
  and rejected: the model writer re-emits and reorders accessors, so it
  is not byte-identical and would rewrite bytes `fix` must leave
  untouched. The `Document` round-trip is the right tool for
  `convert`/`transform`; in-place `fix` is not a round-trip.
- `convert` and `assemble` are compiled only with the `fbx` feature. `--no-default-features`
  remains a glTF-only pure-Rust CLI with validation, transform, fix, and
  diff commands intact; `report` is controlled separately by the
  `report` feature.
- `convert --bake-static-mesh-transforms` is an opt-in static geometry
  operation. It accumulates rest hierarchy transforms into positions and
  inverse-transpose normalized normals, then writes canonical identity-root
  geometry. It retains topology, UVs, and the model-supported material and
  embedded base-color, normal, metallic-roughness, and occlusion-texture data. It fails closed for any
  animation track, skin signal, uninstanced or shared mesh definition,
  malformed/non-finite data, singular or near-singular transform, or
  reflection; it neither bakes skinning nor guesses animated or reflected
  semantics. It conflicts with `--animation-only`, leaves default conversion
  unchanged, and is deterministic for repeated same-platform input/options.

- `convert --material-texture-recipe` supplies exact named BaseColor, normal,
  metallic-roughness, and occlusion mappings for a conversion. It conflicts with `--animation-only`, is
  compatible with static baking, and leaves the ordinary linked/embedded
  texture path untouched when omitted. Its versioned recipe and provenance
  contracts pin path containment, image processing, and encoder behavior.

- `assemble` is the versioned generic boundary for a skinned base plus clips
  spread across separate files or master timelines. It permits only explicit,
  mechanically verifiable operations: exact-name skeleton remap, exact mesh
  selection, clip slicing/endpoint/hold/gait operations, named channel removal,
  rest-track completion, quaternion cleanup, opt-in constant-track pruning
  after all other transforms while protecting effective-clip `animates_bones`
  names, fail-closed exact-name node-subtree removal after animation transforms,
  bind-consistent skin canonicalization, material recipes, and deterministic
  GLB/evidence emission. Node-subtree removal refuses any surviving track,
  mesh-instance, skin-joint, or complete source-skin reference; it remaps the
  surviving hierarchy but does not reparent descendants or garbage-collect
  mesh, material, or texture resources.
  Character-assembly recipe/evidence v3 introduced that node-removal contract;
  v4 retains it unchanged while adding optional rest/bind scale integration.
  Selectors resolve by exact unique name against the post-canonicalization base;
  their descendant union is planned before completion, excluded from completion
  targets, and projected after clip processing and constant-track pruning.
  `animates_bones` may therefore retain a protected track and force refusal,
  while `[rig] required_bones` remains lint presence policy rather than an
  assembly selector. Accepted non-empty projection clears optional source-native
  skeleton identity, because it cannot remain a complete account after authored
  deletion. Because mesh-bearing nodes are refused, removal cannot newly orphan
  asset resources; only the earlier explicit mesh-instance selection performs
  resource pruning. Evidence lists every removed node once in original
  parent-before-child order, distinguishes direct selectors from descendants,
  and retains original node and parent indices.
  Archive extraction, gameplay naming and acceptance policy, cache/generation
  policy, and publication remain with the consumer.

## 4. Repository & crate layout

One public repo, one cargo workspace, five published crates (plus one
`publish = false` dev crate, `animsmith-testkit`). The split is driven by
two hard constraints: the core must be consumable with zero C compilation
and minimal deps; FBX support pulls in a C build step most library
consumers must never pay for.

```
animsmith/
├── Cargo.toml                  # workspace, edition 2024
├── LICENSE-MIT / LICENSE-APACHE
├── THIRD-PARTY.md              # ufbx (MIT OR PDDL-1.0), vendored viewer assets
├── crates/
│   ├── animsmith-core/          # data model, sampling/FK, metrics, diffs, checks, config, findings
│   ├── animsmith-gltf/          # glTF/GLB → core model; GLB writer for `convert`
│   ├── animsmith-fbx/           # ufbx wrapper → core model; isolates the C build
│   ├── animsmith-report/        # self-contained HTML report generation
│   ├── animsmith/               # CLI binary (features: fbx, report — default on)
│   └── animsmith-testkit/       # publish=false: fixture builders, documentation-source readers, and CLI process helpers shared by tests + the example generators
├── assets/viewer/              # viewer JS/CSS, inlined via include_str!
├── fuzz/                       # cargo-fuzz targets for the untrusted-input loaders
└── testdata/                   # CC0 rigs + procedurally corrupted fixtures
```

- **animsmith-core**: deps `glam` (the de-facto Rust game-math crate — do
  not hand-roll mat4/quat as the Python did), `serde`, `serde_json`,
  `serde_jcs` for strict bounded core-owned RFC 8785 canonical JSON contracts,
  `sha2`, and `thiserror`. Its JCS use remains engine-, format-, filesystem-,
  and network-agnostic. It owns strict versioned JSON contract validation but
  no file-format knowledge or filesystem/network I/O. This is what embedding
  pipelines link.
- **animsmith-gltf**: the `gltf` crate with trimmed features (no image
  decoding); owns GLB emission via `gltf-json`.
- **animsmith-fbx**: `ufbx` (official bindings, v0.11.x, actively
  maintained; bundles the single-file C library via `cc` — no system
  deps). A separate crate rather than a feature flag so the C toolchain
  requirement is structurally isolated.
- **animsmith (CLI)**: `clap`, `serde_json`, `toml`. `--no-default-features`
  yields a pure-Rust glTF-only build.
- Toolchain: stable Rust, edition 2024, MSRV pinned in CI. License:
  MIT OR Apache-2.0. All crate names verified free on crates.io
  (2026-07-03).
- **fuzz/**: a nightly-only cargo-fuzz workspace (detached from the main
  workspace) with libFuzzer targets for the three entry points that ingest
  untrusted files — `animsmith_gltf::load`, `FixSession::apply_to_path(..., Repair::QuatFlip)`,
  and `animsmith_fbx::load`. These are the executable check on invariant-1
  ("untrusted input must never panic or OOM"): targets run in release mode
  to match the shipped CLI's panic semantics, with AddressSanitizer on. A
  weekly CI job (`fuzz.yml`) runs each target for 60s off a checked-in seed
  corpus; minimized crashers are committed under `fuzz/seeds/` as regression
  fixtures, each also pinned by a unit test in `animsmith-gltf`'s hardening
  suite. Continuous/long-running fuzzing (OSS-Fuzz) is deferred.

## 5. Core data model

Two representations of a loaded file, because checks genuinely need both:

**Raw layer** — what the file says. The document keeps the skeleton hierarchy
and rest pose, authored clips and tracks, optional scene assets, and source
metadata together. Mechanical checks use this layer for defects such as NaNs,
quaternion flips, key density, and constant tracks. Exact Rust types and fields
belong to the `animsmith-core` model rustdocs; build them with `just doc` or use
the package README's stable docs.rs link.

`assets` (meshes, skins, PBR materials, and embedded base-color, normal,
metallic-roughness, and occlusion textures) is the scene-asset half of
the document. Both the FBX and glTF loaders populate it from a single
`load` (there is no separate assets-carrying entry point — the two
loaders share one shape); it is empty when the input carries no scene
assets. The check catalog ignores it — checks judge animation — but it
rides the one `load`/`write` round-trip, so `transform` and `convert`
preserve geometry rather than silently dropping it, and `measure`
reports versioned static scene measurements from it (#16): geometry-definition
boxes in primitive coordinates, node-instance boxes in default/rest node-world
coordinates, and per-scene unions. These are not runtime-visible bounds:
animation, skin deformation, morph deformation, and runtime placement are
excluded. The output contract records unavailable static node bounds and scene
partial coverage rather than serializing non-finite values.

The glTF loader also retains metadata-only presence for secondary
`JOINTS_n`/`WEIGHTS_n` attributes. The measurement contract aggregates those
independent sides per mesh definition so unsupported or unpaired sets are
observable without changing the primary four-influence semantics. Secondary
per-vertex payloads, repair, and writer preservation remain outside the core
skinning model.

The scene-asset model keeps source mesh definitions separate from their
mesh-bearing node instances and retains declared scene roots plus the optional
default-scene index. That identity is measurement evidence: duplicate names do
not collapse, shared definitions are reported once, and nodes outside every
declared scene remain observable. The current glTF writer still normalizes an
emitted document to its existing single generated scene; preserving authored
multi-scene membership across `transform`/`convert` is not part of the static
measurement contract.

`validate_document_shape` is the model-owned snapshot check for the structural
invariants full-shape strict operations such as scale share: finite
parent-before-child rest topology, source identity/projection topology,
clip/track storage, and scene instance/inverse-bind references. `Document`
remains publicly mutable, so a
successful check is not a durable token; strict operations revalidate every
input at every public boundary. This is deliberately not universal model
validity. Measurement can retain finite instance evidence from hierarchy
branches unrelated to a malformed chain, sampling remains panic-free and
tolerant of hostile standalone tracks, and static baking validates only the
fields its static operation consumes. Scale's
negative-weight, finite-position, capability, closure, affine, parity, and
proof rules remain operation-specific.

Ingestion is **triangle-list only** — the target inputs are skinned game
rigs, and the model and writer carry no primitive-topology field. A
non-`TRIANGLES` glTF primitive (points, lines, strips, fans) is skipped
rather than misread as a triangle list; other topologies and their
retriangulation are out of scope for now.

**Sampled layer** — what a game runtime sees. A `PoseGrid` built by a
`ClipSampler`: uniform time grid over `[0, duration]` (resolution = max
channel key count, or explicit fps), glTF-spec interpolation semantics
(lerp for T/S, shortest-path slerp for R with negation on `dot < 0`, STEP
hold, cubic-spline Hermite), clamp at ends. For clips declared `loop`,
the wrap pair is `(last frame, frame 0)` — the seam definition every loop
check shares. FK accumulates every skeleton node's local TRS to model
space, including each root node's own transform. Metrics that need a
body-relative frame derive it from resolved roles such as hips and feet.
The metric grid is computed once per clip and shared across checks,
measurements, and reports through the lazy `MetricGrids` owner.

**Rig profiles** — checks reference semantic roles rather than bone names.
Profiles bind those roles to source-rig names; the exact public role set,
binding types, and matcher fallback belong to the `animsmith-core` profile
rustdocs.

Built-in profiles ship for `mixamo` (`mixamorig:Hips`…), `ue-mannequin`
(`pelvis`, `foot_l`…), and `humanoid` (`humanoid_ Pelvis`,
`humanoid_ L Foot`…), plus **auto-detection** that scores every profile by
resolved-role coverage and reports the winner in `inspect`. A check whose
required roles don't resolve records a typed coverage gap — never a false
finding. This is the single design rule that makes the tool useful outside
its birthplace: tolerance data and bone names are config; the math is not.

The runner owns selection, activation, and severity policy. Each check exposes
a cheap applicability predicate and one evaluation method. The evaluation
returns content findings separately from completed scopes and typed coverage
gaps. Severity overrides therefore cannot turn missing evidence into a false
error. `gait-group`, for example, can complete member-existence validation
while reporting phase coherence as a gap. `severity = "off"` disables a check
without hiding its applicability record. A check may also declare an opt-in
default; assigning "note", "warn", or "error" enables it while retaining the
same independent configuration record.

The built-in evidence-code declarations are the single authority for each
scope or gap code's machine identity, meaning, and allowed emitting check ids.
The evaluation boundary rejects a built-in code emitted by an undeclared
check, and the output-reference gate derives its exact inventory, meanings,
and emitters from those same declarations. Embedded checks remain open-ended:
they use namespaced custom codes rather than extending a closed enum.

**Checks** implement one trait and emit structured findings:

```rust
pub trait Check {
    fn id(&self) -> &'static str;              // "loop-seam", "quat-flip", …
    fn enabled_by_default(&self) -> bool;       // true unless policy is opt-in
    fn applicability(&self, ctx: &CheckCtx) -> Applicability;
    fn evaluate(&self, ctx: &CheckCtx) -> CheckOutput;
}
pub struct Finding {
    pub check_id: &'static str, pub severity: Severity,   // Note | Warning | Error
    pub clip: Option<String>, pub bone: Option<String>, pub time_s: Option<f32>,
    pub measured: Option<Value>, pub expected: Option<Value>,
    pub message: String,
}
```

The structured fields (not just a message string) are what make `diff`,
the JSON schema, and the HTML report cheap.

## 6. Check catalog

Tiers are shipping priority. "Prior art" = a proven implementation exists
in the incubating project's pipeline to port, with real-world numbers to
golden-test against.

### P0 — mechanical + the two killer semantic checks (v0 core)

| id | what it checks | needs | config | prior art |
|---|---|---|---|---|
| `nan` | NaN/Inf in key times or values | raw | — | new (trivial) |
| `time-monotonic` | non-increasing/duplicate key times; first key ≫ 0 | raw | epsilon | new |
| `quat-norm` | rotation keys with \|q\|−1 beyond tolerance | raw | eps (1e-3) | new |
| `quat-flip` | adjacent keys with `dot < 0` (long-way slerp in engines that don't neighborhood-correct) | raw | severity | new |
| `duration-sanity` | zero/degenerate duration; channels within one clip ending at different times; frame count non-integral at declared fps | raw + meta | expected fps list, pinned duration | reference contract `duration_s` pin |
| `scale-keys` | interpolation-aware temporal scale variation beyond tolerance | raw trajectory | severity | new |
| `non-uniform-scale` | component inequality anywhere on the interpolation-aware scale trajectory | raw trajectory | severity | new |
| `constant-nonunit-scale` | constant non-unit scale channel or single-key pin | raw trajectory | opt-in severity (off by default) | new |
| `constant-track` | redundant multi-key representation whose interpolation-aware trajectory never moves | raw trajectory | eps | new |
| `frozen-bone` | required bone's max angular deviation from first frame below floor | grid + roles/meta | `min_rotation_deg` | reference contract rotation floor + measured rotation ranges |
| `duplicate-loop-endpoint` | strict authored-key duplicate closing endpoint in a declared loop; warning/default-on | common finite strictly increasing timeline, valid cardinality, matching endpoints, interior motion + loop declaration | — | mechanical subset of #22's endpoint classifier plus open-cycle transform |
| `loop-closure` | maximum per-bone model-space last→first position and shortest-path rotation delta | grid + loop declaration | global `max_position_delta_m` / `max_rotation_delta_deg`; clip `max_loop_position_delta_m` / `max_loop_rotation_delta_deg` | consumer-neutral authoring-loop requirement |
| `loop-seam` | last→first position wrap discontinuity of feet-relative-to-hips, normalized by the *local neighbour* per-frame step, with a stride floor so stationary clips skip | grid + Hips/feet/toe roles | `max_ratio`, `min_stride_step_m` | `locomotion_metrics.py` — port verbatim |
| `loop-seam-vel` | maximum per-bone model-space difference between the in-clip velocities entering and leaving the wrap | grid + loop declaration | global `max_velocity_delta_mps`; clip `max_loop_velocity_delta_mps` | consumer-neutral authoring-loop requirement |
| `loop-seam-rot` | maximum per-bone shortest-path model-space angular-velocity difference between the in-clip steps entering and leaving the wrap | grid + loop declaration | `max_angular_velocity_delta_degps` | consumer-neutral authoring-loop requirement |
| `root-motion-speed` | horizontal root/hips displacement ÷ duration vs declared `speed_mps`; flags stray speed pins on non-locomotion clips | grid + Root/Hips | pinned speed + tolerance (reference gate: 15%) | reference bake |
| `missing-bones` | declared-required animated bones absent; tracks targeting nodes outside the skeleton | raw + meta | bone/role list | reference contract `animates_bones` |
| `required-bones` | declared required rig bones absent from the skeleton; static sockets and IK targets are valid | skeleton + rig config | `[rig] required_bones` | consumer-neutral structural rig contract |
| `naming` | clip names vs convention pattern | meta | regex/glob | new |
| `units-sanity` | hips rest height wildly outside human scale (the cm-vs-m export classic) | skeleton + profile | height band | new |

### P1 — locomotion semantics (the first-of-kind tier)

| id | what it checks | prior art |
|---|---|---|
| `gait-phase` / `gait-group` | stride-phase anchor from the fundamental-harmonic trough of the left-minus-right foot-height signal; an exactly zero-amplitude signal has no phase subject, while positive signals are governed by the configured inclusive `lr_amplitude` confidence floor; circular phase spread across a declared clip ring (directional-blend coherence) | reference metrics module + gait-group contract — port verbatim |
| `in-place` | classify in-place vs root-motion (net + per-frame root displacement) and compare against the clip's declared expectation | new; trivial on the grid |
| `foot-slide` | detect stance (foot height + near-zero vertical velocity), measure horizontal foot velocity during stance in the travel-cancelled frame | new; hardest check — ships opt-in until corpus-tuned |
| `bind-pose` | rest pose vs first frame delta (clip authored against wrong bind); T-pose/A-pose classification; node-TRS rest disagreeing with IBM-derived rest (the disagreement is itself a finding) | reference sidecar already derives rest from IBMs |
| `axis-conventions` | character forward/up at rest vs declared axes; root orientation drift over a loop | reference contract axis vocabulary |
| `key-density` | keys/sec far above the clip fps (unbaked-curve bloat) or far below (starved track) | new |

### P2 — corpus/cross-clip

Cross-clip skeleton & rest-pose consistency across a directory;
ground-penetration of feet/toes; mirrored-pair symmetry (`walk_left` vs
`walk_right`); additive-clip suitability; compression-noise metrics
(per-track jerk); morph-target weight ranges; SARIF output.

## 7. Configuration

TOML (`animsmith.toml`, or `--config`); Rust-ecosystem norm and
diff-friendly in asset repos:

```toml
[rig]
profile = "mixamo"                 # or "auto", or an inline role map:
# A presence-only structural contract; these do not need animation tracks.
required_bones = ["root", "weapon_socket", "ik_hand_l"]
# [rig.roles]
# hips = "humanoid_ Pelvis"
# left_foot = "humanoid_ L Foot"

[checks.loop-seam]
severity = "error"                 # off | note | warn | error
max_ratio = 1.5

[checks.loop-closure]
max_position_delta_m = 0.01
max_rotation_delta_deg = 1.0

[checks.loop-seam-vel]
max_velocity_delta_mps = 0.1

[checks.loop-seam-rot]
max_angular_velocity_delta_degps = 5.0

[checks.quat-flip]
severity = "warn"

[clips."run_*"]                    # glob; exact > glob, later entries win ties
loop = true
movement_owner_xz = "animation"
movement_owner_y = "gameplay"
movement_owner_yaw = "animation"
max_loop_position_delta_m = 0.04   # finite per-family override; global is fallback
max_loop_rotation_delta_deg = 2.0
max_loop_velocity_delta_mps = 0.2
speed_mps = { value = 3.1, tolerance = 0.25 }
fps = 30

[clips.run_forward]                # exact fields overlay matching globs
max_loop_rotation_delta_deg = 0.5

[gait_groups.run-ring]
clips = ["run_forward", "run_back", "run_left", "run_right"]
max_gait_phase_spread = 0.08
min_lr_amplitude_m = 0.05
```

CLI flags override file config (`--select`, `--allow`, `--deny`).

**Engine-agnosticism rule:** the TOML file is merely *one* constructor of
a `CheckSet`. Embedding pipelines build check sets
programmatically through the library API and keep their own contract
formats, hashing, and tolerance semantics on their side. animsmith never
learns an embedder's contract schema.

## 8. Output formats

- **Text** (default): findings grouped per clip, measured-vs-expected on
  one line, colored; `--quiet` for CI summaries.
- **JSON** (`--format json`): current output v19, identified by
  `urn:animsmith:schema:output:19`. Lint emits one result per catalog check and
  represents selection, configuration, applicability, evaluation coverage,
  content findings, completed scopes, and typed gaps independently. Measure
  and lint share a nested, independently versioned measurement contract. The
  current contract inventories glTF material definitions, the five core
  `base_color`, `normal`, `metallic_roughness`, `occlusion`, and `emissive`
  texture slots, texture-to-image identity, and decoded image metadata in
  source order. Complete coverage is scoped to that documented core domain;
  extension-defined texture slots are not implied. Measurements-v17 makes
  per-bone loop-continuity unavailability explicit without suppressing finite
  sibling rows. It retains measurements-v16's
  inventories each mesh's primitives in source order: nullable source material,
  total and finite decoded `POSITION` rows, and finite-only primitive AABB and
  centroid. Indexed geometry counts each stored position once rather than index
  references. Unsupported nonempty images may carry at most 16 bytes of
  lowercase `leading_magic_hex`; it is evidence only and never a guessed
  container format. The contract makes absent
  loader support explicit as unavailable coverage and separates a
  source-declared MIME type from byte-detected container and decoded image
  facts. This remains measurement evidence, not an image acceptance, repair,
  resize, transcode, color-space, writer-preservation, or recipe-authority
  policy.
  Measurements v15 adds the canonical set of surviving non-empty local TRS
  channels keyed by skeleton index, plus engine-neutral sampled Root/Hips
  trajectory evidence. Root is selected whenever resolved; Hips is only a
  fallback when Root is unresolved. Translation and yaw retain independent
  availability, with endpoint X/Z/Y displacement, sampled horizontal travel
  and vertical extrema, and fixed-basis net/unwrapped/travel yaw in normalized
  right-handed +Y-up metre space. These are shared uniform-grid regression
  observations, not continuous-curve extrema or transform-correctness proof.
  Output v10 added the registry-independent engine-prediction provenance and
  scoped-facet substrate described in Appendix E. Output v11 retained it and
  added per-role resolution outcome and match-policy provenance. Output v12
  introduced bounded V2 provenance/settings/prediction evidence and
  catalog-allocated facet summaries; output v17 retained that evidence while
  advancing its nested measurements to v16 and adding V6 root-motion
  provenance. Output v19 retains the immutable prediction graphs against a
  fail-closed measurements-v16 projection while publishing the complete
  measurements-v18 evidence. Output-v9, measurements-v15, output-v12, and
  collection-output-v4 remain immutable historical contracts.
  Measurements v14 gives every clip fact that is not applicable to every
  clip (loop continuity, loop endpoint mode, frame grid, loop seam ratio,
  gait and its phase, and root-motion speed) a required sibling
  `<field>_availability` status of `measured`, `not_applicable`, or
  `unavailable`, so a consumer can distinguish a legitimately missing
  subject from an applicable metric whose derivation failed and must fail
  closed. It also retains v13's exact raw inverse-bind matrices and affine
  and conditioning evidence before deriving their inverses, and v12's
  corrected linear-transform observations through shared f64 affine facts;
  v13 and earlier contracts remain immutable historical evidence.
  `convert --format json` instead emits the separately versioned
  `urn:animsmith:schema:conversion-evidence:2` producer-evidence contract:
  requested options, written-artifact counts, optional static mesh bake entries
  with source/output identities and applied world transforms, and deterministic
  material-texture recipe provenance when requested. v1 remains historical;
  the current CLI emits v2 exclusively.
  CLI exit status derives from content severity (warnings block only with
  `--deny-warnings`) plus required-unavailable engine-prediction facets;
  ordinary coverage gaps remain nonblocking evidence.
  The current output-v19 envelope types and immutable identities live in
  `animsmith-core` so CLI and embedded producers serialize the same reporting
  contract. Static-bake evidence is also a public core type; the conversion
  envelope remains a CLI producer contract.
- **Future serializers**: no game-industry standard exists for skeletal
  animation lint results. Keep native JSON as the source of truth, then
  add serializers where downstream tools expect them: SARIF for code
  scanning, GitLab Code Quality/CodeClimate for MR widgets, JUnit XML for
  CI test dashboards, and CSV/HTML for humans.
- **`diff A B`**: compares measurement maps per metric with fixed public
  per-metric significance thresholds, independent of configured lint caps;
  prints deltas; exits 1 on significant movement. Primary use: "did this
  DCC re-export change anything that matters?"

## 9. HTML report (the visual preview)

`animsmith report clip.glb -o report.html` → **one self-contained offline
HTML file** (CI-artifact- and PR-attachment-friendly; no CDN, no install).

The explicit comparison spelling is a separate presentation producer: it
preflights named/indexed skeleton correspondence, selected metric-grid work,
and input text before either side's checks run; writes atomically only to an
output distinct from both inputs; and uses one shared coordinate/path scale.
Both judged poses are drawn through that one camera: side by side by default,
and at the reader's option overlaid in a single pane with the after skeleton
dashed over the before one at the same two selected frames. The overlay is a
second reading of the embedded grids — no second copy of the pose data, no
refit, and no new authority.
Its phase synchronization labels both source times and never claims a retime.
Findings, coverage, prediction provenance, and immutable identities remain
side-specific, so a missing finding cannot read as acceptance.

**Key design choice: no three.js, no `<model-viewer>`.**

- `<model-viewer>` can play a GLB but exposes no skeleton/per-frame API —
  it cannot draw bone lines, foot trails, or sync to charts. Fails the
  requirement outright.
- three.js (~650KB inlined + GLTFLoader) works, but it would *re-sample
  the animation in JS*, and its loop/slerp behavior may disagree subtly
  with what the linter measured — the preview could contradict the
  findings it illustrates. It is also an update treadmill.
- The decisive observation: **the report never needs to sample animation
  in JS.** The Rust side already computed the `PoseGrid` — model-space
  joint positions for every frame the checks judged. Embed that.

So the viewer is a hand-written **WebGL2 skeleton renderer (~15KB)**:
bones as line segments, joint dots, root-motion and foot trails, orbit
camera, play/scrub transport. It renders exactly the frames the checks
measured — when `loop-seam` flags the wrap at frame N, the viewer scrubs
to *that* frame N. Determinism is the feature.

- **Embedded data**: pose grids as base64 Float32Array in
  `<script type="application/json">` blocks (~60 bones × 3 floats × 300
  frames ≈ 290KB base64 per clip; f16 quantization is the escape hatch if
  reports grow). The source GLB embedded once as a download button.
- **Charts**: Rust-generated inline SVG — root-motion top-down path, foot
  heights, L−R gait signal with the fitted fundamental, per-bone
  seam-delta bars — with a small shared JS playhead syncing a cursor line
  across all charts and the 3D view. A declared `gait_groups` blend ring
  adds one cross-clip figure per group: every member's L−R foot height on
  one unshifted normalized source-phase axis, each measured stride anchor
  marked at its own phase, and the group's `max_gait_phase_spread` drawn
  as a band around their circular mean. The members are never aligned to
  their anchors, because that would erase the disagreement the figure is
  evidence for, and no member is nominated as the reference the others are
  judged against — the check judges the set's spread and names none.
- **Findings panel**: each finding links to its clip + time; clicking
  scrubs the viewer.
- **Two clips side by side**: the `with` control (and the `with=NAME`
  fragment key) plays a second clip of the same document beside the selected
  one, as two scissored halves of the one canvas drawn through one camera
  fitted to both clips' bounds. The transport drives the selected clip, and
  every half — its pose, its chart playhead and its path dot — is placed at
  the frame nearest that phase in its own grid, with both source times
  labelled. Both viewers place their two timelines through that one shared
  mapping and label it with one shared disclosure sentence.
  Like the comparison's shared phase it is a presentation mapping between two
  timelines, not a retime: it selects samples the checks judged, nothing is
  resampled, interpolated, or blended, so the view claims nothing about a
  runtime blend of the pair.
- A skinned-mesh view (vendored three.js, `--report full`) is a P2 option
  the crate layout leaves room for; it is presentation polish, not v1.

## 10. FBX ingestion (`animsmith-fbx` + `convert`)

- **Library**: the official `ufbx` Rust bindings (v0.11.x, actively
  maintained; the same C foundation the incubating pipeline already trusted, so
  behavior is already trusted in the incubating pipeline).
- **Normalization at load** via `LoadOpts`: target axes = glTF convention
  (right-handed, +Y up, −Z forward), `target_unit_meters = 1.0` (FBX
  defaults to cm), transform-adjust space conversion (don't rewrite
  geometry), helper-node handling for 3ds Max geometric transforms. The
  core model only ever sees glTF-convention data regardless of source.
- **Animation extraction** uses ufbx's `bake_anim` — it evaluates anim
  stacks/layers, cubic/TCB curves, pre/post-rotation, and inherit-scale
  modes into resampled TRS keys (rate from the FBX TimeMode,
  overridable). Each anim stack (take) becomes one core `Clip`. This
  sidesteps the entire FBX-curve-semantics swamp.
- **Scale inventory without enablement**: the same successful parse produces
  a deterministic ufbx-side status for every current Appendix D.4 domain and a
  documented normalized source-node/source-skin projection when every declared
  skin slot can be represented. A missing cluster bone downgrades that generic
  projection to unavailable instead of silently dropping the slot. It
  explicitly names coordinate/inherit normalization, baked and lost curve state, generated
  normals/helper nodes, cluster-derived bind provenance, four-influence
  truncation/renormalization, rejected negative/non-finite/unrepresentable
  influences, triangulation/welding, omitted face/edge payloads, uninstanced
  mesh definitions, unsupported source data, and ufbx source identities. Only
  successfully projected cluster binds count as bone-convenience overwrites.
  One unreadable cluster makes its ordered bind declaration
  atomically unreadable; finite prefixes are not retained under shifted slots.
  Stackless authored curves, point/line faces, and attached zero-face mesh
  definitions are present-but-unsupported rather than absent or rebuilt; the
  zero-face definitions retain an exact count and stable source identity.
  Retained normalized meshes keep one definition per stable ufbx mesh identity;
  shared source geometry produces multiple node instances that reference the
  same compact normalized definition. They keep their stable ufbx typed
  identity so source-skin attachment joins cannot shift
  when an earlier source mesh emits no primitive.
  Its core capability projection remains unsupported for ordinary operations;
  narrow FBX `rest-bind` instead stages and proves a private GLB under the
  dedicated v5 contract. `whole-document` remains unsupported, and no raw FBX
  span or artifact-preservation claim is made.
- **`convert`** emits glTF 2.0 GLB: nodes + skin (computed IBMs) + one
  animation per clip with at least one writable track; mesh + weights when
  present; `--animation-only` to strip mesh. The glTF writer returns counts
  derived from the emitted artifact, which both `convert` and `transform` use
  for their summaries and to report source clips omitted because they have no
  writable tracks.
  Explicitly *not* an art exporter — no material fidelity promise. Both
  `convert` and `transform` share one `load`→`write` round-trip over `Document`
  (assets included), so geometry survives a transform pass and
  `--animation-only` clears it uniformly across input formats (it is the only
  lever that drops geometry).
  `--bake-static-mesh-transforms` is the separate opt-in path for static,
  unskinned, singly-instanced geometry: it removes the effective rest
  hierarchy transform by baking positions and normals under an identity root
  and reports each applied transform in conversion evidence. Unsupported
  animated, skinned, malformed/non-finite, singular/near-singular, or
  reflected input is rejected before output.
- **FBX pitfalls double as checks** when linting `.fbx` directly: source
  unit ≠ 1m (warn even though we convert), Z-up source, namespace-prefixed
  bone names (profile matcher strips), default "Take 001" naming (feeds
  `naming`), baked-key explosion (feeds `key-density`), non-uniform
  inherited scale.

## 11. Roadmap

- **M0 — walking skeleton.** Repo bootstrap: workspace, dual license, CI
  (fmt/clippy/test on Linux/macOS/Windows), CC0 test fixtures. Core model
  + sampler/FK; `animsmith-gltf`; `inspect` and `measure --format json`;
  the mechanical P0 checks (`nan` → `constant-track`).
- **M1 — reference parity.** Rig profiles, TOML config, per-clip
  expectations; port `loop-seam`, `frozen-bone`, `root-motion-speed`,
  `gait-phase`/`gait-group` — **golden-tested against the reference
  implementation's verified production numbers**; `lint` with exit codes + stable JSON; adopt
  the reference project's mutation-test discipline (corrupt one field, assert the finding
  names exactly that field). **the incubating project's measure
  port lands here**: its sidecar tool becomes a thin wrapper over `animsmith-core` + `animsmith-gltf`
  (the embedder keeps its sidecar schema and hashing);
  `locomotion_metrics.py` and the animation half of `measured_sidecar.py`
  are deleted.
- **M2 — report, FBX, diff → v0.1.0 on crates.io.** HTML report; FBX
  ingestion + `convert`; `diff`. The v0.1 bar for "usable by an unaffiliated
  team": README quickstart works on a raw Mixamo-style GLB with zero
  config (profile auto-detect), built-in `mixamo` + `ue-mannequin`
  profiles, sample `animsmith.toml`, versioned JSON schema doc, no incubator
  vocabulary anywhere in the public API.
- **M3 — the hard semantics.** `foot-slide` (stance detection),
  `in-place`, `bind-pose`/`axis-conventions`, rotational/velocity loop
  seams, `--deny-warnings`, baseline/suppression file for teams adopting
  with a dirty back catalog.

## 12. Risks & open questions

1. **Foot-contact detection robustness** (M3) is the only research-grade
   item — thresholds vary with rig scale and style. Ship opt-in at `warn`
   until tuned on a corpus.
2. **Test-asset licensing**: Mixamo clips cannot be redistributed. Use
   CC0 rigged clips (Quaternius/KayKit/self-authored) + procedurally
   corrupted fixtures; budget M0 time for this.
3. **Sampling-semantics fidelity**: engines differ subtly (cubic
   handling, sub-frame wrap). The doc pins the model — "glTF-spec
   interpolation on a uniform grid, wrap = (last, 0)" — and exposes grid
   fps as config, accepting it is a model of runtimes, not all of them.
4. **ufbx 0.x churn** and thin docs.rs coverage: pin exact versions,
   treat the C library's docs as canonical, keep `animsmith-fbx` thin and
   swappable.
5. **Rest-pose truth**: node TRS and inverse-bind matrices can disagree.
   Rule: IBM-derived rest is authoritative when a skin exists, node TRS
   otherwise, and disagreement beyond tolerance is a `bind-pose` finding.
6. **Report size** on long/many clips: f16 quantization + per-clip lazy
   JSON blocks if it bites.
7. **Scope pressure toward transformation** will come ("you detected the
   seam pop — just fix it"). The linter-first line is the identity; only
   `convert`, `transform`, and mechanical/lossless `fix` operations may
   mutate data.

## Appendix A — naming decision record

Two decisions, both 2026-07-03 (the project was renamed the same day
it was built, before anything was published):

1. **`animlint`** was chosen first, on a linter-first scope with clip
   transformation explicitly out of scope. Rejected then: `gltf-lint`
   (glTF is the carrier, not the domain; permanently confusable with
   Khronos glTF-Validator's spec-conformance role; wrong the moment FBX
   input landed), `animkit`/`clipkit`/`clipforge` (existing projects),
   `gaitkeeper` (minor collisions; overemphasizes locomotion).

2. **Renamed to `animsmith`** the same day, when transformation became
   first-class: the incubating pipeline needs hemisphere normalization,
   frame-range slicing, hold-extend, and gait-anchor rotation sooner
   than later, and its archived-FBX2glTF conversion step wants a
   maintained replacement. The naming record had said the lint-first
   name "only breaks if clip transformation becomes first-class" —
   that fork flipped, so the name followed before the first crates.io
   publish made it permanent. `animsmith` and all sibling crate names
   verified free on crates.io with zero GitHub repository hits. Lint
   remains the flagship subcommand.

## Appendix B — prior-art map (first consumer)

The measurement algorithms, config vocabulary, and testing discipline
were extracted from a private game project's asset pipeline — the
uniform-grid sampler, the local-neighbour loop-seam denominator with
its stride floor, the L−R fundamental-harmonic gait anchor, the
root-motion speed gate, the `Pinned{value, tolerance}` expectation
shape, the `Finding`/severity/exit-code conventions, and the
mutation-test style (corrupt one field, assert the finding names it)
are all faithful ports, golden-tested against that pipeline's shipped
numbers. Its measurement scripts are deleted as the project migrates
onto the animsmith library — the standing proof that the public API is
sufficient for a real bake pipeline.

## Appendix C — result-contract ownership decisions

These decisions record result-contract ownership after output v3 finalization:

1. **The versioned envelope belongs to `animsmith-core`, not the CLI binary.**
   The CLI still supplies its build identity and frontend policy, but
   `MeasurementContract`, the command-specific `MeasureFileReport` and
   `LintFileReport` records, command-specific envelopes, summaries, and URNs
   are shared library types. This lets embedded producers emit the exact
   schema without copying private structs or protocol strings while making an
   invalid measure/lint record shape unrepresentable.

2. **Nested findings keep their `check_id`.** A `Finding` is also a standalone
   embedded result and the record consumed by text, Markdown, and HTML
   presentations. Keeping it self-describing avoids a second wire-only finding
   projection and supports extracting findings without retaining their parent
   check record. Output v3 therefore accepts the small nested redundancy, and
   `evaluate_checks` rejects a child id that disagrees with its parent rather
   than serializing ambiguous ownership.

3. **Read-side validation is consumer-neutral.** `animsmith-core` validates
   output and nested measurement identities and recovers every file's display
   path plus complete clip and mesh measurements in source order. It does not
   impose a file-count rule or prescribe CLI remediation. The `diff` frontend
   owns its single-file policy and operator guidance; embedded consumers may
   accept empty or multi-file reports according to their own workflows.

## Appendix D — decision record: skinned rest/bind scale canonicalization

**Status (2026-08-20): implemented for self-contained glTF/GLB, the narrow
FBX rest/bind re-encode path, and character assembly recipe/evidence v7.** The shared
core plan, typed ledger, exact-source writers, independent core and artifact
proofs, atomic CLI publisher, and immutable scale-evidence v4 producer are the
current contract. Character assembly v7 can opt into rest/bind scale for
glTF/GLB and inventory-complete FBX inputs with an exact root name and factor
after validating a versioned basis for the base and every separate clip;
recipe/evidence v1 through v6 remain unchanged.
Whole-document conversion supports
raw glTF `POSITION` morph deltas while preserving static JSON weights as
numeric values and animated weight accessor payloads byte-exactly; other morph
semantics and every rest/bind morph payload remain refused. FBX
whole-document scaling remains deferred.

### D.1 Problem and two distinct operations

A skinned mesh can already have the intended physical dimensions while its
joint hierarchy carries a compensating uniform scale. For example, a root can
contribute `0.01`, descendant translations can be authored 100 times larger,
and inverse-bind matrices can carry a compensating linear magnitude of 100.
Skin deformation is then visually correct, but a rigid object parented to a
joint inherits the unwanted `0.01` scale.

That case must not be confused with a document whose *entire* linear unit is
wrong. The following are separate, explicitly selected operations:

1. **Whole-document linear-unit conversion** changes physical size. The caller
   declares a finite factor `q > 0`; every represented length is converted by
   `q`. It is appropriate only when the source was authored in a different
   linear unit.
2. **Rest/bind hierarchy reparameterization** preserves already-correct world
   geometry. It removes one compensating inherited scale from a restricted
   skinned hierarchy, rebases local rest and animation translations, and
   regenerates inverse binds.

Neither operation may infer its factor or applicability from mesh bounds,
character height, joint lengths, inverse-bind magnitude, filename, or an asset
category. Measurements from #267 are diagnostic evidence, not conversion
authority. A caller must name the operation and declare or accept the exact
factor that the transform plan validates.

The declared unit-conversion transform is a finite positive scalar only; it is
not an axis, rotation, translation, shear, or reflection operation. The initial
reparameterization source class is finite, orientation-preserving, positive
uniform rest-world scale. Non-uniform scale, shear, reflection, singular or
near-singular matrices, non-finite values, and mixed effective factors in the
selected domain fail before any output is written.

Classification and proof share one versioned tolerance policy and compute in
`f64`, narrowing only at the writer model boundary. The current policy identity
is `appendix-d-v6`. The three finite widened axis lengths are sorted in
ascending order before their arithmetic mean is accumulated and divided by
three. This canonical association rule makes the common factor independent of
authored axis order, including when the sum lies on an equal-axis boundary.
Relative orthogonality, equal-axis, and common-factor tolerance is `1e-5`; an
axis is unequal when
`abs(length - average) > 1e-5 * max(average, length)`, relative to the longer
of the two and to nothing else; a determinant is singular when
`abs(det) <= 1e-6 * product(axis_lengths)`; scalar/vector comparison uses
`abs_error <= 1e-6 + 1e-5 * max(abs(before), abs(after))`; and shortest-path
rotation residual is at most `1e-5` radians. Every bound is inclusive: a
residual exactly equal to its bound is accepted. Exact float equality is
forbidden. The implementation records every threshold and observed maximum
residual in evidence, so noisy values such as `100.000015` can be accepted for
an explicit, reviewable reason rather than by implementation accident.

This is a pre-1.0 policy-identity cutover: the former public
`ScaleTolerancePolicy::APPENDIX_D_V5` associated constant is removed rather
than retained as an alias for the new axis-order-independent meaning. Evidence
records therefore use `appendix-d-v6`. The historical v5 calibration and
revision history live in the
[scale proof calibration notes](docs/scale-calibration.md); the recurrence v5
introduced and v6 retains is specified below as part of the current policy.

**The `f32`-rounding term, per obligation.** The scalar band above is stated
relative to the quantity being *compared*. For five of proof's obligations
that quantity is not the magnitude the arithmetic ran on, and a rotation can
separate the two without limit: a bound component near zero on a mesh four
thousand units across, a near-identity `W * B` whose translation column is the
difference of two terms of magnitude `abs(W)`, or a world translation whose
parent chain cancelled two terms larger still. The compared number is
then small and the absolute rounding error it carries is large, so a purely
relative band derives the tolerance from the small number and the error from
the big one — and `plan_scale` accepts a plan whose candidate `prove_scale`
then refuses. Sweeping a rotating rig at a declared factor of `3190`, that
happened on `86 %` of rotations before this policy, with residuals up to
`9.8e-4` against a `6.1e-5` band.

`appendix-d-v6` therefore retains the further quantity `f32_rounding_ulps =
4`, and the five obligations that compare `f32`-rounded arithmetic use

```text
abs_error <= 1e-6 + 1e-5 * max(abs(before), abs(after))
             + f32_rounding_ulps * magnitude * 2^-23
```

where `magnitude` is named per obligation below. The added term is *absolute*
in `magnitude`, never relative to the compared quantity, which is what makes
it incapable of loosening a comparison whose operands already are its own
magnitude: there it contributes `4 * 2^-23 = 4.77e-7` of them, twenty times
below the `1e-5` the relative band already allows. The `2^-23` is
`f32::EPSILON`, because every one of these residuals is the difference of two
`f32` quantities even though the subtraction itself is done in `f64`.

- **Skinned bounds** takes `magnitude` as the largest per-vertex arithmetic
  provenance over every contributing vertex — read off the candidate, and
  maxed against the source's own *rebased by the conversion factor*:
  `candidate.max(q * source)`, for the reason given after this list. Each
  influence has three stages whose cancellation a later stage cannot recover:

  1. the magnitude the `W * B * p` transform runs on, which is
     `max over i of sum over k of abs((W * B)_ik) * abs(p_k)` with `p` extended
     by the homogeneous `1` that `transform_point3` sums the translation column
     in with — the *product* `abs(W * B) * abs(p)` and not either factor alone
     — because the weighted sum over slots that follows can cancel those terms
     against each other;
  2. the magnitude the contributing slot's `W * B` composition ran on, which
     covers the cancellation that composition performs; and
  3. the magnitude that slot's joint world translation was itself accumulated
     from along its parent chain, which covers the cancellation performed one
     composition earlier.

  For one influence, the base is the maximum of those three stages. A vertex
  combines those influence bases with the same binary64 weighted average of
  the stored non-negative binary32 weights as its skinned point:
  `sum(w_i * base_i) / sum(w_i)`. The accumulator then takes the maximum vertex
  base. All three stages are load-bearing, and each dominates the others by an
  unbounded ratio on some rig. The skinned points alone miss a joint placed far
  from the geometry it carries — a `1000`-unit local translation under a `3190`
  root puts the joint `3.2e6` from the origin while its vertices skin to within
  one unit of it, and the bound inherits `1.6e-1` of error from a `9.7e-1`
  extreme. Dropping the slot-composition stage makes the calibration sweep
  refuse correct candidates in fifty-six of its seventy-two pre-v4 cells, at
  up to `62.6` ulps of the base that remains; dropping the transform stage
  outright refuses in thirty-four, and narrowing it to `abs(p)` alone refuses
  in fifteen at up to `47.7`.

  **Stage 1 has been wrong twice, in the same place.** It first read the
  *skinned* point's magnitude and claimed that covered the transform; the
  correction read the vertex position `abs(p)` and claimed the same. Both name
  one term of a product of two, and each was exact only on the rigs that
  happened to be built.

  - The skinned point coincides with the transform only while no two slots
    oppose. Two slots whose composed `W * B` differ by a half turn send a
    vertex `1000` units out to a blended origin, so the result reads `0` while
    every term summed to produce it carried that vertex's own ulp; put the
    joints near the origin as well and stages 2 and 3 read `1`. The residual is
    then `6.1e-5` — one ulp of `1000` — against a `1.5e-6` band, a demand of
    `503` ulps, rising to `65_527` at `abs(p) = 1e5`.
  - `abs(p)` coincides with the transform only while `abs(W * B)` is `1`, and
    the one rig that reached the cancellation composed `W * B` from a *pure
    rotation*, where it is `1` by construction. Give those same two slots a
    uniform scale of `k` and the sum still cancels while each term carries the
    rounding of a `k * abs(p)`-magnitude transform: at `k = 1024` and
    `abs(p) = 65536` the residual is `4.0` against a `0.0313` band, a demand of
    `512` ulps, and the rig refuses from `min(k, abs(p)) > 8` upward with the
    shortfall growing as `min(k, abs(p))`.

  Both need only that the bind pose is not the rest pose, which is ordinary
  content: with `W * B` the identity on every slot there is nothing for a blend
  to cancel *and* nothing but `1` for `abs(W * B)` to be, which is why a
  section's worth of fixtures built on analytic inverse binds could express
  neither case.
  `two_slots_whose_composed_binds_cancel_a_vertex_still_prove_its_bounds` pins
  the cancellation and
  `two_slots_with_a_scaled_composition_cancel_a_vertex_and_still_prove_its_bounds`
  pins the second factor; both build their binds independently of the rest
  world for that reason, and the
  [calibration sweep](docs/scale-calibration.md) carries composed slots at
  `abs(W * B) = {1e-3, 1, 1e3}` so a third factor of the same kind is visible
  to a measurement rather than only to a reviewer.

  **The blended point is not another stage.** Shared scale-input validation
  refuses every finite negative primary skin weight before planning. The
  implementation widens the stored binary32 weights, transformed-point
  numerator, and denominator to binary64, divides once, and narrows the final
  point to binary32. This prevents a lone subnormal contribution from
  disappearing before division, several large finite weights from overflowing
  their denominator, and rounded binary32 coefficients whose sum exceeds one
  from overflowing an otherwise finite convex blend at `f32::MAX`. Bounds
  residuals are compared per axis, so each blended component is already
  bounded by the same binary64 weighted transform operands. v3's L2 length
  could be up to `sqrt(3)` larger while naming no additional per-axis
  arithmetic; v4 removes it.

  This validation decision removes two formerly unbounded affine cases. With
  weights `1.0` and `-0.99999`, a positive near-zero denominator could amplify
  opposed transformed points to `1.997e8`; with identical transformed points,
  both numerator and denominator could cancel while the accumulation error was
  still divided by the near-zero sum. No finite ULP count could cover the
  latter generally. The two #336 counterexample rigs remain as fixtures and
  pin `NegativeSkinWeight` at the shared planning boundary instead of forcing
  tolerance machinery to describe invalid input.

  Weighting the provenance is equally load-bearing in the other direction. A
  stage at magnitude `1e20` with weight `1e-20` contributes about `1`, not
  `1e20`; an unweighted max would let an influence with essentially no effect
  buy the whole distant stage's tolerance. Separate named fixtures isolate the
  transform and slot-composition halves, a proof-level fixture measures the old
  and v4 detection floors, and the calibration has an explicit profile that
  gives each vertex's larger production influence base the smaller weight.

  The proof-level floor is an adjacent-binary32 bracket on the same
  candidate-only weight mutation in both policies. A frozen v3 evaluator first
  refuses at candidate weight bits `0x3a7a1be4` (the preceding
  `0x3a7a1be3` is accepted), where the Bounds residual is
  `9.5320709005312e13`. v4 searches both directions around the authored
  `1e-20` weight; its smaller refused residual is `2.4400651454925537e-6`,
  with the lower bracket `0x1e3c6f0a` refused / `0x1e3c6f0b` accepted and the
  upper bracket `0x1e3d5b21` accepted / `0x1e3d5b22` refused. That is a
  `3.906e19x` recovery in this adversarial rig. It is a measured detection
  floor for this fixture, not a universal precision claim, and the exact bits
  and residuals are pinned in the test.

  `magnitude` is deliberately *not* read off the bound corner being compared.
  A per-axis extreme is contributed by whichever vertex happened to be
  furthest along that axis, so three vertices at `(3000, .001, .002)`,
  `(.001, 3000, .003)` and `(.002, .003, 3000)` build a corner of magnitude
  `2.4e-3` out of vertices of magnitude `3000`. A corner-derived band is a
  million times tighter than the error the corner carries, and — read the
  other way, as a base for the *relative* term — a corner-derived tolerance
  admits `4.2e-2` on a component of magnitude `5.98`, which is `0.71 %`
  relative error and no longer a rounding allowance at all.

- **The skin equation** and **the inverse binds of an unaffected instance**
  take `magnitude` as the largest quantity any entry of the compared product
  was summed from: `max over (i, j) of sum over k of abs(a_ik) * abs(b_kj)`.
  For `W * B` that is `6.4e3` on the rig above, not the near-identity
  product's `1.0`: the residual there is `1.8e-4` against the `1.1e-5` band
  the product magnitude buys, a `16x` shortfall, and `2.50` ulps of the
  operand magnitude after it. The skin equation maxes that against the same
  parent-chain magnitude skinned bounds takes, because `abs(W) * abs(B)` reads
  the *already composed* `W` and so cannot see terms the chain that produced
  it already cancelled. A joint whose local offset points back along its
  parent's world translation leaves `abs(W) * abs(B)` at `1.0` while the
  residual it must admit is `6.25e-2` — `524288` ulps of that base, and `0.08`
  of the chain's. The skin equation reads that quantity off the candidate and
  maxes it against the source's rebased by the factor, exactly as skinned
  bounds does, and for the same reason. It is also not
  `matrix_magnitude(a) * matrix_magnitude(b)`, which replaces the sum over
  `k` with a product of two independent maxima and reads `7.6e6` where the
  arithmetic ran on `6.4e3` — a tolerance from that would accept a matrix
  that is entirely wrong, and the over-acceptance calibration fixture refuses
  exactly that case.

  For an unaffected instance's binds the two operands are the effective matrix
  and the declared factor, and scaling a column cancels nothing, so the term
  is inert there: a correctly built candidate leaves the effective bind
  unchanged and therefore has exactly zero residual. It is stated rather than
  omitted so that the policy quantity means one thing across every obligation
  that compares `f32` matrices.

- **Rest translation** and **sampled trajectory** compare a node's world
  translation between the two documents, and take `magnitude` as that node's
  parent-chain magnitude alone — the same quantity the two above max against,
  read off the rest pose and off the sampled pose respectively. There is no
  product to sum here: the comparison is between two composed world
  translations, and the only way one of them can be small while its error is
  large is the chain cancellation.

  The root carries zero provenance because its world is a copied local. For
  child `i`, with parent world `W_p`, local matrix `L_i`, and inherited
  provenance `C_parent`, the shipped binary64 recurrence is

  ```text
  s_i,r = sum over k in {0,1,2}
          f64(abs(W_p[r,k])) * f64(abs(L_i[k,3]))
  p_i,r = f64(abs(W_p[r,3]))
  m_i,r = s_i,r + min(max(p_i,r, f32::MIN_POSITIVE), s_i,r / 2^-23)
  m_i = max over spatial rows r of m_i,r
  C_i = C_parent + m_i
  ```

  The first term provisions the local three-product dot. The second provisions
  the final addition by paying the smaller of one ulp at the carried parent's
  scale and the whole new contribution being absorbed. The minimum-normal
  floor covers subnormal product rounding without imposing a fixed charge when
  `s_i,r = 0`. Only the three spatial output rows participate; affine row 3 is
  the exact homogeneous coordinate. Rest and sampled pose construction share
  this recurrence. Its calibration populations and revision history are
  recorded separately in the
  [scale proof calibration notes](docs/scale-calibration.md). The
  policy-neutral matrix rewrite, residual, and provenance kernels live in
  `crates/animsmith-core/src/scale/numeric.rs`; operation selection, tolerance
  decisions, reference expectations, connector products, and proof-owned
  expected values remain outside that leaf.

  This scalar is not a universal componentwise forward-error proof: it does
  not separately propagate error in the inherited linear block before that
  block acts on a later translation. The shipped claim is this additive
  recurrence and its checked empirical envelope, not an unbounded-depth
  analytic guarantee. At the later fixed stages, SkinMatrix maxes the
  accumulated chain against the `W * B` operand magnitude and Bounds feeds
  that slot base through the weight-proportional blend described above. A
  fully analytic matrix-error propagation would be a different model and
  policy contract.

  The chain is the **candidate's**, not the source's and not the max of the
  two. Whole-document conversion scales every translation by the factor and
  leaves every linear part alone, so the two documents' chains are that factor
  apart — subject to the candidate's `f32` narrowing of the factor, since the
  build scales by `factor as f32` while the proof rebases by the `f64` factor,
  a relative difference of at most `2^-24` (`1.49e-8` at `q = 0.1`, whose
  `f32` is `0.10000000149011612`) that this very rounding term covers many
  times over — and the source's rounding is rebased by the same factor before
  it is compared, so the residual scales with the candidate's chain at either
  end of the factor range, and under rest/bind the two chains are equal
  outright. A max over the two sides is therefore never *needed*, and under a
  shrinking conversion it is strictly worse: at a factor of `0.01` the source
  chain is `100x` the candidate's, so the max freezes the band at the source
  rig's size while the candidate it is spent on keeps shrinking, and the
  smallest genuinely wrong candidate either obligation refuses stops tracking
  the factor at all. On the cancelling-chain rig that costs `89x` of
  discriminating power at `0.01` and `648x` at `1e-4`.
  `a_shrinking_conversion_holds_rest_translation_to_the_candidate_s_own_chain`
  and `a_shrinking_conversion_holds_trajectory_to_the_candidate_s_own_chain`
  pin the tightening; the two `..._holds_..._to_the_candidate_side` fixtures
  pin the growing direction, where the candidate's chain is also the larger.

**Transform-only affine is not in this rounding class.** It compares a probe
point transformed through the complete expected and actual world affines as a
vector L2 residual against a vector-magnitude base, so its comparison base is
already the magnitude its arithmetic ran on and it takes no rounding term.

**Overflow is not `NaN`.** A skinned position that leaves the `f32` range is
reported as `InvalidSkinnedPrimitive { reason: "skinned_magnitude_overflow" }`,
separately from the `"non_finite_result"` reported for `NaN`. Both fail closed,
and neither has a fixed preflight magnitude limit: skinning accumulates a dot
product per axis, so overflow depends on orientation and operand magnitudes,
not only on the magnitude of the result.

The magnitude calculations follow the same finite-domain rule. Bounds are
compared per axis and never square a finite blended point into an L2 length
that could overflow above `sqrt(f32::MAX)`. The parent-chain
`abs(W_parent) * abs(t_local)` sums are accumulated in binary64. Composition
and column operand magnitudes are summed in `f32` lanes and recomputed in
binary64 whenever that sum leaves the `f32` range, so an operand magnitude is
never reported as `inf` while the composed result remains finite. These are
current `appendix-d-v6` proof and refusal semantics, not calibration
measurements.

**Why the source side is rebased.** Every residual above is `|a - q * b|` for
the candidate quantity `a` and the source quantity `b`, so the source operand
enters the comparison multiplied by the factor and its *rounding* is multiplied
by the factor with it: a source quantity accurate to `k` ulps of its own
magnitude contributes `q * k` ulps of that magnitude to the residual. A base
that reads the source side unrebased — which `max(source, candidate)` did for
skinned bounds and the skin equation — therefore states the source's error in
the wrong units by a factor of `q`. Under a shrinking conversion that is the
whole defect, and it is loose by exactly `1/q`: the band freezes at the source
rig's size while the candidate it is spent on keeps shrinking. Measured on the
far-joint rig, the smallest inverse-bind shift the skin equation refuses goes
from `1.9e-3` under the max to `1.3e-5` at `0.01` and `1.3e-7` at `1e-4`; the
smallest bounds error refused goes from `4.8e-1` to `4.8e-3` and `4.9e-5`.
`a_shrinking_conversion_rebases_the_skin_matrix_magnitude_by_the_factor` and
`a_shrinking_conversion_rebases_the_bounds_magnitude_by_the_factor` pin both.

Unlike the parent chain this does *not* reduce to the candidate's magnitude
alone, because these two magnitudes are products: only the terms carrying a
translation are a factor apart, and both sides retain an unscaled `O(1)` floor
from the composition's linear block and from the exact `1.0` the homogeneous
row contributes to every chain. Where that floor dominates the source,
`q * source` exceeds the candidate's magnitude under a growing conversion by up
to the whole factor. Production retains the `max` because it is the operand
bound justified by the comparison; no smaller candidate-only or `min` base is
part of the policy contract.

An earlier calibration tried to classify those smaller bases as equivalent by
dividing an obligation-wide maximum residual by a separately selected maximum
slot base. Those maxima can belong to different slots: after calibration began
recording each production comparison's own residual and provenance, the
shipped SkinMatrix demand was `2.213` while that mispairing still printed
`2.084`. The alternative-base figures and equivalence claim were removed
rather than preserved as false mutation evidence.

**Calibration evidence.** The checked-in release calibration, its deterministic
populations, current reference demand tables, historical policy comparisons,
and machine-local cost measurements live in the separate
[scale proof calibration notes](docs/scale-calibration.md). Those notes measure
this policy; the normative thresholds, provenance definitions, and refusal
boundaries remain in this decision record.

**Normative residual norm for the unit-scale postcondition.** The postcondition
"unit composed scale for every affected node" is measured **per axis**, as
`max(abs(scale_axis - 1))` over the three axes — an L-infinity norm, not an L2
norm over the three axes together. This is normative. Per-axis is what the
claim means, it is how the equal-axis check already measures, and it is the
same dimensionless relative quantity the scalar common-factor check compares,
so the input band and the postcondition are directly commensurable.

**How the common-factor band relates to it.** The common-factor band is the
normative *input* contract, and the unit-scale postcondition bound is *derived*
from it rather than declared independently. For an affected node `i` with
observed rest-world factor `s_i`, a domain common factor `s_0`, and a declared
factor `s_declared`, the candidate's composed scale on axis `k` of node `i` is
`axis_ik / s_declared`. Three independently bounded relative comparisons stand
between those two numbers, and an affected chain composes all three:

- the declared-factor match binds `s_0` to `s_declared` within `c = 1e-5`;
- the mixed-factor check binds every other affected node's `s_i` to `s_0`
  within the same `c`; and
- the equal-axis check binds each individual axis length `axis_ik` to `s_i`
  within the same `c`.

The third band is the easiest to lose sight of and cannot be dropped: `s_i` is
the *average* of node `i`'s three rest-world axis lengths, while the
postcondition measures an individual axis, so an axis is permitted a further
band away from the very number the mixed-factor check compared. A rig whose
root is off-factor, whose leaf is off-common-factor, and whose leaf axes are
non-uniform within the equal-axis band loads all three at once.

Each comparison is stated relative to `max` of its operands, so each
contributes at most `c / (1 - c)` when re-expressed relative to the smaller
one, and the composed worst case is `(1 - c)^-3 - 1 = 3.00006e-5`. The policy
therefore sets the postcondition bound to four common-factor bands — `4e-5` —
rounded up to the next power of two, `2^-14 = 6.103515625e-5`. Three of those
bands are the analytic composition above; the fourth is headroom for the `f32`
world-matrix composition and decomposition that produces the measured composed
scale. `2^-14` is also `512 * 2^-23`, and so lies on the binary32 mantissa grid
the measurement lives on, which is what makes the inclusive "at most" above
reachable rather than vacuous for this obligation.

Three bands rounded up (`2^-15 = 3.0517578125e-5`) would leave that analytic
worst case only four binary32 ulps of room — `5.17e-7`, or `4.34 * 2^-23`.
That is a rounding artefact, not headroom, and it would make the reserved
float-headroom band above a claim rather than a fact. The fourth band buys the
claim honestly, and it does not blunt the obligation: every build defect this
postcondition exists to catch — a dropped rebase, a factor applied twice, a
stale no-op candidate — is at least `1e-3`, so `6.1e-5` still leaves better
than a `16x` detection margin.

**No floor on the comparison base.** The common-factor and mixed-factor
comparisons are `abs(a - b) <= c * max(abs(a), abs(b))`, with nothing else
inside the `max`. Flooring that base — at `1.0`, at the `1e-6` absolute scalar
term, or at anything else — turns the band into an absolute tolerance below the
floor, which is a *relative* band of `floor * c / abs(s)` that widens without
limit as the declared factor shrinks. It therefore breaks the closure property
below `floor * c / 2^-14`; at a `1e-6` floor that is
`1e-11 * 16384 = 1.6384e-7` (the crossing point is *inversely* proportional to
the postcondition bound, so the `2^-15` bound an earlier revision declared put
it at twice that, `3.2768e-7`), and at a declared factor of `1e-9` such a band
admits `1e-2` relative error, `1000x` the declared policy. The degenerate
`a == b == 0` case needs no floor either: it compares `0 <= 0`.

What the derivation guarantees is the closure property: **any plan the
common-factor band accepts yields a candidate whose every affected node
satisfies the unit-scale postcondition** — at every declared factor the policy
admits, not merely near unit magnitude — with a full band to spare
(`6.1035e-5 - 3.00006e-5 = 3.1035e-5`). The guarantee is stated for the single
closed affected domain this record admits — one common factor, one declared
factor, hence exactly the three composed bands above. Admitting a second
independently classified factor in one plan would compose a fourth band and
require a new policy identity, not a wider bound at this one.

**Sampling budget.** The policy also bounds the total sampled proof work a
document may demand, as `sample_time_count * per_sample_work_units` against a
budget of `4e8` work units. The per-sample cost is every pass the sampled
obligations actually make, and each is charged once per document *side*, since
proof walks the source and the candidate:

```text
2 * bone_count
  + sum over affected skinned instances of
        [skin_and_bounds] *
          (3 * len(skin_joints)
           + 2 * (vertices over every primitive of its mesh))
```

Every sampled obligation poses both skeletons, hence the bone term. Only the
*source* bone count is measured, which is sound because proof rejects a
candidate whose bone count differs (`bone_count_mismatch`) before it charges
anything: the candidate document is caller-supplied, so an unchecked candidate
skeleton is proof work nothing has billed. The slot term is charged explicitly,
per instance: nothing bounds the instance count, and nothing bounds
`skin_joints`, which may repeat a joint, so slot work bears no relation to the
bone count and must not be folded into it. The vertex term charges each vertex
once, while the `f32`-rounding stage of the skinning walk runs once per non-zero
influence. A four-influence vertex therefore performs that stage four times for
the same vertex charge; the budget is the fixed released work proxy calibrated
for the documented populations, not an exact instruction counter.
`sample_time_count` counts key times
*and* cubic-segment interior times, because both are evaluated. Exceeding the
budget is a typed refusal raised before the first sample time is evaluated;
proof never silently samples a subset. The comparison is `>`, so the budget is
a ceiling a document may reach, matching the inclusive "at most" every other
policy quantity is stated with. The budget is part of the policy identity for
the same reason every tolerance is: it is recorded in evidence, and two
evidence records carrying the same policy id must describe the same amount of
checking.

The calibration notes record the historical timing shapes and measurements
that established this charge as a linear proxy for real work. Those seconds are
machine observations, not a normative wall-time guarantee.

The value has to clear real assets as well as bound bad ones: a 200-bone rig
with a 100k-vertex skinned mesh costs `201_000` units per sample time, so a
30-second clip at 30 fps costs `180_900_000`. A budget that refuses that
refuses a plausible production asset, with no way for a caller to opt into the
work.

Historical implementation cost and baseline comparisons are retained in the
[scale proof calibration notes](docs/scale-calibration.md), not in this
normative work-budget definition.

### D.2 Algebra and rewrite rules

Let `L_i(t)` and `W_i(t)` be node `i`'s local and composed world matrices, `G`
be a skinned primitive's geometry-bind-to-world matrix, and `B_i` be its
inverse bind. Valid input satisfies

```text
W_i(rest) * B_i = G
```

for every joint slot. Matrices below act on column vectors.

#### Whole-document linear-unit conversion

For the caller-declared factor `q`, define `U = scale(q)`. Coordinates change
basis as `M' = U M U^-1`. For a uniform `U`, this multiplies an affine matrix's
translation by `q` while leaving its dimensionless linear part unchanged.
Therefore:

- multiply every node-local rest translation by `q`;
- multiply every animation translation value and both glTF cubic-spline
  translation tangents by `q`; key times remain seconds;
- multiply mesh `POSITION` values and morph `POSITION` deltas by `q`;
- conjugate every per-skin inverse bind as `B_i' = U B_i U^-1`;
- multiply every supported camera, light, collision, or extension length by
  `q` through a field-specific handler; and
- leave rotations, normals under positive uniform conversion, UVs, weights,
  times, morph weights, and rest/animation scale channels unchanged.

glTF `LINEAR` and `STEP` samplers have values only; they do not store tangent
elements to rewrite. “Translation tangents” throughout this record therefore
means the explicit in/out elements of a `CUBICSPLINE` translation sampler.

Root-motion positions and velocities are not separate stored fields: they are
recomputed from the converted translation tracks. Distances and velocities
therefore change by `q`; durations do not. Animation scale is dimensionless
and is never multiplied as though it were a distance.

This operation preserves topology and semantics while intentionally producing
world positions `q` times the source positions. It is not a repair for an
already-correct mesh with a compensating skeleton scale.

#### Rest/bind hierarchy reparameterization

For the initial supported class, every affected joint has a rest-world linear
part within tolerance of `s R_i`, where `s > 0` is one common factor and `R_i`
is a proper rotation. The affected domain is a closed connected hierarchy:
the scaled ancestor, all selected skin joints and paths between them, and all
descendant nodes whose attachment transform would otherwise inherit `s`.
A strict source-node connector with no normalized bone may occur on a path
between two projected nodes when its only modeled contribution is its authored
static local rest, which must be finite and affine. Planning composes that
local in source order; the connector does not become an affected normalized
node, its authored local is preserved exactly, and it owns exact-preservation
and projected-successor bridge obligations rather than a numeric residual.
The selected root and every selected skin joint must still project to
normalized bones. An
unprojected descendant or attachment that would itself need rewriting, a
second effective factor, or an instance outside the closure still rejects the
plan.

The compiled rest/bind plan records this source topology once, keyed by raw
source-node identity. A projected row also records its normalized `BoneId`,
authoritative raw parent, nearest projected parent, role, and the incoming
connector tail from which the ordered span is derived. A connector topology
row records its source and parent identities; field rows state its exact
preservation, and the projected successor identifies ownership of the bridged
rewrite. Derived indexes may accelerate lookup, but they are not additional
topology authorities. This distinction matters to embedders: source-node
indices select and preserve raw structure, while `BoneId` identifies
normalized rewrite and residual domains.
The same global table also retains `OutsideDomain` rows, including every
source row for whole-document conversion, so raw parent/projection identity is
never inferred from absence.

For affected node `i`, define the constant basis correction
`C_i = scale(1 / s_i)`, where the initial contract admits `s_i = s` inside the
domain and `s_i = 1` at its parent boundary. The desired world matrix is

```text
W_i'(t) = W_i(t) * C_i
```

and the corresponding local matrix is

```text
L_i'(t) = C_parent^-1 * L_i(t) * C_i .
```

The same rule remains valid across a preserved static connector, but it does
not reduce to skipping that connector or blindly applying the projected
parent's scalar to the next raw local. Let `H` be the ordered product of
unprojected connector locals between projected parent `p` and projected node
`i`, and let `L_i` be `i`'s authored local after that connector span. Since
`H` is preserved, the projected successor's authored local must become

```text
L_i' = H^-1 * C_p^-1 * H * L_i * C_i .
```

Then `W_p' * H * L_i' = W_p * H * L_i * C_i`, as required. A translated
connector makes the conjugation's additive translation term observable, so a
non-identity connector fixture is required; an identity-only fixture cannot
distinguish composition from omission. This is a static-rest rule only:
`SourceNodeAsset` carries no raw animation channel for an unprojected row, so
support for a frontend that drops an animated connector requires a separate
source-animation inventory and rewrite contract.

Because every admitted `C` is a positive uniform scale, the result remains
representable as TRS. On a direct projected edge (`H = I`) — including the
normalized `Bone` local into which a loader has already folded a connector —
the node-local translation is multiplied by the parent's `s_parent`, rotation
is unchanged, and local scale is multiplied by the dimensionless ratio
`s_parent / s_i`. For a raw projected successor after a preserved connector,
the linear part receives that same ratio but its translation is instead
determined by `H^-1 * C_p^-1 * H * L_i * C_i`; a translated `H` contributes
an additive term, so the simple translation multiplier must not be applied to
that raw local. The connector product, inverse, additive term, complete
bridged translation sum, and bridged linear ratio application are evaluated
in `f64`, then each final source-local component is narrowed once to the
`f32` model boundary. Direct projected edges retain their established `f32`
arithmetic and association. Planning likewise keeps each admitted connector
span widened until its projected endpoint, so a built candidate that retains
Complete source projection remains a consumable factor-one planner input even
when the preserved connector-only product exceeds `f32`. Translation
animation values and both cubic translation
tangents on normalized nodes receive the existing parent-basis multiplier.
Scale animation on those normalized nodes is the same local-scale component
over time, so every stored VEC3 scale element receives `s_parent / s_i` too:
the selected closure root receives `1 / s`, a strict affected descendant
receives `s / s = 1`, and an unaffected node receives `1`. This is a topology
rule, not a constant-track predicate: `LINEAR`, `STEP`, and every value plus
in/out tangent of `CUBICSPLINE` are rebased. Rotation tracks and key times
remain unchanged.

A node on a direct projected edge that declares `matrix` rather than TRS
members takes the same rule in component form. Writing `L_i'` out in glTF's
column-major order, the nine linear entries (`0,1,2, 4,5,6, 8,9,10`) are
multiplied by `s_parent / s_i` and the three translation entries
(`12, 13, 14`) by `s_parent`. Components
`3, 7, 11, 15` are written back exactly as authored, which is the correct
rebase only because the §D.4 gate has already proved them exactly
`(0, 0, 0, 1)`: the basis change multiplies `3, 7, 11` by `1 / s_i`, a no-op
on an exact zero and on nothing else, and leaves `15` alone at any value, so
what the gate buys there is not the arithmetic but the premise — a `matrix`
is the node's whole transform, with no projective divide this record models,
only when `15` is exactly one. §D.3 case 4 records why that gate compares
exactly rather than within tolerance. A raw projected successor after a
preserved connector is governed by the full `H` product above, not this
component shortcut.

Mesh positions, morph deltas, and normals remain unchanged because their world
geometry is already correct. Each inverse bind is regenerated from the
unchanged geometry bind:

```text
B_i' = inverse(W_i'(rest)) * G = C_i^-1 * B_i .
```

Consequently `W_i'(t) * B_i' = W_i(t) * B_i`: the complete skin palette,
not merely joint origins, is analytically preserved.

Both bone convenience inverse binds and every per-instance skin-slot array must
be updated; the single bone-level value is never authority for a multi-skin
document. A raw format has one bind store per skin and no per-node bind, so the
frontend rewrites each affected skin's stored matrices with per-slot factors and
proves through the reloaded document that every per-instance array and bone
convenience value is correspondingly correct. The output deliberately changes
the affected nodes' composed scale to one while preserving their world
translations and orientations, animation trajectories, skinned vertices, and
bounds.

**Conflicting raw accessor claims reject.** Unlike whole-document conversion,
whose converted uses all take the same `q`, rest/bind derives a typed claim for
**every** raw accessor use, not only a scale-bearing write. A claim records the
accessor's expected component shape and its multiplier. Translation outputs,
scale outputs, and inverse-bind slots carry their topology- or slot-derived
multiplier. Factor-one preservation claims cover mesh attributes and indices,
sampler inputs, rotation and weights outputs, and sampler outputs that no
channel selects. The latter are still sampler-referenced payloads, not unused
bytes that the raw proof may overlook.

One accessor index must have compatible component shape and one multiplier
across all of those claims. Thus a translation or scale output reached from the
closure root and from a child, from an affected and unaffected node, or an
accessor used both as mesh `POSITION` and a translation or scale output can
reject; so can a same-index type conflict even when both factors are one. One
inverse-bind store shared by skins whose joints straddle the closure is the
same problem. Splitting the payload is not a remedy, because it changes array
lengths and destroys the identities the proof pins.

Distinct accessor indices are checked too: if their resolved byte ranges
overlap and either claim rewrites bytes, the operation refuses rather than
silently corrupt a preserved neighbour. Direct-image `bufferView` consumers
are not accessors and have their own separate image-overlap refusal. Within
the source domain admitted by the common conservative capability preflight,
uses whose claims agree impose no additional rest/bind restriction however
many reach one accessor. The preflight can still refuse a
scale-bearing/dimensionless alias or overlapping raw range before this ledger
runs. A declared factor of one makes every rest/bind multiplier one, so this
ledger's factor disagreement cannot fire on a no-op; it does not broaden the
preflight's supported raw domain.

### D.3 Worked synthetic cases

The implementation fixtures use literal analytic values rather than values
generated by the transform under test.

1. **Unit skeleton.** A root and child both have unit scale, the child rest
   translation is `(0, 1, 0)`, and the IBM is rigid. Reparameterization with
   an expected factor of one is a deterministic no-op. Requesting any other
   factor rejects as a source-fact mismatch. A whole-document conversion by
   `0.01` instead produces a `0.01 m` child offset and mesh while leaving scale
   channels at one.
2. **Compensated inherited scale.** A root has scale `0.01`; its child has
   local translation `(0, 100, 0)` and local scale one. Their composed child
   position is `(0, 1, 0)`, every selected joint has effective scale `0.01`,
   and the corresponding IBM linear columns have magnitude 100. The
   reparameterized root and child have unit composed scale, the child local
   translation is `(0, 1, 0)`, the IBM is rigid, and mesh positions and the
   child world position stay byte-for-byte or tolerance-equivalent as defined
   by the proof. A transform-only child at local offset `(1, 0, 0)` is rebased
   to `(0.01, 0, 0)`, preserving its world origin while its composed linear
   scale changes from `0.01` to one. Transforming a further off-origin point
   through that child's complete affine distinguishes the result from a no-op.
   Checking only `joint_translation + joint_rotation * offset` is invalid
   because it drops the very scale channel the fixture must prove. Unskinned
   geometry in the affected closure is initially rejected rather than silently
   resized; supporting it would require a separate geometry and instancing
   rule.
3. **Separate clips.** A compatible clip declares the same named parent
   topology, rest orientations, helper layout, and `0.01` translation basis;
   its child translation value and cubic tangents are rebased by `0.01` before
   remapping. A name-identical clip with a unit parent basis, different parent
   rotation, different helper layout, or a mixed factor fails compatibility
   before any keys are copied.
4. **Rejected affines.** Independent fixtures introduce `(0.01, 0.02, 0.01)`
   scale, a shear term, a negative determinant, a zero/near-zero determinant,
   `NaN`/infinity, and one joint with an effective factor different from the
   common factor. Each must produce a stable typed rejection, no artifact, and
   no partial evidence publication. Two kinds of near-valid noise are treated
   differently, by design. Values that reach the tolerance-bearing affine
   classification — a `100.000015` IBM column, or a rest scale axis skewed
   within `equal_axis` — are accepted when their measured residual is inside
   the declared tolerance, and that residual is recorded. Structural facts
   checked at the §D.4 capability gate are compared **exactly**: a node
   `matrix` whose last row is not exactly `(0, 0, 0, 1)` is a
   `NonAffineNodeMatrix` refusal, so `matrix[15] = 1.0000001` is rejected
   before any tolerance is consulted. The exactness is deliberate on three
   grounds: it matches glTF-Validator, whose `isTrsDecomposable`
   (`lib/src/utils.dart`) tests those same four components with exact `!=`
   before it applies any tolerance and reports `NODE_MATRIX_NON_TRS`;
   composing two affine matrices produces an exactly zero bottom row in
   floating point (`0*x + 0*y + 0*z + 1*0`), so no real composition pipeline
   emits noise there; and the §D.2 rewrite rules *rely* on it — the
   reparameterization's node-`matrix` rebase writes components
   `3, 7, 11, 15` back as authored, so a tolerated `matrix[3] = 1e-12` would
   be published unconverted where the basis change owed it `1 / s_i`, and a
   tolerated `matrix[15] = 1.0000001` would be rebased as though the node
   were affine. Relaxing it is a change to the §D.4 contract, not a tolerance
   widening; §D.1's ban on exact float equality binds the comparisons that
   tolerance policy governs — classification and proof — not this gate.

### D.4 Modeled domains and preservation coverage

The transform plan owns an explicit capability manifest. A field absent from
`Document` is not evidence that the source lacks that field; preflight must
inspect the raw input or a loader-supplied complete inventory before mutation.
The plan compiles every modeled container into a typed field ledger. Each row
identifies the target and component, then states either a structural semantic
rewrite kind or `PreserveExact`; absence is not used to mean preservation. A
row does not carry a resolved factor, expected value, or connector product:
candidate construction and proof independently derive those numbers from the
operation and validated topology. `PreserveExact` is typed builder write-set
ownership: the core builder does not write that field. It is not, by itself, a
bit-equality rule for normalized derived state presented to `prove_scale`.
Container rows carry the counts and shapes needed to describe every element
without storing one row per vertex, key, tangent, or matrix component. Candidate
construction consumes this ledger, so each authoritative modeled production
write discharges one rewrite row and every rewrite row must be discharged.
Whole-document conversion retains one compatibility-only exception: raw locals
kept beside `Unavailable` source coverage are still converted best-effort, as
before, but that coverage explicitly makes their identities non-authoritative,
so they produce neither ledger rows nor proof/replay claims. Factor-one rows remain
explicit where identity, aliasing, and write-set coverage depend on them.
Separate typed payload-shape rows retain document, clip, track, instance, mesh,
and primitive identity and cardinality, including empty containers, without
freezing key times or other numeric values. Exact replay therefore detects
added, removed, reordered, or retargeted payload that happens not to add a
field row or obligation.

| Domain | Current model boundary | Unit conversion | Reparameterization |
|---|---|---|---|
| Rest hierarchy | `Bone::rest`; source-node TRS/matrix and parent topology form a canonical typed source table | Scale translations; conjugate retained matrices; preserve topology explicitly | Rebase affected normalized/projected locals; exactly preserve typed static-connector rows; bridge only the projected successor; preserve orientations and topology explicitly |
| Translation animation | Values and cubic tangent triplets are retained for normalized nodes | Multiply values and both tangents by `q` | Multiply by the affected parent-basis factor; infer no raw animation support for an unprojected connector |
| Rotation and scale animation | Retained as dimensionless values | Leave unchanged | Rotation unchanged; rebase each scale VEC3 by `s_parent / s_i`, including cubic tangents |
| Root motion and velocity | Derived from translation tracks | Convert tracks, then recompute | Preserve sampled trajectory and derived velocity |
| Base mesh geometry | Base `POSITION` and normals are retained | Scale positions; normals unchanged | Leave positions and normals unchanged |
| Morphs | Not retained by the normalized core; raw glTF ownership inventories target accessor identities and weight locations | Raw glTF only: scale dense `f32` `POSITION` deltas once per unique accessor; preserve static JSON weights as numeric values and animated weight accessor payloads byte-exactly; reject other semantics and unsafe aliases/layouts | Reject: rest/bind morph preservation is not proven |
| Skin binds | Per-instance IBMs plus a lossy bone convenience value | Conjugate every per-skin matrix | Regenerate every per-skin matrix from output joints and unchanged `G` |
| Cameras/lights | Node transform only; typed fields are not modeled | Reject until all length fields have handlers | Reject when attached to the affected domain until preservation is proven |
| Collision/custom data | No semantic model for extras or extensions | Reject unless a registered handler covers every length | Reject when affected unless exact preservation is proven |
| Other vertex/source data | Several attributes, non-triangle modes, and extension payloads are not writer-preserved | Reject on the normalized-model route | Reject on the normalized-model route |
| Out-of-contract node transforms | A `matrix` beside a TRS member, or a `matrix` whose last row is not **exactly** `(0, 0, 0, 1)` — compared exactly at the gate, never within tolerance (§D.3 case 4) — parses but is not glTF 2.0 | Reject: which transform the author meant is unknowable, and `U M U^-1` leaves `3, 7, 11` unconverted where the basis change owes them `1 / q`, while a `15` other than one is invariant under the rewrite and so would be published still asserting a projective divide (§D.2) | Reject for the same reason, with `1 / s_i` for `1 / q` |
| Animation targeting a matrix node | glTF 2.0 requires an animated node to use TRS; a typed reader can otherwise decompose `matrix` and lose that source distinction | Reject at raw preflight | Reject at raw preflight |
| Shared raw accessor payloads | Every accessor use has a component-shape and typed field disposition; preserved uses remain explicit factor-one claims | Convert once per unique accessor: every converted use takes `q` | Reject same-index type/factor conflicts and overlapping distinct accessors when either range is rewritten (§D.2) |
| Unreferenced accessor payloads | Retained only in the raw source container, including dense bases and sparse index/value payloads | Preserve exactly; reject when any owned byte span overlaps a rewritten accessor | Preserve exactly; reject when any owned byte span overlaps a rewritten accessor |
| Image payload aliases | An `image` reads a `bufferView` directly and never becomes an accessor | Reject when its bytes overlap a rewritten accessor | Reject when its bytes overlap a rewritten accessor |

The narrow normalized FBX bridge applies the collision/custom row at the
operation boundary rather than treating every parser-observed declaration as
scale-bearing. User-defined properties are explicitly discarded before the
private GLB is written, and parser-known texture/video linkage is represented
by bounded resource facts and dependency-closure evidence rather than a
transform field. The source-aware gate may therefore admit those two known
classes while retaining their inventory/evidence rows; rooted capture keeps
supported texture assignments in the normalized document and private GLB. The
same proof boundary admits exactly ufbx's marker, LOD-group, stereo-camera,
camera-switcher, and display-layer typed lists: those records supply no
hierarchy transform, skin bind, animation track, or geometry to the normalized
bridge. Display layers contain only node membership and editor
visibility/freeze/color state.
Shader and binding-table records are material-evaluation metadata and are
admitted on the same basis. BindPose is different rest/bind-domain evidence:
the same-load gate admits it only when it covers every joint of each skin it
touches and every converted row is finite, unambiguous, and agrees under the
fixed Appendix D scalar tolerance with the converted cluster bind or node
rest-world matrix the bridge already consumes. A source without a Pose remains
valid; incomplete, ambiguous, non-finite, or mismatching BindPoses fail closed
as distinct residual kinds. All admitted rows remain counted in the raw
unmodeled-element aggregate. Every residual
unmodeled typed list fails closed, and its refusal retains the exact nonzero
kind counts. This remains a normalized re-encoding rather than a raw FBX
object-property, marker/camera/display-layer behavior, material, or
texture-assignment preservation claim. Extensions, incomplete
construct/resource coverage, and
every unsupported transform, geometry, bind, or animation fact still reject.
The inventory-only API remains conservative because its frozen v5 aggregate
cannot distinguish these discharged classes from another unmodeled source
element.

The connector exception is deliberately narrower than general helper-node
support. The source projection can attest an unprojected row's identity,
parent, and authored local rest, but not a raw animation channel targeting
that row. A frontend that omits such a channel from the normalized model has
not preserved it and cannot cite the static connector rule as evidence that
it is safe to rewrite.

The current glTF writer rebuilds nodes as TRS, emits only modeled triangle
attributes, creates skin/holder structures, and does not preserve arbitrary
source JSON. The FBX loader mechanically inventories every current row above,
including the unverifiable unreferenced-accessor analogue, and exposes a
documented normalized ufbx source-node/source-skin sidecar when every declared
joint slot is representable. A missing cluster bone downgrades the generic
sidecar to unavailable, and one unreadable converted bind makes its whole
ordered declaration unreadable with no retained prefix. Complete coverage here
means every adjusted/compensated ufbx node and skin has stable identity and
projected values; it does not relabel those values as raw authored FBX
transform or payload evidence. The inventory also records
that it bakes takes, normalizes coordinate and inheritance semantics, generates
some missing data, truncates/renormalizes influences, counts rejected invalid
or unrepresentable influences, triangulates, welds, and cannot prove raw
payload spans. Consequently neither current
load-`Document`-write route qualifies as a
preservation-proof frontend for these operations without the raw capability
preflight and explicitly bounded writer work. Unknown extensions or extras,
unsupported morph semantics or unsafe morph storage, cameras, lights, collision metadata, non-triangle primitives,
unmodeled vertex attributes, secondary influence payloads, malformed or
missing inverse binds, node transforms outside the glTF 2.0 contract, and image
payloads sharing bytes with a rewritten accessor fail closed. The FBX
inventory is evidence of why the current frontend remains refused, not an
enablement signal. A later FBX producer may enable only an operation whose
complete inventory/write/proof boundary it satisfies; it must state that FBX
curves were baked, not preserved as authored curves.

Source validity is decided once, at the preflight, and not re-derived per
operation. The preflight's byte-disjointness inspection therefore ranges every
accessor, whether or not a mesh, skin, or animation sampler references it, plus
every image view. An unreferenced accessor contributes its dense base when it
has one and the packed spans of its sparse indices and values; those bytes are
preserved source rather than expendable payload. A referenced sparse accessor
is still refused outright before disjointness matters, and extension-defined
consumers are unreachable while every extension is itself a refusal. Every
unreferenced span may sit adjacent to or alias other preserved data, but any
overlap with a rewritten accessor refuses with both owners located. Each
operation keeps its own guard as defence in depth, since the guard is what must
hold if the gate is ever relaxed, but it shares the gate's classifier rather
than re-deriving one.

The core typed ledger is also the semantic authority for both raw glTF scale
writers. Before binding raw fields, the adapter replays the plan's complete
structural inventory against the current normalized document; numeric-only
replay remains valid, while a stale payload, field, topology, or obligation
inventory refuses. One numeric-free glTF binding then projects canonical
source topology and field dispositions onto raw node members, animation
outputs, mesh payloads, and skin slots. The binding independently walks
`/nodes/*/children` and refuses disagreement with canonical topology before
writing bytes. Writer and artifact proof share those validated identities and
component-group names, but independently select components and derive numeric
multipliers; neither consumes the other's expected values, matrices, factor
helpers, or domain model. The core ledger never replaces raw claims over
scenes, materials, default scene, references, unknown payload, aliasing, byte
ranges, or container bytes.

### D.5 Separate-clip compatibility

Exact bone-name matching is necessary but insufficient. Before transforming or
remapping a separately supplied clip, the producer compares a versioned
skeleton-basis record containing:

- named parent topology and target node paths;
- finite local rest matrices, rest orientations, and helper-node layout;
- declared/normalized coordinate convention;
- effective uniform scale at every translation-track target;
- selected reparameterization domain and factor; and
- loader/tool identity plus the exact input-byte digest.

Compatibility requires topology and orientation agreement within their
declared rules and the analytically expected translation-basis relationship.
Different rest translation bases, parent scale, parent rotation, helper layout,
or effective factor reject before remapping. A digest identifies the evidence
record but does not replace tolerance-aware semantic comparison.

Character-assembly recipe/evidence v4 implements the base record and
comparator; v5 composes it with canonicalization, grounding, and node removal.
V6 admits the narrow normalized/baked FBX projection while leaving both older
contracts immutable. It captures the exact base and clip bytes once, performs
the format-specific capability preflight and plan before remapping, and rebases
each compatible clip's translation values and both `CUBICSPLINE` translation
tangents in the source basis. Evidence pins every input digest and basis
fingerprint together with the compatibility result; v6 additionally binds the
input container, projection kind, full FBX inventory, and private staged-GLB
identity. V1 through v3 remain immutable and v3 rejects the v4 block as unknown.
V7 generalizes the core compatibility basis around selector identity derived
for each accepted plan and source document.
Indexed mode retains v6's exact format-local indices, while named mode compares
the exact resolved root and ordered selected-skin joint names plus semantic
source/helper parent paths. Thus independently captured inputs may use
different source-array indices without weakening topology, rest, orientation,
or factor checks.

### D.6 Proof, evidence, publication, and rollback

The candidate is serialized before it is proved, and the proof runs **once**,
on the document reloaded from the exact artifact bytes — never on a separately
built in-memory candidate, which would prove a document the rewriter did not
emit. That single pass has two layers over the same reload: the
normalized-document layer, which produces the residuals below, and the
artifact layer, which re-derives directly from the container bytes every claim
the normalized model cannot represent. Publication carries the proof to the
file by digest, not by a second proof: the bytes are staged, read back, and
refused unless their SHA-256 equals that of the proved bytes. A second proof
would be redundant, the proof being deterministic over byte-identical input.

The artifact layer's claims are stated over raw source bytes — preservation of
every raw JSON location and every buffer byte outside the rewritten set, array
and index identity, honest reporting of the rewritten locations, container
framing, bounds consistency, and single-step narrowing — and presuppose a
rewriter that edits the source container in place. A frontend without one,
which rebuilds the file from the normalized model as an FBX writer would,
cannot make these claims and is **not** thereby exempt from the obligation
they discharge: that declared-unaffected payloads are unchanged. Such a
frontend owes that obligation by the other route — a complete loader-supplied
inventory of every §D.4 domain — and may enable an operation only over the
domains that inventory covers.

The exact raw-JSON-preservation refusal is diagnostic evidence as well as a
refusal: the current scale-evidence v4 record carries up to the first 16
deterministic differences, each as a pointer and an added/removed/changed
direction, plus the omitted remainder; the full count is the retained length
plus that remainder. The field is null when the exact-preservation walk did
not supply locations for a refusal; it is not an alternate spelling of the
capability preflight's violations.

For the glTF producers, that raw complement explicitly includes scene
membership, default-scene selection, materials, and their references: neither
scale operation owns a write location in those payloads, so the artifact layer
proves their raw JSON values exactly unchanged. The normalized core proof does
not also deep-compare `assets.scenes` or `assets.materials`; those model values
cannot replace the raw fields and references the container proof owns. A
rebuilding frontend therefore cannot cite the core proof for those domains
and must discharge them through the complete inventory route above before
enabling an operation.

The proof evaluates rest, every key time, and analytic or sufficiently bounded
interior times for cubic segments. For reparameterization it must prove,
within versioned tolerances:

- equal world joint translations and orientations before/after;
- equal sampled animated trajectories, root motion, and derived velocities;
- equal skinned vertex positions and bounds for analytic one-joint and
  multi-joint fixtures plus deterministic sampled production evidence;
- unchanged mesh/material/skin identity and declared unaffected payloads —
  including that each candidate mesh instance draws the same mesh, binds the
  same skin joints, hangs off the same node, and names the same source node as
  its source counterpart, none of which either operation rewrites, and that a
  skin *outside* the affected closure keeps each slot's effective inverse bind.
  Identity here covers *placement*, not only what an instance is: the node is
  what positions an unskinned prop, and the source node is what resolves the
  skin whose absent bind accessor stands for the format-defined identity
  default. Both are compared positionally, so the instance list must also
  agree in length and order. Candidate proof also compares normalized parent
  topology and source-projection coverage exactly for every operation. When
  coverage is `Complete`, the raw-node parent/bone map is exact too; rows under
  `Unavailable` coverage are non-authoritative, produce no raw topology or
  field rows, and are not compared. Neither
  operation rewrites any authoritative identity. For rest/bind, the compiled
  ledger includes an `ExactUnchangedWorldRest` obligation for every bone in the
  exact unaffected complement and proof compares each complete composed
  world-rest affine exactly. The discriminating case is an
  independent sibling or leaf outside the closure's ancestry; an unaffected
  ancestor was already constrained transitively by an affected descendant's
  world-rest residual. This is an exact semantic placement invariant, not a
  tolerance or a claim that equal matrices prove every stored local field was
  retained; exact local ownership is a separate field-ledger obligation.
  Whole-document conversion has no unaffected complement, but its typed global
  topology obligation remains in force. Before either operation builds or
  proves, it recompiles the supplied source and requires exact equality of the
  operation variant, canonical source topology, payload and instance
  inventory, field dispositions, and derived obligation rows. Added, removed,
  reordered, or retargeted positional payloads (including stable-order source
  skins, attachments, tracks, meshes, instances, slots, or evidence) therefore
  make the plan stale even when the former aggregate boolean inventory would
  have been unchanged. Canonical source-node and connector topology is keyed
  by raw source identity, so merely reordering its backing rows is not a
  topology change. Numeric values may still change when that exact inventory
  and ledger remain equal; replay does not freeze the planned source snapshot.
  The shared input/candidate checks, source-skin structural rows, canonical
  source topology and rest/bind domain derivation, world-pose and inverse-bind
  readers, and one-time affected-skin classification live in
  `crates/animsmith-core/src/scale/validation.rs`. Operation selection, both
  planners, typed field/payload/obligation ledger compilation, clip-evidence
  declaration, and complete numeric-value-free replay live in private
  `crates/animsmith-core/src/scale/planning.rs`. Planning consumes the shared
  validation/topology boundary and remains the single owner of the compiled
  plan vocabulary; validation does not depend back on planning or reconstruct
  a second plan vocabulary. The opaque candidate wrapper, feature-gated
  analytic reference builder, and writer-owned direct/connector source-local
  rebase arithmetic live in private
  `crates/animsmith-core/src/scale/reference.rs`. That module consumes the
  compiled plan and shared validation/numeric leaves; proof derives its own
  connector products, expected source locals, and scale-track boundary rather
  than importing the reference writer's expectations. Those proof-owned
  derivations, exact field discharge, sampled work and budget accounting,
  semantic residuals, and skin/bounds checks live in private
  `crates/animsmith-core/src/scale/proof.rs`. Its paired residual recorder is
  nested under `proof::residual`, so only proof can mutate a residual maximum
  and its comparison count; the public read-only values continue to resolve
  through the `animsmith_core::scale` facade.
  For an admitted connector span, typed obligations require every connector
  local to remain bit-exact and independently derive the projected successor's
  expected bridged source local from canonical source topology. The connector
  owns no normalized residual, but a writer and proof cannot agree on a wrong
  connector rebase by sharing the writer's product calculation. Rewritten
  direct rewritten source-local components compare under the `appendix-d-v6`
  scalar tolerance, rather than bit equality: a raw adapter may apply the same
  factor before its one `f32` narrowing while the normalized source view was
  already narrowed. Preserved source-local components retain their exact-bit
  obligation. A connector-bridged projected successor also remains bit-exact:
  that core-only connector path does not cross a raw frontend narrowing
  boundary, and its exact connector-projection contract is unchanged.
  Operation-fixed rewrite and tolerance policy fields are not source
  inventory. Numeric affine/factor classification is not repeated there, so
  proof's normalized-skeleton witness remains independent of planning's
  raw-projection witness; a structurally stale plan still cannot omit a newly
  added bone, connector, or payload from every proof walk. The
  unaffected-bind comparison is over each slot's effective bind, in the
  resolution order the model defines: per-instance array, then the bone
  convenience value, then the format-defined identity only when a complete
  attached source skin declares its inverse-bind accessor `Absent`. When both
  sides resolve, the effective matrices are compared even if their stored
  representations differ. Exactly one side resolving is missing proof
  evidence; neither side resolving is reported as out of the proof's scope,
  never as proven — a document carrying an unrelated skin with no bind
  evidence and no licensed identity default is not an operation this record
  touches, and must not be refused for lacking evidence about it;
- unit composed scale for every affected node, measured per axis against the
  derived bound of §D.1; and
- the analytically expected full world affine of a transform-only attached
  child: preserved origin/orientation and unit linear scale, so a no-op cannot
  pass.

For whole-document conversion, the corresponding length facts must differ by
exactly the declared factor within tolerance while dimensionless facts remain
equal. The raw glTF artifact proof independently walks admitted morph
`POSITION` accessors, requires one conversion per unique accessor (including
base/morph and cross-target aliases), compares every static JSON morph-weight
numeric value, and compares every animated weight accessor byte exactly. Both
operations prove finite output, the skin equation, deterministic artifact
bytes, and deterministic evidence bytes.

**Every obligation above is derived from the evidence it needs.** The typed
ledger contains a row only when the validated planned inventory carries the
payload that row reads — a transform-only attachment in the closure for a
full-affine probe, an affected translation track for key-time checks, an
affected cubic segment for interior samples, any affected track for sampled
trajectories, a skinned instance touching the closure for one shared skinned
evidence group, and a non-empty affected closure for the inseparable
rest/unit-scale group. Skin-matrix and bounds results remain separately
reported projections of that one skinned group; neither can be independently
armed, and unit scale cannot exist without the affected-rest obligation.
A row whose evidence disappears from a replayed source changes the recompiled
ledger and yields typed `PlanDocumentMismatch`, never a zero residual. A
counterpart or value missing inside an inventory-matched walk is typed
`MissingProofEvidence`. The residuals *are* the evidence record, so a record
stating "residual 0.0" for a claim nothing checked is false rather than
missing. This matters most for the transform-only obligation, which is the
claim ensuring a no-op cannot pass — a guarantee an empty probe loop reporting
zero would not provide.

Per-element track values, base mesh positions, and inverse binds outside the
affected closure are ordinary typed obligations rather than three special
ungated loops. Their rows are derived for every corresponding planned
container. For normalized `Bone::rest`, inverse binds, tracks, and meshes,
proof discharges ownership through the existing versioned residual or exact
semantic obligations, independently deriving rewritten expectations from the
operation. A normalized candidate may therefore be non-bit-identical while
remaining inside the declared tolerance; field-row ownership neither replaces
nor tightens that accepted set. Rewritten and preserved track/mesh comparisons
retain their released maxima and counts. Core bit equality applies only to
authored `SourceNodeRest` locals that every admitted producer copies, including
preserved connector locals. Exact structural obligations such as topology,
connector projection, and unchanged world rest own no numeric residual. Raw
byte-exact preservation of other fields — including signed zero and quaternion
sign — remains the artifact proof's PR-B responsibility. No separately mutable
boolean controls any of these walks.

Proof cost is bounded, not merely expected to be small. Source and candidate
world matrices are derived once per sample time and shared across the
trajectory, skin, and bounds obligations, and the skin and bounds obligations
share one walk over the affected instances and their vertices. The remaining
work is `sample_time_count * per_sample_work_units` with the per-sample cost
spelled out in §D.1 — bones, skin slots, and skinned vertices, each charged
once per document side — which the tolerance policy of §D.1 caps. A document
above that cap is refused with a
typed error naming the policy identity, the sample count, the per-sample cost,
the computed work, and the budget. It is never proved against a truncated
sample set: a bounded proof of a subset would publish evidence for claims it
did not check. Because the budget lives in the policy identity, it is not a
per-run flag, and the producer's fixed-policy rule below covers it.

The glTF/GLB producer emits immutable scale-evidence v4 containing
the operation kind, declared and observed factors, tolerance policy and
residuals, affected node/skin identities, raw capability manifest, input/output
byte digests and counts, proof sample coverage/results, tool identity, and
rejection reason. It is written as an
atomic artifact/evidence publication pair. Failure leaves no new output; an
existing pair is restored. The original input is retained so rollback is
selecting the prior artifact, not attempting an inverse float rewrite.

**Narrow FBX rest/bind path.** With the ufbx feature, `scale rest-bind` may
accept an `.fbx` input only when its complete inventory proves the normalized
rest/bind subset: adjusted metre/Y-up coordinates, compensated inheritance,
complete finite bind provenance, unchanged four-slot influences, and no
unsupported scale-bearing semantic D.4 row. Same-load source facts may prove
that user-defined properties, bounded texture/video declarations, or exact
marker, LOD-group, stereo-camera, camera-switcher, and display-layer typed
lists are known, non-scale-bearing inputs to this normalized bridge. Display
layers contain only node membership and editor visibility/freeze/color state.
Shader and binding-table metadata is equally non-bearing. BindPose rows are
admitted only after the same-load converted matrices reconcile with the
cluster/node matrices the bridge consumes; no Pose is required. Other unmodeled typed lists remain
fail-closed and are reported by exact nonzero kind counts. The same-load
boundary may also classify an exhaustive set of conversion-fidelity facts as
scale-invariant: omitted authored vertex/face/edge metadata, influence
truncation/rejection/renormalization with complete effective coverage,
triangulation, and exact-bit welding. The frozen inventory continues to report
those losses and its detached projection remains conservative. Missing
effective influences, omitted point/line or mesh definitions, unsupported
skinning/deformers, incomplete binds or normals, and unclassified payload stay
fail-closed. The same-load
`FbxScaleSource` retains the raw facts and dependency closure, while v5 keeps
the frozen inventory projection rather than adding either sidecar to its wire
shape. The three raw-span rows are deliberately
unverifiable because FBX is not rewritten. The frontend serializes the
normalized document to a private GLB, maps the selected root and ordered skin
joint topology by exact unique normalized names, rewrites that GLB with the
existing raw glTF writer, reloads and semantically proves the exact candidate
once, read-back-digest checks it, and atomically publishes the resulting GLB
with immutable scale-evidence v5. V5 records the complete FBX inventory and
the private staged-GLB identity plus the nested raw-GLB proof; it makes no raw
FBX span, object-property, authored-curve, material, or texture-assignment
preservation claim, nor a marker/camera/display-layer behavior claim; supported
linked texture bytes are nevertheless captured before staging so admission cannot
silently remove them. Refusals name the exact incomplete coverage domain,
semantic row, inventory counter, or residual unmodeled kind.

**Two observed factors, and the divergence between them.** For rest/bind
reparameterization, "observed factor" names two independent witnesses of the
same quantity, measured from deliberately different state: planning classifies
the raw source projection's node-local rests composed through the raw parent
chain, and proof reads the normalized skeleton's bone rests composed through
its own parent chain. For whole-document conversion, both fields are the
declared factor by definition and their divergence is exactly zero; that
operation does not classify scale from either hierarchy.
The raw composition includes every preserved static connector local. The
normalized witness reads the corresponding folded transform through the next
projected bone; the connector itself has no normalized identity and therefore
owns no residual obligation. After semantic residuals succeed, proof's typed
field-row discharge separately checks exact connector preservation and the
analytic bridged source local at each projected successor. This preserves the
released residual-error precedence while keeping both the connector's
preserved authored components and its bridged projected successor bit-exact.
Direct rewritten raw rows outside a connector bridge use the published scalar
tolerance at the frontend narrowing boundary described in the compiled-ledger
contract above.
Strict scale-input shape validation requires the two chains to *agree* — under
`Complete` source-skeleton coverage, which this operation requires anyway, a
projection that contradicts its own skeleton is refused before either witness
is taken — but nothing reconciles the two *readings*: each is still composed
from its own stored state, and that independence is the reason both witnesses
exist. The record therefore carries **both**, plus the relative divergence
`|planned - proved| / max(|planned|, |proved|)` between them explicitly, so a
consumer neither mistakes one witness for the other nor has to derive the
relationship from two separate policy fields.

The expected ceiling on that divergence is **the sum of two bands the policy
already declares**: the common-factor band plus the postcondition unit-scale
residual (`1e-5 + 2^-14 = 7.103515625e-5` under `appendix-d-v6`). Planning
binds its witness to the declared factor within the first band or refuses;
and for a candidate this operation built from the source under proof, that
candidate's composed root scale is the proof witness divided by the declared
factor, so the unit-scale postcondition binds the proof witness to the
declared factor within the second. The two bands are not stated the same way,
and the sum is a ceiling only up to that difference: planning's is relative to
the `max` of its two operands, exactly as the divergence is, while the
postcondition's is an absolute L∞ deviation from `1` on the *candidate's*
composed scale — so it bounds `|proved − declared|` as a fraction of the
declared factor rather than as a fraction of `max(planned, proved)`.

The divergence is **reported, not enforced**. The second step above holds for
a candidate this operation built from the source it is proved against — which
proof deliberately does not require — and it costs the binary32 rounding of
the rebase on the way, so the sum is the ceiling the design guarantees rather
than a bound proved to the last ulp; refusing at it would refuse documents
whose two witnesses each honour their own band. A divergence beyond it is not
evidence that the two parent chains disagree — those are validated to agree,
above — but that the state each witness reads differs: most often the raw
node-local rests against the normalized bone rests, two separately stored
descriptions of one rest pose. How far apart those are is a fact about the
input, not a residual this proof owns. Recording both witnesses and
their divergence introduces no band of its own and no new policy identity.

Migration is opt-in. `assemble.canonicalize_skin` in recipe v3 remains the
existing unanimated bind-geometry operation with identity source-to-metre
conversion; it does not gain rest-scale rewriting, and old recipes are not
silently reinterpreted. Recipe/evidence v4 adds the optional, explicit
`rest_bind_scale` operation and basis-compatibility evidence. Consumers that invert scaled
inverse binds with the rigid-only shortcut `-A^T t` can observe a change by
`s^2`; that exposes a pre-existing consumer error because the shortcut is valid
only for an orthonormal linear part, not a geometry change made by this
operation.

### D.7 CLI, configuration, and public API

The CLI uses a required operation subcommand, not one flag whose
meaning depends on the input:

```console
animsmith scale whole-document INPUT.glb -o OUTPUT.glb \
  --factor 0.01 --evidence OUTPUT.scale.json

animsmith scale rest-bind INPUT.glb -o OUTPUT.glb \
  --source-skin-index 0 --source-root-node-index 3 \
  --expected-factor 0.01 --evidence OUTPUT.scale.json
```

All numeric and source-identity arguments shown are required; there is no
inferred factor, implicit first skin/root, or in-place mode. Artifact and
evidence paths must be distinct from each other and the input. The narrow FBX
path additionally refuses either destination when it names a retained safe
dependency key, and refuses publication entirely when that key was rejected as
symlink-mediated or the dependency closure stopped before enumerating every
key. Initial support is self-contained glTF/GLB for both
operations, plus the narrow inventory-complete FBX `rest-bind` path that emits
a `.glb` and v5 evidence.
The command uses the fixed tolerance-policy version recorded in evidence rather
than accepting per-run tolerance flags. A future policy change requires a new
policy identity and compatibility review.

The single-document producer has no `animsmith.toml` key and no separate plan
file: mutation must not become an incidental effect of a lint configuration.
Assembly recipe v7 exposes the same rest/bind operation through an optional
`[rest_bind_scale]` block whose exact `root_node_name` and factor are required.
The name rejects leading or trailing whitespace instead of normalizing it.
For every captured base and clip input, that name must resolve to exactly one
normalized source node and exactly one non-empty source skin whose every joint
is that node or its descendant. The resolved per-input source indices feed the
unchanged core operation and private staging bridge; no cross-file source-array
index is inferred or reused. It accepts glTF/GLB plus the narrow FBX inventory
admitted by the
standalone rest/bind boundary, validates every base and clip basis before
applying the operation or remapping keys, and records the declared name,
resolved per-input name/indices, and normalized/baked FBX projection rather
than claiming raw FBX curve or span preservation. Recipe v6 remains immutable
with its explicit source indices, recipe v5 remains glTF-only, and recipe v3
has no such block and continues to reject it as unknown.

The public `animsmith-core` shape is a non-exhaustive `ScaleOperation` with
distinct `WholeDocumentLinearUnits { factor }` and
`RestBindUniformScale { source_skin_index, source_root_node_index,
expected_factor }` variants, carried by a `ScaleRequest`. Pure planning returns
either an opaque, non-exhaustive `ScalePlan` (operation variant, affected
closure, canonical topology, typed field rows, tolerance policy, and derived
obligations) or a typed `ScaleError`; proof returns a
`ScaleProof` with one read-only `ScaleProofResidual` per semantic claim. Each
value carries the maximum and number of comparisons together, so a residual
nothing walked is distinguishable from a measured zero (issue #319) and an
adapter cannot combine one claim's count with another claim's maximum (issue
#323).
Planning and proof take format-neutral node, track, bind, and capability facts.
They do not accept paths, glTF/ufbx types, config parsers, or publication
policy. The format frontend owns raw inventory and exact source rewriting; the
CLI owns atomic artifact/evidence publication. Applying a plan builds a
candidate and never mutates the caller's source on failure.

`animsmith-core::scale` implements this contract (issue #281): the
`ScaleOperation`/`ScaleRequest` planning entry point, the typed `ScalePlan`
and `ScaleError`, the no-authority candidate handoff, and paired `ScaleProof`
residual values. Any change to the operation variants, required selectors, or failure
boundary is a design change rather than an implementation detail.

`ScalePlan` exposes its compiled ledger only through non-exhaustive read-only
field, payload-shape, topology, and obligation row views; callers cannot
use those row values to construct or mutate a plan with an incoherent
operation, topology, field inventory, or obligation set. Plan
equality includes that exact structural ledger. The former public
`ScaleDomainRewrites` and `ScaleProofObligations` boolean bags and their plan
accessors are removed in this pre-1.0 API break: operation-fixed facts and
evidence gates have one typed authority rather than independently selectable
booleans. The CLI privately projects the same five `domain_rewrites` booleans
from the operation variant's frozen mapping, so payload presence and
factor-one rows cannot change their meaning. Immutable scale-evidence
v1/v2/v3, `ScaleProof` serialization, `appendix-d-v6`, tolerances, calibration
semantics, and policy pins do not change. The internal ledger is not serialized
into evidence.

`ScalePlan` and `ScaleProof` each carry the observed factor beside the
declared one, as §D.6's evidence contract requires (issue #290). The declared
factor is what the build applies and what every proof expectation is stated
in; the observed factor is the rest-world uniform factor measured at the
scaled root, which the declared-factor band admits without requiring equality.
Proof re-measures it from the source it is handed rather than reading it off
the plan, so evidence does not depend on planning having recorded it. It is
reported, not checked: the input band is planning's obligation and the
postcondition derived from it in §D.1 is proof's. For a whole-document
conversion the observed factor is the declared one, because §D.1 gives that
operation's factor no measurable source counterpart at all.

`animsmith-gltf` owns the glTF/GLB frontend of that contract:
`preflight_scale_source` (issue #280) captures the raw inventory,
`capability_facts` / `rewrite_linear_units` / `prove_rewritten_artifact`
(issue #282) implement the whole-document conversion of §D.2 directly on the
source's own JSON and buffer bytes, and `rewrite_rest_bind` /
`prove_rewritten_rest_bind` (issue #283) implement the rest/bind
reparameterization the same way. Because that route produces a candidate
core did not build, `ScaleCandidate::from_document` exists so the reloaded
artifact can reach `prove_scale`; the type asserts nothing that `prove_scale`
does not independently re-derive. Core exposes no production candidate
builder: its analytic reference constructor is available only through the
non-default `fixtures` feature for tests and calibration.

`rewrite_rest_bind` is the selector-taking convenience boundary; it compiles a
plan and delegates to the same plan-taking raw adapter used by
`rewrite_scale_plan`. The latter validates the plan against the supplied
source before any write, so the CLI can carry one authoritative compiled plan
through rewrite and proof without admitting stale-plan replay.
`GltfScaleArtifact` reports which operation produced it
beside the declared factor, because the two operations rewrite different
domains and a factor alone does not distinguish them. Beyond the shared
plan/proof rejections, the frontend owns three refusals the format-neutral
layer cannot state: the disagreeing-multiplier refusal of §D.2, and two
agreement checks over the raw node hierarchy, the loader's source-node
projection, and the normalized skeleton's parent links. `animsmith-core`
requires the last two of those three to describe the same tree — that is the
document-shape validation of §D.6 — but the raw `/nodes/*/children` arrays
live in JSON that never becomes a `Document` field, so bringing the third
description into the comparison is only possible here.

### D.8 Implementation ownership and deferred integrations

Putting all semantics in assembly was rejected: it would hide a reusable
single-document operation and preserve assembly's currently insufficient
separate-clip proof. A standalone command with unrelated duplicate math was
also rejected. The chosen end state is one shared core transform-plan and proof
layer with distinct, explicit frontends: the shipped single-document producer
and character-assembly recipe/evidence v7 integration. The existing
`canonicalize_skinned_bind_pose` remains the narrower unanimated bind-geometry
foundation; it is not silently widened or renamed into this contract.

**Shared affine classification, distinct acceptance contracts.** The core uses
one typed, symmetric `f64` positive-uniform `Mat3` classifier for both Appendix
D and skinned bind-pose canonicalization. Appendix D retains
`appendix-d-v6`'s `1e-5` equal-axis/orthogonality and `1e-6`
relative-determinant thresholds. The canonicalization input
`source_to_meters_y_up` retains its numeric `1e-4` equal-axis/orthogonality
thresholds and exact-zero determinant threshold; these are compatibility with
the existing assembly/canonicalization contract, not a second scale-policy
version.

Canonicalization does deliberately adopt the shared classifier's semantics:
axis equality is relative to the three-axis average and longer operand rather
than to the first column, derived arithmetic is `f64`, and singularity is
classified before non-uniformity or shear. Thus a unit basis with one axis at
`1.00015` is accepted while `1.00016` is refused, and an input that is both
exactly singular and sheared now uses the existing
`reflection_or_singular` reason rather than `non_uniform_or_sheared`. The
machine-readable vocabulary is unchanged; these symmetric boundaries and
precedence replace two independent algorithms with one explicit typed policy.
The classifier runs after canonicalization's separate existing
zero-translation/fourth-column `w_axis ~= Vec4::W` gate and its first-axis
`<= 1e-4` `zero_scale` reason. That zero-scale gate reads the same widened
`f64` axis length as the classifier, so a finite large basis cannot overflow a
binary32 length intermediate and be mislabeled as zero scale.
Near the shear boundary, the symmetric average base and binary64 dot products
can also refuse inputs that the former first-column/binary32 check accepted.

**Measurement shares facts, not the positive-only verdict.** Measurements v12
derives axis lengths, their canonical ascending-order mean, determinant,
axis-length product, and XY/XZ/YZ dot products from the same crate-private
widened `f64` fact record. It also uses the shared symmetric
mean/longer-operand equal-axis predicate, which removes v11's privileged-X
result and reconciles the finite
`diag(1, 1, 1.000012)` fixture. Measurement still asks a different question:
it keeps finite partial evidence for singular, reflected, sheared, and
non-uniform inputs; normalizes each orthogonality comparison by that axis
pair's length product; and classifies in singular, reflected, sheared,
unit/uniform, then non-uniform precedence. Calling the positive-uniform
classifier directly would discard those descriptive facts and impose the
operation's different precedence. Skin-bind summary factors are sorted before
their mean is derived and every factor is compared to that mean, so authored
joint order cannot choose `consistent_uniform` versus `mixed_uniform` or the
reported common factor. These corrected numeric semantics require the nested
measurements-v12 identity. The JSON shape remains unchanged, but the outer
identity advances to output-v6 because each immutable output schema statically
pins its nested measurement URN; measurements v11 and output-v5 remain
immutable historical evidence. In general, changing a nested identity that an
immutable outer schema references requires a new outer identity too; historical
outer schemas are never retargeted or broadened in place.

Measurements v13 makes inverse-bind derivation auditable and fail-closed. Each
per-joint bind observation retains the exact finite raw inverse-bind matrix for
its accessor slot. Before publishing an inverse, the source bottom row must be
within absolute `1e-6` of affine `[0, 0, 0, 1]`; this admits several binary32
round trips around one but refuses projective translation columns. The linear
3x3 reports the scale-free reciprocal infinity-norm condition number
`1 / (norm_inf(A) * norm_inf(inverse(A)))`. Exactly singular sources and
finite sources at or below `1e-6` use distinct `non_invertible` and
`ill_conditioned` unavailable reasons. This avoids the prior scale-bearing,
exact-zero determinant decision: a uniformly small but well-conditioned matrix
is assessed by its error amplification rather than its determinant magnitude.
The new nested shape advances the immutable outer envelope to output v7;
measurements v12 and output v6 remain historical.

Measurements v14 gives every clip fact that is not applicable to every clip —
loop continuity, loop endpoint mode, frame grid, loop seam ratio, gait (and
its own phase), and root-motion speed — a required sibling
`<field>_availability` status of `measured`, `not_applicable`, or
`unavailable`. A bare optional value cannot distinguish "this clip has no
subject for the fact" (`not_applicable`, a legitimate absence) from "the
subject applies, but derivation failed" (`unavailable`, which a consumer must
fail closed on rather than silently treat as passing); those two absences now
compare distinctly under `diff` as well. The value field is present if and
only if its status is `measured`. The new nested shape advances the immutable
outer envelope to output v8; measurements v13 and output v7 remain
historical.

Measurements v15 inventories each clip's surviving non-empty local TRS
channel set by skeleton index and adds normalized model-space root-trajectory
regression evidence. A resolved Root always wins; Hips is a typed fallback
only when Root is unresolved. The record retains independent translation and
yaw availability, signed endpoint displacement, sampled horizontal travel and
vertical extrema, and fixed-witness net/unwrapped/travel yaw. All trajectory
path/extrema/winding values come from the shared inclusive uniform metric grid
and therefore do not prove continuous-curve extrema or authored transform
preservation. The new nested shape advances the immutable outer envelope to
output v9; measurements v14 and output v8 remain historical.

The implementation status is:

- shipped: raw glTF/GLB capability preflight, the shared core plan/ledger and
  proof, preservation-safe whole-document and rest/bind exact-source writers,
  artifact proof, and atomic CLI publication with scale-evidence v4;
- shipped: issue #286's conservative FBX rest/bind re-encode path with the
  full ufbx inventory, exact staged-GLB rewrite/reload proof, v5 evidence, and
  atomic GLB/evidence publication; whole-document FBX scaling remains refused;
- shipped: glTF/GLB and inventory-complete normalized/baked FBX assembly
  integration in recipe/evidence v6, including the
  versioned basis fingerprint/comparator, pre-remap clip rebasing, exact input
  digests, compatibility evidence, and proof over the exact staged artifact;
- shipped: recipe/evidence v7 exact-name rest/bind selection, with a unique
  source-node and fully-governed-skin resolution for every captured input and
  explicit declared/resolved selector evidence (issue #459);
- shipped: issue #286-A's explicit conservative ufbx-side status for every
  current D.4 domain and documented normalized source-skeleton projection;
- shipped: raw glTF whole-document scaling for dense `f32` morph `POSITION`
  deltas with numeric-value preservation for static JSON weights and byte-exact
  preservation for animated weight accessors (issue #298);
  rest/bind morphs and a normalized core morph model remain out of scope.

The live implementation sequence is owned by tracker issue #344 and roadmap
issue #165, not by a numbered future-slice list in this decision record. Work
beyond this positive-uniform, fail-closed decision remains ADR-gated.

## Appendix E — decision record: engine profiles

This appendix is the accepted design outcome of issue #150. It defines the
consumer-model boundary required by the 0.4.0 engine-feedback milestone; it
does not claim that the configuration, checks, or generators described here
already ship.

### E.1 Decision and ownership

An **engine profile** is a versioned set of externally verified importer or
runtime facts. It answers *how a named consumer handles a supported input*.
It is not any of the following:

- a rig profile, which resolves semantic roles to bones;
- project intent, such as whether a clip should be in-place, looping, or
  animation-driven root motion;
- a severity or artistic-quality preset;
- evidence that a particular engine import actually ran; or
- authority to rewrite an asset.

Those distinctions are structural. `[rig]` continues to own role resolution,
`[runtime_nodes]` owns the shared selection of project-declared attachment,
socket, IK, collision, and other runtime-facing nodes, `[clips]` owns per-clip
intent plus explicitly nested clip-scoped importer choices, `[checks]` owns
activation, severity, and project tolerances, engine readback owns observed
import results, and Appendix D plus each transform's own proof contract owns
rewriting. An engine profile may describe a hard importer constraint or
interpretation, but it must not silently change the math, thresholds, or
findings of an existing consumer-neutral check.

The issue's original suggestion that profiles own loop/contact thresholds is
therefore rejected. An engine may expose a loop toggle or a compression
setting; it does not know a project's acceptable seam, contact, or slide
error. Generated guidance may propose a documented starting value, but the
effective value remains explicit project policy.

### E.2 Selection and built-ins

The first configuration surface is deliberately small:

```toml
[engine]
profile = "unity-generic"
profile_revision = 2
engine_version = "6000.3"
importer = "fbx-model-importer"

[engine.settings]
animation_type = "generic"
avatar_setup = "create_from_this_model"
import_animation = true
root_motion_source = "Reference/Root"

[clips."locomotion_*"]
movement_owner_xz = "animation"
movement_owner_y = "gameplay"
movement_owner_yaw = "animation"

[clips."locomotion_*".engine_settings]
root_rotation = "extract"
root_position_y = "bake"
root_position_xz = "extract"

[clips.idle]
movement_owner_xz = "gameplay"
movement_owner_y = "gameplay"
movement_owner_yaw = "gameplay"

[clips.idle.engine_settings]
root_rotation = "bake"
root_position_y = "bake"
root_position_xz = "bake"

[runtime_nodes]
selectors = ["weapon_socket", "ik_*", "collision_anchor"]
```

`[runtime_nodes].selectors` is project policy, not an importer fact. Exact
names and `*` globs resolve with the deterministic miss and ambiguity behavior
already established by issue #268. The existing
`[checks.rest-world-scale].node_selectors` field remains accepted as a
compatibility alias and is normalized into this shared selection before
evaluation. Declaring both forms is a typed configuration error rather than a
precedence rule. `rest-world-scale` and the selected-node facet of
`engine-unit-scale` consume the same resolved set; one check never reads the
other check's private settings. An absent selector field and an explicit empty
selector list both mean no declared policy, preserving the legacy check's
behavior. Only that state makes the selected-node engine facet genuinely not
applicable. If a
declared selector misses or is ambiguous, `rest-world-scale` retains its
existing engine-neutral coverage behavior, while an enabled, otherwise
applicable `engine-unit-scale` facet emits `required_prediction_unavailable`
and follows E.4's exit policy. Component/root and mesh-node unit-mapping rules
may still apply independently.

An absent `[engine]` section means today's engine-neutral behavior. There is no
`generic` engine profile, no auto-detection, no nearest-version match, and no
fallback profile. Family, profile revision, engine version, and importer route
are all required when `[engine]` is present. An unknown or unsupported tuple,
unknown setting, or setting that is invalid for that importer is a typed
configuration error. A caller that still wants ordinary engine-neutral linting
removes `[engine]`; it does not receive a partial prediction under a different
consumer contract.

Every profile setting descriptor declares its exact value domain, applicability,
and either document or clip scope. Document-scoped importer choices live only
under `[engine.settings]`. Clip-scoped importer choices live only under the
matching `[clips.<selector>.engine_settings]` table and resolve through the
existing clip-selector rule (exact name over matching globs, later matching
globs winning ties). For the Unity Generic FBX profile, the Motion/Root Motion
Node choice is an exact source-transform path that applies to all imported
clips and is therefore document-scoped as `root_motion_source`. It is
explicitly not applicable to the Unity Humanoid profile, whose importer uses
Avatar/body semantics. Rotation and position bake/extract choices remain
clip-scoped for both Unity profiles. `[clips.<selector>]` movement-owner fields remain project intent;
the sibling `engine_settings` table records the importer state that a prediction
must compare with that intent. A setting at the wrong scope is a typed
configuration error, not an override.

Each applicable V1 setting is classified as required without a verified
default; values in the example are not defaults. Adding a documented default
requires a later profile revision and settings-contract revision rather than a
silent registry edit. Omitting a required setting is a typed configuration
error; it is never guessed, treated as an implementation default, or deferred
to a coverage gap. The resolved settings record therefore contains one exact
document value or per-clip value for every applicable setting.

The registry contains the preserved revision-1 tuples plus the current
revision-2 Unity Generic, Godot, Unreal, and Bevy tuples and the revision-3
Bevy tuple:

| family | revision | engine version | importer | AnimSmith input boundary |
|---|---:|---|---|---|
| `unity-generic` | 2 | `6000.3` | `fbx-model-importer` | FBX |
| `unity-humanoid` | 1 | `6000.3` | `fbx-model-importer` | FBX |
| `unreal` | 1 | `5.8` | `fbx-importer` | FBX |
| `unreal` | 2 | `5.8` | `fbx-importer` | FBX |
| `godot` | 1 | `4.7` | `resource-importer-scene` | glTF JSON, GLB, FBX |
| `godot` | 2 | `4.7` | `resource-importer-scene` | glTF JSON, GLB |
| `bevy` | 1 | `0.19.0` | `gltf-asset-loader` | glTF JSON, GLB |
| `bevy` | 2 | `0.19.0` | `gltf-asset-loader` | glTF JSON, GLB |
| `bevy` | 3 | `0.19.0` | `gltf-asset-loader` | glTF JSON, GLB |

These are exact versions, not ranges. The Godot row is AnimSmith's bounded V1
profile boundary rather than a claim that Godot cannot import other formats.
The preserved Unity revision-1 advice profiles expose document booleans
`convert_units` and `bake_axis_conversion`, plus per-clip `root_rotation`,
`root_position_y`, and `root_position_xz` values in the closed `bake | extract`
domain. Current Unity Generic revision 2 instead exposes the frozen document
values `animation_type = "generic"`, `avatar_setup = "create_from_this_model"`,
`import_animation = true`, and the exact document-scoped
`root_motion_source`, plus those three per-clip root controls. Only Generic
also exposes `root_motion_source`; it is not applicable to Humanoid. Unreal,
Godot, and Bevy revision 1 expose no V1 setting vocabulary, so every supplied
key is rejected. Bevy revision 2 has a separate closed setting vocabulary for exact
loader coordinate toggles, mesh loading, extension-handler environment,
animation feature, and animation loading. It materializes version-pinned
defaults and retains each value's explicit/default origin.

Bevy revision 3 preserves the earlier tuples and adds only the narrow
animation/channel gate-support fact. It resolves the compiled
`bevy_animation` feature and `load_animations` with explicit/default origins;
the compiled feature gate takes precedence. Its raw same-load inventory binds
animation coverage independently from each animation's channel coverage.
Extensions, other animation constructs, and positive runtime survival remain
outside this revision; the latter requires an actual Bevy readback.
The separately versioned `bevy-gltf-addressability-rules:1` bundle paired with
this immutable profile owns the richer scene, skin, named-map, and target-path
projection; it does not mutate profile revisions 1--3.

The two Unity ids are import modes, not skeleton-name maps. Likewise,
`unreal`, `godot`, and `bevy` do not imply a mannequin, humanoid bone map, or
other rig profile. glTF is not an engine profile: its specification defines
format semantics but deliberately does not define looping, playback, import
options, or other runtime policy. glTF 2.0 compatibility remains a loader and
consumer-neutral check boundary. Bevy has separate supported-extension,
asset-addressability, and animation-target behavior.

Each built-in resolves its exact `(family, profile_revision, engine_version,
importer)` tuple to an immutable fact-bundle identity such as
`urn:animsmith:engine-profile:unity-generic:2`. Its data record contains exact
engine version and importer values, accepted input formats, source
and target coordinate bases, importer/runtime facts, allowed setting
vocabulary, and primary-source references with a verification date. For every
claimed input format it must also enumerate, rather than leave implicit:

- handedness, up/forward axes, and the selected importer's axis-conversion
  mapping;
- each applicable clip-boundary predicate (including whole-frame constraints)
  or an explicit statement that none is known; and
- supported, dropped, and reinterpreted channels, extensions, and animation
  constructs, with an explicit unsupported or unknown state where evidence is
  incomplete.

Changing any such fact requires a new profile revision and normal release
notes; a historical result never silently resolves through the current
revision. `[engine.settings]` selects only choices the immutable importer facts
already expose; it cannot override those facts.

Facts use explicit known, unknown, and genuinely not-applicable states. The
accepted-format fact is the sole input-format authority. The facts digest is
the core `InputIdentity` of a versioned, fixed-field-order, length-prefixed
UTF-8 encoding of the full tuple, URN, every fact state and value (including
accepted formats), every setting descriptor/domain/scope/applicability and
default status, verification metadata, and source references sorted by stable
id. Source references bind stable fact/setting ids to one target version, URL,
and verification date; live documentation bytes are never digest input. The
settings digest uses the same encoding over the full resolved identity and
fully materialized document settings plus actual clip names/settings in lexical
order, never TOML spelling, selector patterns, or map insertion order.

Version one does not accept arbitrary engine-fact overrides in TOML. A user
cannot safely make an importer support a channel or reinterpret root motion by
assertion. Embedders may supply namespaced custom checks and keep their own
consumer contracts, as they do today, without extending the built-in profile
registry.

Crate ownership is fixed before implementation. A new format-neutral
`animsmith-engine` library owns the built-in fact registry, strict tuple
resolution, prediction-rule adapters, and the standalone advisory/readback
payloads. It depends on `animsmith-core`, but not on format libraries, TOML, a
filesystem, or engine SDKs. `animsmith-core` continues to own consumer-neutral
documents, measurements, checks, shared source-evidence value types, every
measure/lint envelope and URN, and a registry-independent wire projection for
profile identity, rule basis, prediction state, and shared provenance. The
engine library populates that core-owned projection; core never depends on the
engine registry or its concrete profile types. The glTF and FBX libraries
populate the common raw-source projection while retaining their format-specific
parsing details. The `animsmith` CLI owns strict TOML mapping and orchestration;
`animsmith-report` renders the resulting public contracts. This preserves
Appendix C's envelope ownership, prevents a core/engine dependency cycle, and
gives embedders the same registry and resolver as the CLI.

Format loaders bind that projection to the normalized document in an opaque,
immutable `LoadedSource`. Its consumer surface is deliberately limited to
`document(&self)`, `source_facts(&self)`, `dependency_closure(&self)`, and
`into_document(self)`: callers cannot mutate a document while continuing to
present its source facts or captured closure as current, and consuming the
document explicitly discards both sidecars. The facts view borrows the
canonical `Document.assets.source_skeleton` projection rather than copying
source-node or skin authority. Existing document-only loader entry points
remain compatibility conveniences that consume this wrapper.

The V1 projection is bound to the exact primary-file `InputIdentity` and an
explicit source format established by the loader; `SourceInfo.path` and its
free-form format label are neither identity nor fact authority. Row sets carry
independent `complete`, `partial`, or `unavailable` coverage. Rows retained
under partial coverage prove positive presence only; absence is established
only by complete coverage. Observation availability, provenance, and AnimSmith
loader disposition remain orthogonal and never imply target-engine support.

V1 bounds projection work to 65,536 enumerable rows, 4,096 clips/takes, 4,096
resource-reference declarations, 4,096 UTF-8 bytes per retained source string,
8 MiB of retained source strings in total, and traversal depth 128. At N+1 the
loader keeps the deterministic source-order prefix (lexical order only where a
parser exposes a map without source order), marks the affected domain partial
with `projection_budget_exceeded`, and stops new work in that domain. A legacy
load that succeeds does not become a failure merely because this sidecar is
partial. These are projection limits, not claims that every existing parser,
resource read, image payload, or bake allocation is globally bounded.

Raw resource facts stop at bounded declaration identity, kind, and a safely
retained source-relative spelling. They do not themselves open, normalize,
deduplicate, hash, or map referenced content. Unsafe absolute, remote,
data-payload, or escaping spellings are classified without being reproduced in
the new facts surface, its debug/serialization views, or its errors. Legacy
diagnostic `Document::source.path` remains outside that projection.

The separate dependency-closure v1 sidecar is exact only over that versioned
declaration domain. Format loaders construct it during the same load that
consumes the resource bytes; a post-load reopen cannot establish the identity
of bytes already parsed. Embedded/data/BIN/view-backed declarations map to the
primary identity. Safe relative declarations map through a normalized
source-relative key to one captured `InputIdentity`; aliases of one key are
opened and hashed once, while different keys remain distinct even when their
bytes match. The source-order reference map and key-sorted distinct-resource
table form a domain-separated canonical closure identity. Only complete raw
declaration coverage with an identity for every declaration produces a
complete closure identity. Unsupported extensions or unmodelled FBX
resource-bearing domains keep the result conservative rather than turning a
known subset into a whole-document or target-importer claim.

Each reference also records a purpose derived from its source kind:
loader-essential buffers, nonessential images/textures, or target-only
video/cache declarations. This is source/loader evidence, not a prediction of
whether a selected engine profile will import the resource.

Closure capture has its own immutable v1 budget: at most 4,096 declarations,
1,024 distinct external keys, 4,096 UTF-8 bytes for each source spelling and
normalized key, 128 path components per spelling, 8 MiB of aggregate
normalization input, 64 MiB read and hashed per external resource, 256 MiB in
aggregate, and one bounded non-quadratic deduplication probe per declaration.
The loader checks N+1 before allocation or opening, retains the deterministic
prefix and work counters, and stops the affected capture. Absolute, escaping,
out-of-root, remote, malformed, oversized, and symlink-mediated locators are
typed refusals and are never opened or reproduced. A trusted resource root
comes from path-based loading or an explicit byte-loader API; byte loading
never falls back to the process working directory. Missing loader-essential
bytes remain a load/operator error, while unavailable nonessential or
target-only bytes may leave typed partial closure. No captured resource is
recursively parsed for more declarations. The limits bound this closure walk;
they do not certify the primary parser or a concurrently mutating trusted
filesystem.

### E.3 Rules, checks, and precedence

Engine rules consume engine-neutral measurements plus a dedicated read-only
raw-source-facts projection; they do not recompute those facts differently per
engine. The normalized `Document` alone is insufficient because importer-
sensitive extensions, authored FBX curves and units, take boundaries, and
other declarations may already have been normalized or discarded. The raw
projection preserves such facts and their availability without exposing a
scale operation's private capability ledger as the general engine API.

That projection is the one shared provenance-and-coverage model for raw source
evidence, not a parallel per-feature copy. Loaders reuse its identity,
availability, and provenance primitives wherever scale planning needs the same
source observation. A scale capability inventory and proof ledger remain
operation-specific derived consumers because they additionally encode a
requested rewrite's closure and proof obligations; engine rules do not create
a third representation of the underlying source fact.

The format-neutral core also owns `urn:animsmith:exact-source-timing:1`, a
bounded same-load sidecar attached to a loaded source. It records generic exact
time units, frame periods, selected clip ranges, and typed availability without
format names, parser types, or format constants. A format adapter performs its
own projection into that vocabulary: the FBX adapter alone owns KTime constants,
integer period arithmetic, and ufbx resolved-property selection. Engine rules
consume the generic sidecar through RawSourceFactsV2 and prediction provenance
v3; neither the V2 outer identity nor the historical readers are widened.

A rule declares all of its preconditions: the full resolved profile identity,
input format and raw-source coverage, required measurement fields, rig roles
where applicable, and every explicit project intent or importer setting. A
required engine-prediction facet with missing evidence records scoped
`required_prediction_unavailable` rather than a passing finding, guessed
value, or generic engine-neutral coverage gap.

New engine prediction checks are grouped by stable concern rather than copied
once per engine. The initial families are:

- `engine-clip-boundary` for ranges, whole-frame requirements, and slicing;
- `engine-root-motion` for importer extraction/bake interpretation;
- `engine-track-support` for channels, extensions, and data an importer drops
  or reinterprets;
- `engine-unit-scale` for explicit unit conversion and resulting transform
  scale; and
- `engine-addressability` for scenes, animations, targets, and runtime labels.

The current `engine-root-motion` slice is the exact Unity Generic revision-2 /
6000.3 / `fbx-model-importer` FBX tuple. Its V6 provenance binds the V5
same-load source evidence to a bounded raw transform-path inventory and the
normalized per-clip movement-owner intent. The rule is applicable only when
that exact profile is resolved and at least one axis is explicitly declared.
For each declared `horizontal_xz`, `vertical_y`, or `yaw` axis, it compares
`movement_owner_xz`, `movement_owner_y`, or `movement_owner_yaw` with
`root_position_xz`, `root_position_y`, or `root_rotation`. `bake` is
`baked_into_pose`; `extract` is `stored_as_root_motion`. Gameplay ownership
matches baking, animation ownership matches extraction, and a mismatch emits
an ordinary error finding bound to the available prediction facet.

The rule has one explicit lifecycle: all available axis facets are `complete`,
a mixture of available and required-unavailable facets is `partial`, and all
required-unavailable work is `not_evaluated`. Missing or ambiguous path
resolution, a path that does not identify the explicitly resolved `Root` role,
incomplete raw-path or project-intent coverage, settings overflow, duplicate
clip names, or unavailable axis-specific translation/yaw evidence emits
`required_prediction_unavailable`; this state is not a content finding and
cannot be suppressed by `--allow`. The rule never falls back to `Hips`, even
though the consumer-neutral root-trajectory measurement still uses Hips when
Root is unresolved. Measurement availability is required, but numeric
translation/yaw magnitude is not a threshold and does not affect routing.

Generic's `root_motion_source` is a closed, case-sensitive, byte-exact relative
path. `/` is the only separator and has no escape syntax. Empty segments,
leading/trailing or doubled separators, `.`/`..`, backslashes, Unicode control
or format characters are rejected. The bounds are 1,024 UTF-8 bytes per
segment, 4,096 bytes per path, and 256 segments. The FBX adapter projects
source identities, names, and parent chains from the same input bytes with a
raw-preserving ufbx load. The implicit ufbx root and generated geometry/scale
helpers remain rows but cannot match. Complete inventory coverage is required
to prove `NoMatch`; partial or unavailable coverage yields `CoverageIncomplete`.
This is source evidence only: AnimSmith does not execute Unity, read back an
imported asset, play runtime clips, or certify engine behavior. Repository and
CI fixtures for this slice are self-authored synthetic FBX files only.

The selected profile chooses applicable rules within those check ids. The
file-scoped provenance retains the full resolved profile and source identity;
each check retains prediction facets keyed by the existing evaluation scope,
with the exact fact, setting, policy, and measurement basis for clean and
finding-bearing work alike. Findings bind to an available facet rather than
introducing another applied-rule id. Checks do not use `unity-*`, `unreal-*`,
and similar duplicate families.

The bounded 0.4.0 production slice is the exact Bevy revision 1 / 0.19.0 /
`gltf-asset-loader` animation-index selector rule. With complete nonempty glTF
or GLB source-animation inventory, `engine-addressability` emits one available
facet per source row whose subject is Bevy's `Animation{source_clip_index}`
display label. A source name is only metadata: absent and duplicate names do
not change the index selector. Partial or unavailable inventory emits one
required-unavailable inventory facet and no retained-prefix predictions, so
the 4,096-row raw-source and file-facet caps compose without an N+1 facet.
The resolved-settings/provenance v1 contract separately caps actual clip rows
at 4,096. A 4,097-clip document is therefore a bounded operator error before
prediction, not a silently truncated settings inventory; #485 owns any future
overflow evidence and shared multi-rule facet-budget policy.
This predicts the canonical selector convention only. It does not claim a
successful Bevy load, runtime asset existence, animation-target survival, or
graph wiring, and the index is not stable across source-order edits. Richer
scene/name/target addressability remains a separate inventory/adapter concern.

The current additional production slice is the exact Unreal revision 1 / 5.8 /
`fbx-importer` whole-end-frame rule. For every retained FBX animation stack,
the FBX adapter projects ufbx's selected complete range pair and exact KTime
period into the generic exact-source-timing sidecar. `engine-clip-boundary`
tests only whether the absolute selected end coordinate is on that lattice.
Incomplete inventory and unavailable declarations or ranges yield scoped
required-unavailable facets; decimal FPS, inferred frame numbers, import-range
selection, resampling, compression, root motion, and all other importer policy
remain out of scope.

The corresponding 0.4.0 standalone producer is
`generate addressability`. Its V1 root is separate from output-v11 and contains
one engine-neutral glTF/GLB animation inventory plus a nullable exact-Bevy
adapter. `animsmith-engine` owns this format/profile-specific envelope and its
strict reader; core remains limited to the generic source, closure, prediction,
and evaluation contracts it consumes. The neutral identity covers the primary
input, full existing dependency closure, and bounded raw animation/channel
observations only; tool metadata and the adapter cannot change it. The exact
Bevy adapter embeds the same prediction provenance and unchanged
`engine-addressability` evaluation,
so it creates neither a second selector rule nor a second check lifecycle.
Absent and repeated source names remain metadata. Scene/default-scene, skin,
target path or UUID, named-map-winner, and extension-handling claims require
new raw facts and a later profile revision and are outside this V1.

Issue #488 adds those rich facts as a separate immutable producer contract,
`urn:animsmith:schema:gltf-addressability:2`; the historical V1 root and
readers remain unchanged. V2 preserves the V1 animation inventory and adds
bounded same-load raw glTF scene, node, skin, node-to-skin attachment,
scene-path, default-scene, named-map, and unique animation-target evidence.
Coverage is independent for every domain: complete empty coverage proves
absence, while a retained prefix or unavailable domain never does. Rows retain
source-array indices, authored names, parent/child order, scene roots, ordered
skin joints, explicit `skin.skeleton`, and the dependency-closure identity.
The explicit skeleton member remains source evidence only because Bevy ignores
it; V2 makes no inferred-root claim and does not claim scene-instantiated
`SkinnedMesh` attachment.

The optional exact-Bevy adapter is paired with the existing one
`engine-addressability` evaluation and reuses the exact `Animation{i}`
primitive, without creating a second check lifecycle. Its separately
versioned rule bundle accompanies immutable Bevy profile revision 3 and pins
tag `v0.19.0`, commit
`c6f634ca9f406d68ba5109d921247b654cb42c10`, `bevy_gltf 0.19.0`, locked
`gltf 1.4.1`, and the label, loader, node-path, target-id, feature, and root
`Cargo.lock` sources.
`Scene{i}` is emitted for every declared scene; `Gltf.default_scene` is only a
route to an existing scene handle, with no `DefaultScene` label and no
fabricated `Scene0`. `Skin{i}/InverseBindMatrices` is eager for every source
skin, including unreferenced skins, using Bevy's identity fallback for an
absent accessor. `Skin{i}` is created when any source node references the skin
during the all-source-node construction pass. Source indices, not collected
vector order, are identity. Named maps are separate, source-order
last-write-wins projections; skin winners follow lazy first-reference order.

Target rows are per unique source animation target node and retain their
animation/channel contributors. Authored names or `GltfNode{source_index}`
fallbacks form the path, excluding the scene world-root. A path/UUID is
published only with complete hierarchy, reachability, closure, feature,
settings, and collision evidence. Unreachable or multiply reachable targets,
duplicate paths, UUID collisions, disabled/missing `bevy_animation`, disabled
`load_animations`, missing target pointer width, and incomplete evidence are
typed required-unavailable states. Exact target IDs require explicitly pinned
32- or 64-bit pointer width because Bevy hashes segment lengths with the target
pointer width's little-endian encoding; the host width is never inferred.

The projection carries explicit `target_coverage` separate from retained
target rows. It is complete, including for an empty target domain, or is
required-unavailable when raw node/scene/path or animation/channel evidence is
incomplete, and with `target_domain_truncated` at the 4,096-target row
ceiling. Other new-rich-projection bound failures use
`projection_bounds_exceeded`. Each new scene/node/skin/attachment/path/target/
map domain is bounded at 4,096 rows, with aggregate structural references at
65,536 and dynamic projection text at 1 MiB; the sealed V1 animation inventory
and `CheckEvaluation` scopes retain their own bounds. Names and path segments
are capped at 1,024 UTF-8 bytes, paths at 4,096 bytes and 256 segments, and
staged reports at 256 MiB. Overflow retains an independent canonical prefix;
strict readback rejects N+1 collections and contradictory states. A bounded second parse of already captured primary bytes is permitted
within the same loader invocation; reopening the primary or dependencies is
not. V2 is prediction evidence only and never certifies runtime loading,
scene spawning, target survival, graph wiring, or playback. Required-
unavailable prediction retains exit 1; malformed tuple/config/input remains
exit 2.

The second bounded 0.4.0 standalone producer is `generate import-advice`.
`animsmith-engine` owns its independent V1 envelope, canonical identity, and
strict reader. It embeds the same exact prediction-provenance record but is
not a check evaluation: it projects fully materialized profile settings into
document/per-clip importer properties beside same-load source identity,
dependency closure, source-to-normalized clip linkage, explicit loop/movement
intent, and normalized measurement availability. Unity 6000.3 Generic and
Humanoid revision 1 can project Convert Units, Bake Axis Conversion, Generic
Root Motion Source, and the three bake/extract root controls. Frozen Unreal
5.8 and Godot 4.7 revision 1 have no setting descriptors and therefore emit a
typed refusal rather than a guessed preset. Authored frame coordinates,
sampling policy, root-motion prediction, and unit-conversion inference remain
out of scope until their own exact source/profile authorities exist.

Revision 2 of the producer is an independent immutable contract,
`urn:animsmith:schema:engine-import-advice:2`. It uses output-v15's V4
prediction-provenance and prediction-basis types by reference and carries one
optional native `projection`. `available` requires that projection and omits
`refusal_reason` when the same-load dependency closure is complete; partial or
unavailable closure requires `refused`, `dependency_closure_incomplete`, and
no projection. Unknown tuples, missing settings, and unsupported formats are
configuration errors, so neither those errors nor incomplete closure can be
mistaken for an importer preset.

The only V2 advice profiles are exact: Godot `2` / `4.7` /
`resource-importer-scene` accepts glTF JSON and GLB and projects
`animation/fps` (1..120, verified default 30) plus `animation/trimming`
(verified default false); Unreal `2` / `5.8` / `fbx-importer` accepts FBX and
requires explicit `sample_rate` from `default_30`, `source_determined`, or
`custom_hz(1..48000)`. The projection retains explicit/default origin and no
other settings. It is a parameter projection from same-load evidence, not
engine execution, imported-asset readback, runtime/gameplay certification, or
project-file mutation.

`[checks.<id>]` remains the only authority for severity and explicit
enable/disable overrides. Existing checks such as `loop-closure`, `in-place`,
`foot-slide`, and `root-motion-speed` keep their current behavior under every
engine profile.

The shipped narrow slice is Bevy revision 3 / 0.19.0 /
`gltf-asset-loader` `engine-track-support`. It binds a bounded raw animation
inventory from the same load, with animation coverage and independent
per-animation channel coverage. A complete inventory can produce only
negative gate outcomes for rows dropped by the compiled `bevy_animation`
feature or by `load_animations`; a dropped row is not a content finding. The
compiled feature gate has precedence over the load setting. A complete-empty
inventory is not applicable. Partial or unavailable coverage emits exactly one
subjectless, unsuppressible `required_prediction_unavailable` inventory facet
and no retained-prefix prediction, including the N+1 boundary. If both gates
allow loading, the result is a stable required-unavailable runtime-survival
state. Extensions, other constructs, and positive runtime survival are not
modeled by this slice. V5 provenance and output-v19 are current for this Bevy
slice; V4/revision-2 and output-v15 remain preserved and readable. The Unity
Generic root-motion slice uses V6 provenance in output-v19, while output v9
and all other historical readers remain immutable.

Configuration precedence is consequently:

1. loader and measurement evidence establish what the input contains;
2. the resolved engine profile supplies immutable verified consumer facts;
3. resolved document and per-clip importer settings state the configured
   consumer behavior, while `[clips]` fields state the intended use;
4. `[checks]` supplies activation, severity, and tolerances; and
5. the runner evaluates each applicable facet or records its typed
   required-unavailable prediction state.

No later layer may fabricate evidence missing from an earlier one.

### E.4 Reproducible output and fail-closed coverage

`measure` remains engine-profile-independent: selecting an engine profile or
its importer settings does not change measurements. Existing measurement
inputs from `[clips]` and engine-neutral check policy retain their current
semantics. `lint` and generated-advice outputs that use a profile record, at
minimum:

- requested family, profile revision, engine version, and importer;
- resolved immutable profile URN and engine-facts digest;
- input format, raw-source coverage, and fully materialized document and
  per-clip importer settings;
- measurement and outer-output schema identities consumed;
- each check id plus facet scope, exact fact/policy/measurement basis, and
  primary-source reference set, including clean evaluations; and
- scoped required-unavailable reasons for prediction evidence that is missing,
  distinct from ordinary engine-neutral coverage gaps.

Output v17 publishes this record; output v9 remains immutable historical
evidence. Under Appendix C, `animsmith-core` owns the outer lint envelope, its
URN, and the
registry-independent prediction/provenance
wire types. `animsmith-engine` constructs those wire records through a one-way
adapter without making core depend on the engine crate. The exact JSON shape is
fixed by the v10 contract and permits a consumer to distinguish
a measured fact, a profile prediction, generated advice, and an engine
readback. Text and HTML views render that model rather than maintaining separate
conclusions.

Prediction, advice, and readback contracts reuse one versioned provenance
header for input dependency-closure identity, profile tuple and facts digest,
settings digest, schema identities, and source references. The primary-file
`InputIdentity` remains present, but it is not sufficient closure identity for
a text glTF or any source with external dependencies. The header also carries
a deterministic source-reference-to-resource map and the exact byte count and
digest of every distinct external resource declared or consumed by the loader
or target importer, keyed by a source-relative declaration rather than a host
absolute path. It records closure coverage as complete, partial, or
unavailable. If an applicable rule, suggestion, or readback comparison depends
on a resource whose identity cannot be established, that work is unavailable
rather than silently bound to primary-file bytes alone.

The three contracts' top-level schema identities and typed payloads remain
separate because their authorities differ: a shared header must not let advice
or an observation deserialize as measured or predicted evidence. This
centralizes common identity plumbing without coupling the contracts'
independent compatibility rules.

Profile selection never turns unavailable evidence into `not_applicable`.
`not_applicable` is reserved for a rule whose subject genuinely does not exist
under the resolved profile and project intent. An engine prediction is
**required** when the resolved profile and project intent make its rule
applicable, the run's `CheckSelection` selects its check, and the check remains
enabled after `[checks]` configuration. An unselected check remains visible as
`unselected` and is not required; an explicitly disabled check remains visible
as `disabled`. Once a check is selected and enabled, severity does not change
whether evidence is required. A required facet with schema-valid unavailable
measurement evidence or incomplete raw-source coverage emits typed
`required_prediction_unavailable` and makes `lint` exit 1. Malformed or
non-finite present measurement data remains a contract/operator error and exit
2; it is not prediction unavailability. The facet is not rewritten as a
content finding, and existing engine-neutral coverage gaps
remain nonblocking. Advice generation refuses the affected suggestion without
changing lint status. Unknown profile, version, importer, or setting
selections fail earlier as configuration errors; they are never downgraded to
coverage. Output v10 records the requirement state so embedded
consumers can apply the same policy without parsing CLI exit behavior.

### E.5 Root motion, units, and scale

Profiles keep these independent:

- the source/interchange linear unit captured from normative format semantics
  or an explicit declaration in the source itself;
- the target engine's native distance unit;
- the exact source-to-target distance mapping and whether the importer
  preserves physical dimensions;
- source and target handedness, up/forward axes, and the importer's selected
  axis-conversion behavior;
- the resulting component/root, mesh-node, selected runtime-facing joint, and
  attachment transform scales; and
- the project's movement-ownership policy.

For example, a centimetre-native engine does not imply a component scale of
`100`, and a metre-native format does not prove that a source hierarchy has
unit effective scale. Rules consume the coordinate-domain and selected-node
measurements designed in issues #267 and #268. They may recommend an importer
setting only when its documented behavior accounts for animation
translations, bones, sockets, attachments, IK targets, and collision anchors.
Version one has no TOML or profile override for source units. A loader records
the format-defined or source-declared unit and its provenance in the shared raw
projection; if that evidence is absent, an applicable unit prediction is
unavailable under E.4 rather than inferred from bounds, character height,
filenames, importer defaults, or a caller assertion.
They never infer units from bounds, character height, filenames, inverse-bind
magnitudes, or an observed persistent object scale.
Animation scale channels are dimensionless and never become length-bearing
merely because the target engine uses a different world unit.

The contract can therefore state, independently, that one source metre maps
to 100 target centimetre units, physical dimensions are preserved, and the
expected imported component/root, mesh-node, and selected-joint transform
scales are dimensionless `1.0`. A mismatch in any one of those claims cannot
be hidden by satisfying another.

Issue #269's accepted Appendix D result is incorporated here: whole-document
unit conversion and rest/bind hierarchy reparameterization are distinct
operations with separate positive-uniform, closure, and proof contracts.
Profile selection neither authorizes nor freezes an automatic normalization
path. It may diagnose the need for, or advise a separately requested,
Appendix D operation only when that operation's own source coverage and proof
requirements are satisfied.

Root motion is handled the same way. The profile describes which source or
projection a configured importer uses and what a bake/extract option means.
The artifact supplies engine-neutral horizontal displacement, signed vertical
displacement plus non-collapsing vertical excursion/extrema, and yaw evidence.
`[clips.<selector>]` independently declares
`movement_owner_xz`, `movement_owner_y`, and `movement_owner_yaw` as either
`"gameplay"` (the entity/controller owns world motion and the importer bakes
that component into pose) or `"animation"` (extracted root motion owns world
motion). Issue #466 owns these engine-neutral clip-intent fields. The existing
`in_place` field remains a compatibility alias only for
horizontal ownership (`true` means gameplay, `false` means animation); declaring
both spellings in one selector entry is a typed configuration error. Each valid
entry is normalized before the existing field-by-field exact/glob overlay, so
an alias in one layer and the canonical field in another remain valid. A missing
axis declaration makes only the intent-dependent rule for that axis not
applicable; it is never inferred from another axis. A filename such as `_RM`,
small translation magnitude, or the profile alone cannot declare intent.
Conversion between root motion and in-place remains ADR-gated and is not
authorized by this profile design.

### E.6 Advice, manifests, and readback

Generated import settings use a separate immutable advisory-sidecar contract.
They bind the complete input dependency-closure identity, full profile identity
and facts digest, selected importer settings, measurement/check identities,
and every suggestion or refusal. They must say when upstream canonicalization
is required instead of presenting an unsafe importer bake as equivalent. They
never affect lint exit status or claim to be measured evidence or the engine's
actual imported state.

The 0.4.0 generation boundary is one input document. The engine-neutral glTF
inventory may publish source indices, names, targets, and the complete input
dependency-closure identity.
A versioned Bevy adapter may derive typed labels and supported-extension
predictions from that inventory; those fields are not mislabeled as
engine-version-agnostic. Cross-file clip identity, runtime sets, collection
manifests, and collection-wide presets consume the file-scoped contract from
issue #409 in the 0.5.0 milestone rather than inventing a second identity here.

Engine readback has another immutable contract that records the exact engine
build, project/import-settings digest, observed outputs, and warnings. Smoke
tests compare a versioned prediction with that captured readback; successful
parsing alone is not proof that an expected clip, target, or setting survived
import. Engine SDKs, editor installations, licenses, and runtime harnesses stay
outside `animsmith-core`. CI fixtures must be synthetic, self-authored, or
redistribution-safe. Commercial packs may supply local, one-time validation
evidence, but are never repository fixtures, workflow downloads, caches, or
uploaded artifacts.

### E.7 Primary-source and version discipline

Engine behavior is mutable external state. Every built-in rule cites a primary
engine or specification source, records the verified target version and date,
and is rechecked when that target or rule changes. Marketplace observations
and local import probes are useful counterexamples, not authority for a
general engine claim.

The design was reconciled on 2026-08-20 against these current primary sources:

- Unity 6000.3 Animation Clip, Root Motion, and model-import manuals:
  <https://docs.unity3d.com/6000.3/Documentation/Manual/class-AnimationClip.html>,
  <https://docs.unity3d.com/6000.3/Documentation/Manual/RootMotion.html>, and
  <https://docs.unity3d.com/6000.3/Documentation/Manual/FBXImporter-Model.html>;
- Unreal Engine 5.8 Animation Sequence and FBX import documentation:
  <https://dev.epicgames.com/documentation/en-us/unreal-engine/animation-sequences-in-unreal-engine?application_version=5.8>
  and
  <https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-import-options-reference-in-unreal-engine?application_version=5.8>;
- Godot 4.7 advanced 3D import and scene-importer documentation:
  <https://docs.godotengine.org/en/4.7/tutorials/assets_pipeline/importing_3d_scenes/advanced_import_settings.html>
  and
  <https://docs.godotengine.org/en/4.7/classes/class_resourceimporterscene.html>;
- Bevy 0.19 `GltfAssetLabel`, `AnimationTargetId`, and animation API rustdoc:
  <https://docs.rs/bevy/0.19.0/bevy/gltf/enum.GltfAssetLabel.html> and
  <https://docs.rs/bevy/0.19.0/bevy/animation/struct.AnimationTargetId.html>;
  and
- the glTF 2.0 specification:
  <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html>.

Unversioned `latest` documentation is not retained as profile evidence.

### E.8 Implementation sequence and non-goals

The dependency order is:

1. this decision record;
2. coordinated engine-neutral measurement work for per-property coverage and
   horizontal/vertical root trajectory plus yaw (#402 and #408);
3. independent per-axis clip movement-ownership intent (#466), required before
   root-motion prediction but not by the measurement/profile substrate;
4. the dedicated raw importer-sensitive source-facts projection (#463);
5. strict engine-profile registry, config, and resolution types without
   prediction behavior (#464);
6. bounded, rooted dependency-closure capture from the raw resource domain
   (#475);
7. reproducible output-v17 provenance for the resolved target, facts digest,
   required-prediction state, and per-check basis (#465 and #482);
8. per-concern prediction rules, split from umbrella issue #154 into bounded
   engine/rule slices where their input facts differ, including the Unity
   Generic V6 root-motion path;
9. single-document preset advice and separate glTF-inventory/Bevy-adapter
   generation (#155 and #156);
10. prediction-versus-readback feasibility and harness decisions (#151); and
11. per-engine guides generated from the accepted rules and evidence (#157).

The two measurement tickets share schema, diff, rendering, and contract
surfaces, so they require one version plan and sequential implementation rather
than sibling branches. They do not depend on profile selection. Bevy graph-
template demand (#163) remains gated by the manifest and a demonstrated
consumer; the profile design does not turn it into an implementation
commitment.

This decision does not add engine dependencies, custom TOML profile
definitions, automatic unit or root-motion conversion, retargeting,
compression simulation, artistic thresholds, cross-file collection policy,
or claims of visual, deformation, artistic, gameplay, or runtime correctness.
Those require their own evidence and, where they rewrite motion, a separate
accepted design and proof boundary.

## Appendix F — decision record: file-scoped clip identity and collections

Issue #409 decides the boundary used when logical clips and runtime
relationships span physical source files. The decision is intentionally a
protocol decision followed by bounded implementation slices. It does not make
the existing multi-file spelling of `lint` collection-aware.

### F.1 Separate collection authority

AnimSmith provides two independently versioned contracts:

- `urn:animsmith:schema:collection-manifest:1`, a strict TOML declaration; and
- `urn:animsmith:schema:collection-output:5`, deterministic JSON evidence.

The historical `urn:animsmith:schema:collection-output:1` and
`urn:animsmith:schema:collection-output:2` contracts remain immutable. V2
derives its exact shape, budgets, and reader rules from V1, changing only the
nested ordinary output envelope identity from output v10 to output v11. The
historical `urn:animsmith:schema:collection-output:3` derives from V2 and adds one
bounded `dependency_closure` state to every source. A complete state retains the
loader's exact `DependencyClosureIdentityV1`; partial and unavailable states
retain a nonempty, sorted, unique sequence from the seven closed
`DependencyClosureCoverageReasonV1` values. The current `collection-output:11`
embeds the current output-v19 lint envelope with measurements-v18 while
preserving collection-output-v8 and its nested output-v16, and
collection-output-v4 with its nested output-v12/measurements-v15, as immutable
historical evidence. Only complete closure coverage can
establish a source, logical clip, or runtime-set member. This keeps primary
input identity, optional manifest digest pin, and complete loader-input
identity separate.

They are consumed and produced by an explicit command:

```text
animsmith collection lint COLLECTION.toml --format json
```

The command name, input identity, and output identity are distinct from
ordinary `animsmith lint FILE...`. Existing lint and measure invocations remain
document-local and continue to emit output-v13 with measurements-v16. A file
extension, the presence of multiple inputs, or repeated embedded take names
never switches commands implicitly.

The collection manifest is the sole AnimSmith authority for:

- durable logical clip identifiers;
- physical source and take bindings;
- optional source digest pins; and
- cross-file runtime-set identity and membership.

It is not a second authority for rig roles, per-clip expectations, check
severity, or check tolerances. Those remain in an explicitly selected existing
`animsmith.toml` config basis and retain their ordinary per-document selector
semantics. The collection manifest may select a complete config basis for a
source; it cannot override individual config fields.

An assembly recipe is not a collection manifest. Assembly is a mutating
producer that chooses a base, projects source domains, and publishes one
artifact. Collection lint preserves each physical source and evaluates
relationships without renaming, merging, or rewriting it. Likewise, the
animation-pack evaluation manifest remains a skill-level evidence and taxonomy
format. It may supply retained validation evidence, but it is not accepted as
the CLI protocol.

The repository-owned structured evaluation model is a separate skill-level
authority. `animation-pack-evaluation:1` remains frozen to independently
validated `collection-output:2`. Current evaluations use the additive
`animation-pack-evaluation:2` contract and bind the exact raw
`collection-output:11` digest and byte count plus its manifest identity. V2
retains every scrubbed source state, including complete dependency-closure
identity or the full typed partial/unavailable reason set, and proves exact
clip source/take and ordered runtime-set relationships against that V11 input.
The V2 validation boundary requires the explicitly selected checkout-matched
AnimSmith binary and invokes a hidden validation-only collection subcommand
that calls the one authoritative V11 strict reader before any projection.
Schema validity plus a matching raw digest is not sufficient, and there is no
second Python semantic mirror.
Clip duration and speed preserve exact measured, unavailable, or not-applicable
availability rather than collapsing the latter two categories.
Unavailable rows are mandatory typed soft failures, not grounds to shrink the
model to surviving records. A separate projection sidecar was rejected because
it would duplicate clip/set authority and create a third versioned contract;
V2 is the single report authority, while the exact hashed V11 document remains
the mechanical evidence binding. Neither model is accepted as a CLI protocol.

### F.2 Logical and physical clip identity

Every collection clip has one user-declared logical id and one physical
binding. Neither is sufficient alone.

The logical id is the durable public reference used by runtime sets and future
sidecar schemas. V1 ids are portable, lowercase ASCII, slash-separated tokens:

```text
com.example.pack/locomotion/walk-forward-in-place
```

Each token starts with `[a-z0-9]` and then contains only `[a-z0-9._-]`; empty,
`.` and `..` tokens are invalid. An id has at least two tokens and at most 255
bytes. The namespace is a collision-avoidance convention, not an ownership or
compatibility claim. It is opaque to AnimSmith: filenames, directories, take
names, vendor metadata, and clip behavior never generate or rewrite it.

The physical binding contains:

- one manifest-local source key;
- a zero-based source-local take index; and
- the exact expected embedded take name at that index.

Index and name are both required. Name alone is ambiguous when many files or
multiple takes use `Take 001`; index alone would silently accept reordered or
renamed source content. The source record supplies the safe root-relative
locator, observed primary `InputIdentity` (SHA-256 and bytes), optional
expected SHA-256, complete or typed-incomplete dependency-closure state, and
config basis. A source rename does not change the logical id, but
the manifest must be edited explicitly and output records the changed physical
locator. A digest pin is a binding assertion, not the logical identity.

One physical `(source, take_index)` may have only one logical id. Two ids bound
to the same physical take are an ambiguous duplicate and invalidate the
manifest. One source may legitimately bind several distinct take indices.
Distinct source locators with equal content digests remain distinct physical
sources; content equality does not create an alias or compatibility claim.

### F.3 Manifest V1 shape and composition

The normative implementation issue freezes the exact JSON Schema and Rust wire
types, but V1 has this authority shape:

```toml
schema = "urn:animsmith:schema:collection-manifest:1"
schema_version = 1
collection_id = "com.example.pack"
input_root = "assets"

[[sources]]
key = "walk-forward"
path = "locomotion/walk_forward.fbx"
config = "configs/locomotion.toml"
expected_sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

[[clips]]
id = "com.example.pack/locomotion/walk-forward-in-place"
source = "walk-forward"
take_index = 0
take_name = "Take 001"

[[runtime_sets]]
id = "com.example.pack/sets/walk-ring"
kind = "directional-blend"
members = [
  "com.example.pack/locomotion/walk-forward-in-place",
  "com.example.pack/locomotion/walk-left-in-place",
]
```

The schema/version pair, collection id, nonempty source/clip arrays, and every
field shown for clips are required. `input_root`, source `config`, and
`expected_sha256` are optional. An absent config means `Config::default()`;
there is no ambient config discovery. A selected config path applies to the
whole loaded source document. The same source cannot select different configs
for different takes.

The collection id and source keys use one token from the lowercase ASCII
logical-id grammar (no `/`) and are each limited to 255 bytes. Source keys are
manifest-local plumbing rather than public clip identity. Every clip and
runtime-set id must start with the exact collection id followed by `/`; this
makes the declared collection namespace an enforced prefix without treating it
as an ownership claim. Unknown fields, duplicate ids/keys, duplicate
source/take bindings, malformed digests, dangling source references, dangling
or repeated set members, and sets with fewer than two members are operator
errors.

Runtime-set ids use the logical-id grammar. V1 set kinds are membership
vocabulary only:

- `gait-group`;
- `sync-group`;
- `directional-blend`;
- `speed-blend`;
- `transition-chain`;
- `mask-composition`;
- `retarget-group`;
- `paired-interaction`; and
- `motion-database`.

The closed list prevents typo-created semantics. A kind does not itself claim
that its members blend, retarget, synchronize, or play correctly. V1 preserves
declared member order because some relationship consumers assign coordinates
or roles by declaration order. Output sorts source, clip, and set records by
their stable ids, while each set retains its declared member order.

V1 intentionally carries no generic policy map and no untyped TOML payload.
Check-specific coordinates, tolerances, or controller policies require a typed,
versioned contract owned by that check. They must not be smuggled into
collection V1 as open data merely to avoid a schema revision.

The parser and wire reader are bounded before allocation. The implementation
slice must freeze limits for manifest bytes, source rows, clip rows, runtime
sets, aggregate memberships, identifier bytes, and output bytes, with exact
`N`/`N+1` tests and checked arithmetic. Safe path spelling reuses the immutable
dependency-closure V1 key limits: 4,096 UTF-8 bytes and 128 components.

### F.4 Rooted access and immutable sources

The manifest directory is the control root. `input_root`, when present, is a
relative directory below it; source paths resolve below that input root. When
it is absent, source paths resolve below the manifest directory itself, never
the process working directory or another ambient root. Config paths always
resolve below the manifest directory. All declared paths use `/` as the
separator and are retained as UTF-8 evidence labels, never as host absolute
paths.

The CLI/frontend owns filesystem access. It reuses
`DependencyResourceKeyV1` with `ParserRelativePath` for lexical validation and
uses a rooted resolver equivalent to the assembly/material boundaries. It
rejects, before source execution:

- empty, `.`, `..`, absolute, drive-qualified, UNC, remote, or backslash paths;
- NUL, control characters, oversized keys, and excessive components;
- a symbolic-link component or final symbolic link;
- a non-directory input root, non-regular config target, or existing
  non-regular source target; and
- a canonical target outside its declared root.

Case and Unicode are preserved exactly; AnimSmith does not case-fold or Unicode
normalize a declaration into another identity. Lexical duplicates and aliases
the host resolver exposes as one canonical target are rejected. V1 does not
claim portable hard-link identity beyond what the host safely exposes.

The source is opened read-only, measured, and never renamed or rewritten.
Observed identity is derived from the bytes actually analyzed. An expected
digest mismatch is evidence about a readable binding, not permission to use the
bytes as if they matched. Root containment and symlink refusal prevent ordinary
path escapes; they do not claim to defeat a privileged actor concurrently
mutating the filesystem between checks and reads. That portable TOCTOU limit is
retained in the public security notes.

### F.5 Existing config and document-local checks

Collection execution loads each source with exactly one selected existing
config basis. `[clips."<selector>"]`, `[gait_groups]`, and `[sync_groups]`
continue to resolve embedded clip names inside that one document. Reaching a
config through a collection does not make its selectors match logical ids and
does not reinterpret a previously valid single-document project.

Document-local gait/sync checks may run as part of the nested source analysis.
Collection runtime sets are separate records keyed by logical ids. A shared
name does not merge the two domains, and membership is never inferred from a
document-local group, filename, directory, or evaluation taxonomy.

Core remains format- and filesystem-neutral. It may own validated bounded
logical/source/take/set values and deterministic evidence types. TOML parsing,
rooted access, loader dispatch, and command exit routing remain in the CLI
frontend. Format loaders continue to own their raw-source and dependency
closure projections.

### F.6 Collection output and completeness

`collection-output:1` is a historic strict envelope, not ordinary output v12. It contains:

- the manifest schema identity, observed manifest SHA-256/bytes, collection id,
  tool version, and frozen collection budget identity;
- one source record with its safe declared locator, observed or unavailable
  source identity, expected-digest result, config state/path/digest, loader
  state, and nested ordinary per-document result when available;
- one clip record with logical id, source reference, take-index/name witness,
  binding state, and exact nested measurement/check references;
- one runtime-set record with id, kind, every declared member, each resolution
  state, an explicit evaluation lifecycle, coverage gaps, findings, and any
  later check-specific measured payload; and
- deterministic totals and terminal work counters needed to prove bounds.

Nested ordinary results retain their published schema identities and strict
readers. They are not copied into a nearly equivalent collection-only shape.
Host absolute paths and raw host I/O messages are never emitted. A source row
uses its safe declared locator; stable typed states carry failures.

Collection-output V1 additionally carries one existing `ClipMeasurements`
value per established logical clip, selected from a duplicate-safe
normalized-index projection only after the raw source take index and exact
authored name map to that normalized index. This additive in-memory projection
does not change measurements-v15: ordinary output remains name-keyed. When two
authored names remain equal after loader normalization, their indexed
measurements remain distinct while a name-addressed check reference is typed
unavailable; no first/last match is guessed. Runtime-set V1 completeness
depends on the binding and indexed measurement, not that optional check
reference, and its decision remains `not_evaluated` until a later typed set
check owns a conclusion.
The normalized name is bounded at 4,101 bytes: the 4,096-byte authored-name
maximum plus `#` and the largest duplicate ordinal possible under the
4,096-clip manifest cap. Available nested measurement keys retain output-v10's
4,096-byte bound.
When that derived key exceeds output-v10's immutable 4,096-byte text bound, V1
retains physical binding and indexed measurements but marks the nested document
and name-addressed check reference `nested_output_unavailable`, then exits 1;
it never embeds a schema-invalid ordinary envelope.

The immutable output budget permits 1 GiB per primary source, 16 GiB of
aggregate primary-source reads, and 256 MiB of serialized/readback JSON. It
reuses the manifest V1 source, clip, runtime-set, aggregate-membership, and
aggregate-work limits. Primary reads and strict output readback stop at N+1;
checked work counters retain the terminal observation needed to distinguish an
exact limit from exhaustion.
Producer serialization also writes through a 256 MiB N+1 sink before any full
envelope can be allocated, and its self-reported byte count converges over the
finite decimal widths permitted by that same cap.
After an aggregate primary-read N+1 witness, later declared sources are retained
as `aggregate_exhausted` without being opened, so the terminal counter cannot
advance beyond that single witness.

The manifest digest binds the exact input bytes. Reordering otherwise
equivalent TOML therefore changes that input identity, while the decoded source,
clip, and set record arrays remain canonically ordered and byte-identical. The
contract does not erase source-manifest changes merely to make two different
inputs share a full output digest.

A set may report a complete conclusion only when every declared member needed
by that conclusion resolved and supplied the required evidence. Missing,
rejected, mismatched, quarantined, or unmeasurable members remain visible. The
evaluator must not shrink a set and report a clean result over the survivors.
Member ordering in output is deterministic and every group-level measurement
names its exact member basis.

Logical ids, equal source digests, common skeleton names, or membership in one
set prove none of retargeting, blend, artistic, deformation, engine, gameplay,
or cross-pack compatibility. Those claims require their own declared policy and
evidence.

### F.7 Failure and exit policy

The collection command introduces one explicit aggregate-data exception to the
ordinary `lint` operator-error rule. The exception is scoped to a manifest that
has already passed all control-plane validation:

- exit `2` with no collection envelope for an unreadable or malformed manifest,
  unsupported schema identity, budget violation, duplicate/unsafe declaration,
  existing non-regular source target, missing/unreadable/malformed selected
  config, serialization failure, or other control/tool error;
- exit `1` with a schema-complete collection envelope when a lexically safe declared
  source is missing or unreadable, when readable bytes are unsupported or
  malformed, when a digest/take binding mismatches, or when another required
  collection member cannot be established; and
- otherwise apply the existing lint severity and required-unavailable rules to
  available nested/check evidence, with exit `0` only when none requires `1`.

Missing/unreadable bytes and readable-but-rejected bytes have distinct typed
states even though both make a required collection incomplete. Unsafe paths and
configs are never downgraded to data availability. This preserves all declared
members for batch automation without silently changing ordinary `lint FILE...`,
which retains exit `2` and no output for an unreadable primary file.

An unavailable measurement inside a successfully loaded member follows the
owning check's existing coverage/required policy; collection membership alone
does not promote every nonblocking coverage gap to exit `1`. A source that
cannot be loaded at all is different: the declared physical clip identity was
not established, so the collection result is incomplete and exits `1`.

### F.8 Retained evidence exercise

The retained, externally generated animation-pack evidence behind #409 contains
the required marketplace shape: many physical files repeat one embedded take
name, while meaningful logical motions and runtime sets live outside those
takes. The public eight-pack rollup reports 895 individual files, 582 logical
motions, and 90 runtime-set records without publishing licensed source bytes.

The mapping exercise is lossless for the foundation facts:

- retained per-file locator and digest facts map to source rows;
- retained logical-motion identities map to namespaced clip ids;
- selected source-local takes map to the index-plus-name witness;
- retained runtime-set membership maps to logical-id member lists; and
- evaluator roles, confidence, capability profiles, workflow stages, report
  prose, and engine observations deliberately remain outside this contract.

This shows why the evaluation manifest must not be reused wholesale. It proves
the collection identity and membership need while keeping evaluation taxonomy
and product conclusions at their existing skill/report authority.

The retained license-safe spike fixture is
[`crates/animsmith/testdata/collection-spike`](crates/animsmith/testdata/collection-spike/README.md).
Its self-authored, motion-free glTF files cover two distinct sources with the
same bytes and embedded `Take 001`, one two-take source, cross-file gait and
sync sets, an explicit config basis, and duplicate-member, missing-member, and
escaping-source failures. The preservation table pins each locator, byte count,
digest, logical id, and index-plus-name witness without treating that retained
fixture as output-schema authority.

Repository tests use only synthetic/self-authored or explicitly
redistribution-safe multi-file assets. Licensed pack bytes and motion-bearing
derivatives remain outside source, CI, caches, logs, and public artifacts.

### F.9 Implementation sequence and non-goals

The 0.5.0 order is:

1. this accepted decision record and bounded issue decomposition (#409);
2. strict collection-manifest V1 types, budgets, logical/physical identity
   validation, and rooted resolver with license-safe fixtures (#545);
3. explicit `collection lint`, deterministic collection-output V1, nested
   per-document execution, aggregate failure routing, and strict reader tests
   (#546);
4. contact-sidecar and transition-family schema spikes (#147 and #148) using
   the logical clip-reference vocabulary without generation code;
5. truthful gait-phase statistic publication (#504), after separately choosing
   the existing maximum circular deviation from mean or a deliberate changed
   threshold statistic; and
6. root-speed/stride set evidence (#411), split so raw evidence and any typed
   controller policy do not expand collection V1 into an open policy map.

The parser/resolver and execution/output work are separate PRs because rooted
I/O review and public output/exit review have different failure surfaces. Both
must land in 0.5.0; closing the ADR alone does not deliver the milestone's
collection foundation.

This decision does not add collection report HTML, generated transitions,
engine graph output, archive extraction, filename-based intent inference,
source renaming/merging, motion rewriting, cross-pack compatibility claims, or
a generic untyped runtime policy language. The strict contact-fragment sidecar
producer specified in F.10 is the narrow exception; host-side merge and all
runtime interpretation remain later consumers or separately accepted contracts.

### F.10 Contact-fragment V1 (#147)

Issue #147 freezes the interchange contract for contact facts. In 0.6.0,
issue #152 delivers its narrow strict producer: `animsmith generate contact-fragment` and
`animsmith collection generate-contact-fragment` sample stance support and
atomically publish one source-bound fragment. The collection command reloads
its selected manifest source rather than consuming collection-output evidence.
Neither issue adds host-side sidecar merge or a runtime event system. Its
independently versioned identity is:

```text
urn:animsmith:schema:contact-fragment:1
```

The fragment is an importable envelope, not a second independently edited
authority beside a host's measured sidecar. A V1 envelope contains `schema`,
`schema_version`, `producer` (tool and tool-version), `artifact` (the exact
primary source bytes' existing `InputIdentity` wire `{sha256, bytes}`),
`dependency_closure_identity` (the complete existing
`DependencyClosureIdentityV1`,
serialized through that same `{sha256, bytes}` wire), one `clip` reference, a
measured positive `duration_s`, and an ordered `events` array. The `sha256`
field makes both the algorithm and digest explicit; this contract does not
introduce an `{algorithm, digest, bytes}` shape. Both identities are mandatory.
The closure identity binds the primary source and every dependency in the
loader's complete versioned dependency domain; partial or unavailable closure
coverage refuses fragment generation. This is the identity only, not the full
`dependency_closure` record used by output v10. The captured closure's
`primary_input` must equal `artifact`; the producer and any consumer checking
staleness validate that relationship against the complete captured closure.
Unknown fields are rejected by the strict core reader implemented in 0.6. A mismatch in either
identity makes the fragment stale rather than authorizing a consumer to use it.

AnimSmith implements the format-neutral strict reader and bounded
canonicalization/identity seam in `animsmith-core`, plus the strict producer
above. The producer loads one exact direct take or reloads one exact
manifest-selected take, requires complete finite bilateral stance evidence,
and publishes the canonical fragment as a single durable sidecar. It does not
implement contact transforms, host-side merge, runtime scheduling, engine
mapping, physical-contact inference, or gameplay semantics; those consumer
boundaries remain separately deferred.

V1 deliberately requires the complete modeled closure, including declared
dependencies such as textures that may not affect contact calculations. It can
therefore refuse generation when an unrelated dependency is unavailable. This
conservative policy avoids a second format-specific dependency-relevance
authority; narrowing it requires a separately versioned contract.

The clip reference is a tagged scope rather than a filename or animation-array
index:

- `collection` carries the #409 logical clip id together with its physical
  `source`, take-index, and exact take-name witness. The witness lets a host
  detect a manifest edit or take reorder even when a logical id was retained.
- `document` carries the exact embedded clip/take name and the primary-plus-
  dependency-closure binding that scopes that name. If the document exposes
  duplicate names or cannot establish an unambiguous source-local take
  identity, the reference is
  unavailable and the fragment is refused; no first/last animation match is
  guessed.

An event has a stable `event_id`, an engine-neutral `role` and `phase`, and
either a point `time` or a window `{start, end}`. Times are normalized to the
closed interval `[0, 1]`; `start <= end` is required for a window. A point or
window is never identified by its array position. Optional confidence and
explicitly versioned extension data are non-normative additions; confidence,
when present, is finite and lies in `[0, 1]`. Unity, Unreal, Bevy, and
gameplay-specific event types do not belong in V1. An
`event_id` is unique within the clip and remains stable when the same fact is
regenerated. It is opaque to consumers, is not an engine handle, and is not
derived from array position.

V1's closed role vocabulary is `left_foot`, `right_foot`, `left_hand`,
`right_hand`, `left_toe`, `right_toe`, `left_knee`, `right_knee`, `left_elbow`,
`right_elbow`, `root`, `prop`, and `body`. Its closed phase vocabulary is
`begin`, `end`, and `marker`. A producer that needs another semantic role or
phase must propose a versioned contract extension; it must not smuggle an
engine enum or arbitrary string into V1.

Canonical bytes use the complete [RFC 8785 JSON Canonicalization Scheme
(JCS)](https://www.rfc-editor.org/rfc/rfc8785), including its object-member
sorting, string escaping, and number serialization rules. Every value,
including an opaque extension payload, must be JCS-canonicalizable or the
fragment is refused; an extension does not supply a private key-ordering rule.
The reader accepts decoded IEEE-754 numbers only within the safe magnitude
`[-9007199254740991, 9007199254740991]`, including identity byte counts and
extension-payload numbers. Lexical decimal aliases are therefore the same
decoded JCS value, not distinct V1 facts. Extension payloads are reparsed from
their bounded JCS bytes before storage, so canonical serialization followed by
strict reread preserves the validated fragment value.
Before JCS serialization, a mixed point/window event sort key is exactly
`(start, kind_rank, end_key, role, phase, event_id)`: a point sets
`start = time`, `kind_rank = 0`, and the end sentinel `end_key = null`, which
sorts before every numeric window end; a window uses its declared `start`, has
`kind_rank = 1`, and sets `end_key = end`. Thus a
point and a window at the same start have a deterministic order, including
when their numeric endpoints are otherwise equal. Every string comparison in
that tuple, including the opaque `event_id`, is lexicographic by unsigned
UTF-16 code units exactly as RFC 8785 orders object property names; no Unicode
normalization is applied. Extension arrays retain
their declared order. Seconds and frame numbers, if
present for display, are derived values and never identity or comparison
coordinates. The strict core reader rejects unknown fields in the
core envelope and events, duplicate event ids, non-finite values, out-of-range
times, and ambiguous references before accepting the fragment. These rules
make a regenerated fragment reviewable without making event detection part of
this contract.

V1 is bounded before retention or canonicalization. A contact-fragment source
is at most 8,388,608 bytes of UTF-8 JSON with at most 32 nested arrays/objects,
4,096 events, and 256 extensions. Every authored string is at most 4,096 UTF-8
bytes; identifiers (including `event_id`, producer fields, schema ids, source
keys, and logical ids) are additionally at most 255 bytes, except the exact
64-lowercase-hex SHA-256 spelling. One extension payload is at most 262,144 JCS
bytes and 16 levels deep. The complete canonical fragment is also at most
8,388,608 JCS bytes. A contact-transform-result source and its complete
canonical result are each at most 16,777,216 bytes; it contains at most 4,096
control points and 4,096 event outcomes and embeds a contact fragment obeying
the preceding limits. Container depth is the number of object/array containers
on the path from the root to a value, inclusive: the top-level envelope object
has depth 1. An extension payload object participates in that full-envelope
count and is also depth 1 for its separate 16-level payload limit. A strict
reader accepts each exact maximum and rejects the first byte, element, string
byte, or nesting level above it while decoding, before retaining that excess
value or allocating an unbounded canonical buffer. Canonical byte limits are
enforced with a bounded JCS sink.
The published JSON Schema's `maxLength` limits are Unicode-code-point limits;
the reader's UTF-8 byte limits are deliberately stricter for non-ASCII text.

Extensions, when present, are an array of strict envelopes. Each envelope has
exactly `schema`, `schema_version`, and `payload`; `schema` is the extension's
versioned identity, `schema_version` is a positive integer, and `payload` is a
JSON object validated by that extension's schema. The core reader rejects
unknown envelope fields and preserves a well-formed unsupported extension as
opaque data; a consumer that requires its semantics may reject the fragment
with a typed unsupported-extension result. Core fields remain closed and an
extension cannot redefine them or add an engine event type.

A minimal collection-scoped envelope is illustrative of the normative shape
(the two input identities are not second clip digests):

```json
{
  "schema": "urn:animsmith:schema:contact-fragment:1",
  "schema_version": 1,
  "producer": {"tool": "animsmith", "version": "0.6.0"},
  "artifact": {"sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", "bytes": 123456},
  "dependency_closure_identity": {"sha256": "1111111111111111111111111111111111111111111111111111111111111111", "bytes": 456},
  "clip": {"scope": "collection", "logical_id": "com.example.pack/locomotion/walk-forward-in-place", "source": "walk-forward", "take_index": 0, "take_name": "Take 001"},
  "duration_s": 1.2,
  "events": [
    {"event_id": "left-foot/0", "role": "left_foot", "phase": "marker", "time": 0.23, "confidence": 0.92},
    {"event_id": "right-foot/0", "role": "right_foot", "phase": "begin", "window": {"start": 0.71, "end": 0.79}}
  ],
  "extensions": [{"schema": "urn:example:contact-quality:1", "schema_version": 1, "payload": {"quality": "high"}}]
}
```

The standalone `document` clip variant replaces the collection fields with
`clip_name` and retains both envelope input identities. A host may deterministically
merge this fragment into its existing sidecar, but the host remains the sole
authority for that file's unrelated fields.

Trim, slice, resample, and time-warp are contract-preserving only when the
contact fragment is transformed in the same operation. The minimal transform
result has identity
`urn:animsmith:schema:contact-transform-result:1` and fields `schema`,
`schema_version`, `operation`, `input`, `outcome`, `event_outcomes`, and,
only when `outcome = "transformed"`, `output`; a refusal additionally has a
`refusal` object containing exactly a stable `code` and `message` for display.
The strict tagged `operation` object has exactly these structurally
representable V1 shapes:

- `{"kind":"trim","version":1,"interval":{"start":a,"end":b}}`;
- `{"kind":"slice","version":1,"interval":{"start":a,"end":b}}`;
- `{"kind":"resample","version":1,"mapping":"identity"}`; and
- `{"kind":"time_warp","version":1,"output_duration_s":d,"control_points":[{"input_time":0,"output_time":0}, ...]}`.

The control-point array is ordered, finite, starts at `(0, 0)`, ends at
`(1, 1)`, and has strictly increasing input and output coordinates. The mapping
is piecewise linear: an exact input knot maps to its declared output knot; for
adjacent knots `(x0, y0)` and `(x1, y1)`, an input `t` strictly between them
maps to `y0 + ((t - x0) / (x1 - x0)) * (y1 - y0)`. Point times and both window
endpoints use that same mapping. Structural decoding retains a known V1
operation even when its finite numeric domain or ordering is invalid, so a
refusal can echo the request. An unknown kind, version, field, mapping token,
missing field, or malformed field type is a strict request/reader error and
produces no V1 transform result or event outcomes. `input`
has exactly
`{artifact:{sha256,bytes},dependency_closure_identity:{sha256,bytes},fragment:{sha256,bytes}}`
and refers to
the separately supplied input fragment; the input fragment is not duplicated
inline. A successful `output` has exactly
`{artifact:{sha256,bytes},dependency_closure_identity:{sha256,bytes},fragment:{sha256,bytes},contact_fragment}`;
its
inline `contact_fragment` is the complete transformed
`urn:animsmith:schema:contact-fragment:1` value. The output `fragment`
identity must equal the canonical serialized inline fragment's `{sha256,
bytes}`, and the output `artifact` identity must equal the inline fragment's
`artifact` binding. The output `dependency_closure_identity` must equal the
inline fragment's closure binding and must be freshly captured for the output
artifact rather than copied from the input. Its captured closure's
`primary_input` must equal the output `artifact`. The output is absent, never a
null placeholder, on refusal.

V1 operation mappings are exact: `trim` and `slice` carry a finite retained
interval `[a, b]` with `0 <= a < b <= 1` and map an in-range `t` to
`(t - a) / (b - a)`; `resample` carries the identity mapping on normalized
time; and `time_warp` carries the piecewise-linear control points above. A point outside
a trim/slice interval has outcome `outside` and is omitted only from a
successful output fragment. For a point, `t < a || t > b` is outside and both
endpoints are retained. A window is wholly outside exactly when
`end < a || start > b`, and is fully contained exactly when
`a <= start && end <= b`; equality at either retained endpoint is contained.
Every other window crosses a boundary and has outcome `refused` with code
`partial_window`, refusing the entire operation—no partial window is silently
clamped.

Preserving an opaque extension through an artifact transform cannot prove
that extension-owned times or event references remain valid. A transformer may
succeed only when it recognizes every extension schema/version and implements
that extension's operation-specific transform contract. Otherwise it refuses
the whole operation before event inventory with top-level code
`unsupported_extension`, an empty `event_outcomes`, and no output fragment.
The core transform context therefore carries the handler-produced extension
outputs, in exact input order and with the same schema/version at every
position. It never treats a caller-declared support set or an automatic opaque
payload copy as proof that an extension was transformed.

The inline output fragment's `duration_s` is exact: trim and slice use
`input_duration_s * (b - a)`, resample preserves `input_duration_s`, and
`time_warp` requires a finite positive `output_duration_s` field in its
operation object, which the output fragment must equal. These duration rules
apply even when every event is outside the retained interval. If the rounded
trim/slice duration is not finite and positive, the operation refuses before
event inventory with `invalid_value` and an empty `event_outcomes` list.

Every V1 time and duration number is a finite IEEE 754 binary64 value.
Arithmetic uses round-to-nearest, ties-to-even after each operation in the
following order; fused multiply-add and extended-precision intermediates are
not permitted. Write `rn(x)` for that rounding. Trim/slice first computes
`span = rn(b - a)`, then `mapped = rn(rn(t - a) / span)` and
`output_duration_s = rn(input_duration_s * span)`. Between time-warp knots it
computes `dx = rn(x1 - x0)`, `alpha = rn(rn(t - x0) / dx)`,
`dy = rn(y1 - y0)`, and `mapped = rn(y0 + rn(alpha * dy))`; an exact knot
bypasses interpolation and returns its declared `output_time`. Resample returns
the decoded input value unchanged. Any non-finite intermediate is
`invalid_value`. These binary64 results are the numbers supplied to JCS.

After input binding and fragment identity validation, `event_outcomes` contains
exactly one object per input event, in the fragment's canonical input order.
Each object has exactly `event_id` and `outcome`, plus `value` only when
`outcome = "transformed"` (the exact mapped point or window) and `code` only
when `outcome = "refused"`. A pre-inventory binding, fragment-identity,
operation-validation, or extension-support refusal uses an empty
`event_outcomes` list. Global success means
`outcome = "transformed"`, an output is present, and every event outcome is
`transformed` or `outside`; global refusal means `outcome = "refused"`, a
typed top-level refusal is present, and output is absent. A refusal discovered
after inventory has at least one refused event outcome. `partial_window` means
the exact crossing predicate above; `invalid_mapping` means a structurally
known operation has an out-of-domain interval, invalid endpoint/order, or
non-monotonic control points; `invalid_binding` means the supplied fragment,
primary artifact, or dependency-closure identity does not match current input;
and `invalid_value` means valid finite inputs produced a non-finite arithmetic
intermediate or a rounded trim/slice duration outside the fragment's finite
positive domain. Unsupported extension transformation uses
`unsupported_extension` as specified above. A malformed fragment/result or
duplicate, missing, or unknown event-outcome identity is a strict reader error,
not another refusal result. The result records the operation, bindings, and
outcomes. Core implements the exact mapping and independently rederives a read
result against its separately supplied input fragment and external transform
context. That context carries the current input and output artifacts, their
complete captured closures, the expected output producer, and any
operation-specific transformed extension outputs. Core validates that each
closure's primary input is its corresponding artifact before deriving the
identity recorded in a successful result; detector policy, asset mutation, and
publication remain outside this contract.

AnimSmith owns the strict sampled stance-support contact facts and these
identity/time/transform semantics. The host owns final file layout and merge,
unrelated measurements and provenance, runtime scheduling, and engine-native
mapping. The delivered contact-fragment producer does not perform contact transforms, infer
gameplay meaning, validate foot placement, or claim engine, artistic, or
gameplay correctness. Host-side merge and runtime behavior remain deferred.

### F.10A Skeleton compatibility V1 (#158)

Issue #158 adds a strict structural seam for declared source/target skeleton
comparison without implementing retargeting, mesh rewrite, engine-avatar
generation, masking, contacts, or gameplay proof. Its immutable JSON result
identity is:

```text
urn:animsmith:schema:skeleton-compatibility:1
```

The public command is
`animsmith skeleton compare SOURCE TARGET --correspondence FILE --format json|text`.
`FILE` is a separate strict TOML authority with independent identity
`urn:animsmith:skeleton-correspondence:1`. It binds the exact source and
target primary input identities, explicit selected roots and finite selected
node-name sets on both sides, either exact-name or one-to-one explicit
correspondence, and every numeric tolerance. Unknown fields, duplicate names
or targets, malformed or stale identities, ambiguous selectors, non-finite
tolerances, oversize control input, or source/target primary inputs above the
existing 64 MiB bound are operator errors that emit no result.

The result records exact source and target primary identities, complete
dependency-closure identities when same-load capture is complete, the exact
correspondence bytes, canonical selected-skeleton provenance for each side,
deterministic row order, and separate non-scoring evidence facets for source
skin membership, inverse-bind accessors, and deformation model coverage.
Topology/rest rows are the only scoring surface in V1. `compatible` means
every required row was available and within tolerance; `partial` retains mixed
available and unavailable required evidence even when another retained field
mismatches; otherwise any required mismatch is `incompatible`; and
`not_evaluated` means no correspondence row was evaluable. Missing source/target
names and parent mismatch remain explicit typed rows rather than being
collapsed into an aggregate score.

The seam compares only the declared selector members. It does not infer a
skeleton subtree from attachments, scene reachability, or skin membership.
Parent correspondence and normalized child length walk through unselected
intermediate nodes to the nearest selected ancestor, so they describe the
declared projection and do not claim complete raw-hierarchy equality.
`selected_skeleton_identity` is derived provenance, not a copied authored
profile: AnimSmith serializes a canonical, name-ordered projection of each
selected node's measured structural evidence and the declared selector, then
records that exact identity. Local and rest-world transform deltas reuse the
existing normalized source-skeleton measurements contract; world-space
comparison stays unavailable when the loaded source evidence does not prove a
finite right-handed unit-orthonormal rest-world basis.

Non-pass rows and unavailable evidence sides carry a typed remediation surface
and likely remedy class. This routes work to correspondence, the selected
skeleton authority, selected skin/bind evidence, or a measurement boundary
without guessing whether a project integrator, artist, or vendor is to blame.
The human text view renders those fields together with exact identities,
matching authority, tolerances, row deltas, and facet details.

### F.10B Foot-cycle parameterization planner V1 (#18)

Issue #18's first post-result implementation slice adds the separate strict
`urn:animsmith:schema:foot-cycle-parameterization:1` TOML declaration and pure
format-neutral map planner. The declaration binds exact manifest bytes, one
existing ordered `gait-group`, one explicit reference, one safe fragment path
per member, the future generation directory, and finite positive segment-slope
bounds whose inclusive interval contains identity slope `1.0`. It also requires
a closed `[proof]` table whose finite maximum gait-phase spread and contact
boundary phase error each lie in `[0, 0.5]`, and whose finite minimum
left/right amplitude is non-negative. These values are bound by the exact
parameterization bytes and retained in the plan, with no defaults, config
lookup, inference, or cross-member merge. The planner
accepts only canonical collection-scoped fragments with
the exact known stance-detector V1 provenance and independently measured typed
Root/Hips evidence bound to the same exact artifact, dependency closure, and
collection source/take witness. Missing, ambiguous, malformed, or non-finite root evidence
refuses. It retains signed endpoint X/Z displacement and signed accumulated
unwrapped yaw, while the unchanged admission applies binary64 horizontal
`hypot(X, Z)` at 0.01 m and absolute yaw at 1 degree; this pure slice does not
derive those source measurements. It partitions positive windows and markers
by side, requires one marker per physical window and strictly disjoint
same-side runs, and preserves those physical rows. A distinct same-side first
run beginning at `0` and last run ending at `1` coalesce into one logical
cyclic stance whose artificial seam edges do not enter correspondence; a full
`[0,1]` stance refuses. Opposite-side overlap and touch are valid. Logical
left/right counts must be equal and nonzero, with alternating onsets around the
cycle. The planner builds the true logical boundaries in deterministic cyclic
chronological order, rotates correspondence (not phases) to a true left onset,
and requires one identical boundary signature across members. It pairs
boundaries positionally and maps them to the reference's authored phases.
Simultaneous boundary groups must agree on both source and reference instants;
identical map pairs collapse before `(0,0)` and `(1,1)` are included and the
retained 4,096-knot cap is checked. It returns bounded duration-preserving
`time_warp` operations with exact contact input bindings. All members must use one exact detector contact
height because that policy determines the compared boundaries. The whole ring
admits at most 16,384 contact events and 32 MiB of canonical fragment bytes
before topology retention and repeated canonicalization, and marker/window
validation is ordered rather than cross-product work. Only the correspondence
topology is circularly normalized: phase zero is not rotated, so incompatible
wrap/non-wrap correspondence or any other mapping that cannot remain strictly
monotone and endpoint preserving refuses. This slice does not load or mutate
animation assets, transform fragments, derive its required typed in-place
Root/Hips evidence, serialize candidates, prove output, or publish the
collection generation.

### F.10C Foot-cycle clip candidate V1 (#18)

The next independent slice is the pure format-neutral application of one
planned member map to an already selected `Clip`. `time_warp_clip_v1` validates
track-local clip/track shape again, requires the plan's exact duration-preserving
V1 operation, and returns a cloned candidate without mutating its input. This
Clip-only seam has no skeleton input, so its host must already have bound every
track bone index to the selected, validated skeleton.
Normalized control points map source time to output time: LINEAR tracks retain
all mapped authored keys and add interior map-knot samples with the existing
runtime sampling semantics, while STEP tracks map only authored breakpoints.
Binary64 calculations may narrow to finite binary32 track times, but distinct
source or output instants that collide after narrowing refuse rather than being
silently collapsed. A map knot and an authored key that are the same instant in
that emitted binary32 domain — equal, or one representable place apart, since
nothing is representable between them — are one emitted key at the authored
time and value, carrying the control point's own output.
`FootCycleClipWarpKnotV1` is the single definition of that question, and
`time_warp_rows_v1` merges a track's authored keys with its knots into the key
sequence a candidate stores. A knot-bearing row carries that knot — its
control-point index and that point's two narrowed times — and no stored value,
so the independent `ClipMap` proof in the CLI crate consumes the rows for the
sequence and reads only the index, narrowing the control point itself rather
than spelling coincidence a second way. What the proof takes from core is which
keys exist and in what order; every number is its own, including a knot's two
times, which it narrows again from the control point the row names rather than
reading the producer's resolution of them. A resolution that drifted by one
binary32 place is then a mismatch rather than a shared assumption. That holds
for
every authored key the map has a control point for, including a track's own
first and last: the span test is inclusive of one place on either side, so
which side of a rounding step the reconstructed instant lands on does not
decide the outcome. A key the knot is bit-equal to binds before a key it is
merely beside, and coalescing only ever re-times an authored key — it never
removes one. The single exception is an interior point that is the same
instant as a source endpoint, `0` or the duration: it contributes nothing,
because the exact endpoint rows define the output there and a candidate retains
those two instants as themselves. Identity maps return a structurally identical
admissible clip.

CUBICSPLINE remains deliberately conservative: one-key tracks are retained;
multi-key tracks are retained only when every stored key value is bit-exact
equal and all stored input/output tangents are zero. Any other cubic track
refuses the whole candidate. Finite quaternion keys must also have a finite
positive binary32 squared length before any runtime sampling. Fixed checked
caps refuse N+1 retained name bytes, tracks, authored keys/values, generated
keys, and aggregate inspection work before candidate allocation. Candidate
storage has a derived public upper bound: retained cubic storage is already
bounded by authored values, while LINEAR/STEP emit one value per bounded
generated key. This slice does not prove that the
plan belongs to the loaded artifact, transform contact
fragments, serialize or reread an output, or publish a generation directory.
Those bindings and atomic transaction steps remain later #18 work.

### F.10D Foot-cycle source preparation V1 (#18)

The CLI crate's next private seam binds the two prior pure boundaries to exact
local source bytes without adding a command. It bounded-reads the manifest and
parameterization once, resolves the exact member-reachable manifest
source/config subset in canonical manifest source-key order plus every
parameterization contact/output locator under its explicit declaration root,
and rejects symlinks, canonical collisions, and platform-observable hardlink
aliases before parsing any motion source. The future output leaf must be absent
under an existing symlink-free parent. Pure manifest/runtime-set binding is
validated first. Per-source and aggregate byte caps then cover only the exact
member-reachable source subset, retained in manifest order; unrelated manifest
sources do not gate this collection-scoped operation. Every reachable source
and its exact config basis are loaded. Both the normalized config semantics and
the explicit config byte identity are retained for later proof without a
control-file reread. Any declared digest pin is enforced, while complete
dependency-closure identities, complete raw take inventories, and exact
index/name-to-normalized-clip witnesses are mandatory.

Each normalized reachable document passes the core's strict structural shape
validation before it can reach sampling or candidate work. The adapter then
preflights every selected member's `frames * bones` pose cells and `frames *
tracks` sampling work, including the complete invocation totals, before
constructing the first `MetricGrids` grid. It uses configured role resolution
and `root_trajectory_metrics` as the existing authority. Horizontal
admission is the binary64 `hypot` of retained signed X/Z endpoint components
and yaw admission is the absolute value of retained signed accumulated
unwrapped yaw. Missing, ambiguous, non-finite,
or over-threshold evidence remains typed and makes the core planner refuse. The
prepared collection retains the exact aggregate source pose-cell and
sample-evaluation totals for a later shared source/output proof budget. The
adapter cross-checks every plan's member, source artifact, closure, canonical
fragment, selected take, duration, and root binding before building the pure
clip candidate. Any later member failure discards all candidates; preparation
does no filesystem mutation.

Contact output remains intentionally unfinished at this boundary. Core exposes
one operation-aware handler that validates and reconstructs the exact closed
stance-support detector payload for `time_warp`; the adapter retains that
handler output together with the exact input fragment, artifact, closure, and
operation. It cannot truthfully construct the output artifact identity or
output dependency closure until a complete candidate document is serialized.
The typed continuation accepts those later exact identities and only then calls
`transform_contact_fragment_v1`. This slice therefore performs no
serialization, generated-artifact hashing, reread/proof, generation staging,
publication, or CLI exposure.

### F.10E Bounded candidate GLB projection V1 (#18)

The next private format-edge seam is an in-memory GLB writer, not a collection
producer. It first makes an allocation-free strict admission pass over the
normalized document: checked row, name-byte, and traversal-work ceilings bound
the modeled writer arrays, scalar binary components, validation lookups, source
sidecars, and copied texture bytes before it constructs writer-owned JSON. It then counts the
single shared projection with checked JSON/BIN/GLB framing arithmetic and
returns an immutable receipt containing padded chunk sizes, framing size, and
JSON/BIN digests. The receipt binds that emitted projection, not deliberately
excluded source-only provenance. Retaining output reruns the projection and
accepts only the same receipt, reserving exact output capacities before copying bytes.

The legacy path remains its established permissive conversion projection. The
new `StrictFootCycleV1` policy instead fails closed on any normalized fact that
the writer cannot preserve: omitted clips/tracks, primitive or texture domains,
non-explicit skinned inverse binds, invalid primary skin slots, incompatible
skinned/unskinned mesh reuse, meshless materials, scene membership collapse,
or source-local rest and source-skin root facts without an exact normalized
representation. Strict output attaches a skinned mesh to its normalized node,
rather than applying legacy's generated-holder normalization. Non-empty
complete source material-resource sidecars refuse: V1 has no source
texture/image identity or inspection-fact projection. An exact complete-empty
resource graph is admitted only when the writer-facing material list is also
empty. Source-only names, stable
source indices, and image-inspection/provenance evidence remain deliberately
outside the output projection; they are not presented as candidate facts.
This seam does not bind a prepared collection member to its source, serialize
contact transforms, reread/prove output, stage files, publish a directory, or
add a CLI command.

### F.10F Foot-cycle serialized-candidate proof V1 (#18)

The next private transaction consumes one complete prepared collection without
adding filesystem or command behavior. For every declared member it clones the
member's retained source document and replaces only the selected clip with the
prepared candidate. It validates the document, runs `StrictFootCycleV1`
preflight, sums exact framed GLB sizes through one checked 256 MiB candidate
ceiling, and admits the combined source-plus-output metric-grid pose-cell and
sample-evaluation work before the first candidate write. All candidate writes
finish before any readback; every exact byte vector is then reread through the
ordinary glTF loader. Any refusal drops the local vectors and returns no partial
batch.

The reread artifact identity must equal the exact retained bytes. Its dependency
closure must be complete and bound to that primary identity, with no external
resources and no reference target other than `Primary`; embedded BIN-buffer
references therefore remain valid self-contained evidence. Only then does the
transaction finish the prepared contact transform with the reread artifact,
closure, and independently redetected producer identity.

Proof does not treat the existing checks as its verdict. It independently
revalidates the finite strictly increasing endpoint-preserving map and exact
LINEAR/STEP/cubic candidate relationship, including the constant-value,
zero-tangent multi-key CUBICSPLINE restriction. One already-budgeted output grid
per member supplies fresh contact topology/boundaries, gait phase/amplitude,
signed Root/Hips trajectory, and per-bone loop pose plus linear/angular seam
velocity. Contact and gait use the declaration's required proof fields; root
facts are compared to retained signed source facts under the fixed 0.01 m and
1 degree V1 limits; loop facts use the exact captured source config's effective
per-clip/check/default tolerance resolution. The result retains exact artifacts,
closures, completed transforms, independently detected contacts, proof facts,
phase spread, and separate source/output/combined work totals for the later
evidence and publication transaction. Its contact proof independently repeats
the per-side marker/disjointness validation, seam coalescing, logical-count and
cyclic-onset checks, and deterministic true-boundary ordering rather than
calling planner internals. Before its circular numeric error comparison, proof also
requires the expected and independently detected `(side, edge)` signature in
absolute phase-zero chronological order to match, so a complete left/right
role swap remains a topology failure at every numeric error tolerance. It
defines no public evidence schema,
filesystem layout, publication, or CLI behavior.

### F.10G Foot-cycle generation producer V1 (#18)

The final CLI-private transaction exposes exactly
`collection transform-foot-cycle COLLECTION.toml --parameterization
FOOT-CYCLE.toml --format json`; the required JSON-only format has no default
and the parameterization is the sole destination authority. The glTF-only
build retains this command, while FBX admission remains feature gated. Every
successful source becomes a self-contained strict GLB.

For `N` members the fixed generation has checked `3N+1` cardinality and exact
declared order: each `members/NNNNNN/` contains `artifact.glb`,
`contact-fragment.json`, and `evidence.json`, followed by the root
`aggregate-evidence.json`. The member evidence V1 contract is
`urn:animsmith:schema:foot-cycle-member-evidence:1`; it binds exact control
inputs, source artifact/config/complete closure, output artifact/complete
closure, transformed and independently redetected contacts, canonical map
identity, policy, proof facts, and deterministic relative paths. Aggregate
evidence V1 is `urn:animsmith:schema:foot-cycle-aggregate-evidence:1`; it binds
the ordered member summaries and member-evidence identities,
phase spread, component and exact total bytes, retained GLB bytes, and
source/output/combined proof work. Strict bounded readers and published JSON
Schemas close both records.

All GLB, fragment, member-evidence, and aggregate-evidence bytes share one 256
MiB retained/staged ceiling; each component retains its narrower contract cap.
Checked arithmetic admits the exact file count and byte total, including the
aggregate record's own converged serialized size, before the first filesystem
write. One `publish_generation` call stages and flushes the complete fixed set
under a sibling directory, then performs one platform no-replace directory
rename. An existing or racing destination wins unchanged. This is atomic
visibility of a new local generation, not replacement, power-loss, network
filesystem, or crash-cleanup atomicity.

Success emits the exact aggregate file bytes to stdout only after publication;
a broken stream is best-effort diagnosed and retains exit 0. Valid asset,
capability, refinement, contact-transform, or proof refusal publishes nothing,
emits one `producer-refusal:1` record, and exits 1. Malformed or stale control
bindings, unsafe/aliased paths, I/O, serialization, publication, and race
failures publish nothing, keep stdout empty, and exit 2. The producer never
rereads mutable manifest, parameterization, config, or contact controls after
their exact capture and never substitutes the writer's candidate model for the
independent serialized readback proof.

### F.11 Transition-family declaration V1 (#148)

Issue #148 freezes declarations for transition families without implementing
transition checks, reports, inferred graph edges, or runtime generation. Its
independent identity is:

```text
urn:animsmith:schema:transition-family:1
```

The document-local tables are admitted by the CLI's normal strict config
loader, which retains the whole-config exact input identity alongside the
normalized declaration. The JSON-only document command and manifest-bound
collection command now evaluate that authority. The collection form binds its
exact manifest before source work and reloads only manifest-witnessed takes.

The format-neutral core now owns the strict `SkeletonBasisV1` identity, both
document and collection transition-pose evaluators, and their shared result contract
`urn:animsmith:schema:transition-pose-evaluation:1`. It admits a mutable
loaded document's skeleton and selected T/R tracks, plans bounded endpoint
work before sampling, and returns an
immutable complete/finding/not-evaluated result. The collection adapter
reloads only manifest-witnessed sources; neither adapter adds a lint check.
The document adapter is the
JSON-only `animsmith evaluate-transition-poses INPUT --format json` command:
it loads one document through the normal public loader/config path, binds the
loaded exact raw document identity, and emits the core result unchanged. It
uses the exact zero-byte TOML sequence as the defined omitted-config
declaration source, so an omitted config and an explicitly empty config have
the same exact and normalized declaration identities. The collection adapter
preserves every declared family member and makes an unavailable or unusable
source a whole-family incomplete result rather than evaluating a subset.
Its scope-neutral `subject_input` binds the exact raw document in this
document evaluator; the collection adapter binds its manifest subject under
the same result schema. The normalized declaration scope determines
which subject meaning applies. For document scope, the public evaluator accepts
the loader's same-load `DependencyClosureV1`, takes `subject_input` only from
its `primary_input`, and retains its complete
`subject_dependency_closure_identity`. Every document member retains the same
primary identity as `source_input` and the same complete closure identity as
`source_dependency_closure_identity`. Complete coverage and a present
`DependencyClosureIdentityV1` are both required before any configured family
can be complete; partial, unavailable, or identity-less coverage makes every
configured family `incomplete/not_evaluated` with
`dependency_closure_incomplete`. The `no_configured_families` result is the
deliberate exception because it performs no source evaluation and remains a
complete pass even when closure capture is unavailable.

Collection scope does not fabricate a dependency closure for its manifest:
`subject_input` is the manifest's primary identity, while each available
member independently retains its raw `source_input` and complete
`source_dependency_closure_identity`. An unavailable source may retain a null
`source_input` when no primary bytes were obtained. One missing or incomplete
member closure makes the entire family `dependency_closure_incomplete`, but
other available members keep their closure identities as positive evidence;
the family is never evaluated as a survivor subset. Thus unchanged primary
text glTF bytes cannot mask changed external animation-buffer bytes.
Before sampling, the evaluator directly verifies every declared index/name
witness, admits at most 4,096 document clips for its one-pass global
duplicate-name proof, 4,096 bones, and 8 MiB of basis-name text. A document
above the clip cap with no direct witness contradiction yields normal
`input_limit` family rows without scanning its full name domain. Each selected
clip first admits at most three raw flat track rows per bone (12,288 maximum),
then bounds the consumed translation/rotation domain to two channels per bone.
Scale values, shape, and duplicates remain semantically ignored, but raw scale
rows count toward that first resource admission because the loader stores all
properties in one vector. Across the invocation it inspects at most the V1
aggregate-comparison bound of 16,777,216 selected time/value elements. A
valid-witness input exceeding one of these admission limits retains every
family as `incomplete/not_evaluated` with `input_limit`;
unrelated assets and unselected clips are outside this evaluator boundary.
Before sampling it serializes the full pair-free authority, including subject
and per-member dependency-closure identities, and subtracts that from a
conservative detailed JCS reservation, producing `result_limit` rather than
cloning a representation that must later be discarded as over-cap.

The declaration has a tagged `scope`, a stable `family_id`, an explicit
ordered `members` array, a `boundary` (`entry`, `exit`, or `both`), a typed
`basis`, and typed named `tolerances`. A family has at least two members.
`family_id` is unique within its owner and is never inferred from filenames,
directories, embedded names, or member order. Unknown fields and duplicate
members are invalid.

Document-local family ids are one lowercase-ASCII token, 1–255 bytes, starting
with `[a-z0-9]` and continuing with `[a-z0-9._-]`. The table key is the id and
there is no duplicate `family_id` field. TOML must quote the key whenever it
contains punctuation or a dot (the canonical spelling quotes it always), for
example `[transition_families."walk_to_run"]` or
`[transition_families."combat.entry.v1"]`. Collection family ids retain
Appendix F's slash-qualified logical-id grammar and collection-id prefix.

The two scopes are intentionally different authorities:

- `document` declarations are placed in the existing `animsmith.toml` config
  basis under `[transition_families."<family_id>"]`.
  The table key itself is
  the family id; it is a reusable config declaration and carries no artifact
  digest. The document evaluator binds the exact primary document
  `InputIdentity` and complete dependency-closure identity in its output, while
  members use exact embedded take-index/name witnesses. A
  repeated or loader-ambiguous take name is a strict resolution failure.
- `collection` declarations use a separate declaration envelope, not
  an extension of collection-manifest V1 and not another path/member authority.
  The envelope binds the exact collection-manifest `InputIdentity` `{sha256,
  bytes}`, collection id, and one or more family records. Each record
  references a declared logical clip id plus its corresponding `source`,
  take-index, and take-name witness.

A declaration cannot mix scopes, use a filesystem path, use an animation-array
index as identity, or import a member from another document or collection.
Missing, duplicate, cross-scope, and ambiguous members are strict resolution
failures. `stale_digest` is a collection-only failure: it means the envelope's
manifest `InputIdentity` no longer matches the declaration. A source digest
pin, when present in that manifest, is enforced by manifest resolution and a
mismatch makes the affected member unavailable; it is not a second identity
carried by this declaration. A reusable document-local config has no
stale-digest state at parse time; the document evaluator binds the exact primary
document `InputIdentity` plus its complete dependency-closure identity and
reports its source/take resolution in its output. A family
is not silently reduced to the members that happen to resolve, and a
collection family is not copied into a document-local config. The existing `[clips]`, `[gait_groups]`, and
`[sync_groups]` sections retain their document-local embedded-name semantics;
this declaration does not replace, reinterpret, or create a second authority
for them. The document-local table and collection envelope share the same
`transition-family:1` family-record semantics; only their ownership and
placement differ.

V1's basis is explicit and engine-neutral: translation is metres in the
declared skeleton-local basis, rotation is degrees in the same basis, and time
is normalized clip time in `[0, 1]`. The named tolerance record is closed and
unit-bearing: `translation_m`, `rotation_deg`, and `time_normalized`, each a
finite non-negative value. `boundary` selects whether the evaluator
considers entry, exit, or both; it does not itself perform a comparison.
Additional policy fields require a separately versioned contract, not a
generic TOML map or untyped extension payload.

Both declaration placements are bounded before retention or
canonicalization. The exact declaration source is at most 8,388,608 bytes and
16 nested tables/arrays, with at most 4,096 families, 4,096 members in one
family, and 16,384 members across all families. Authored strings are at most
4,096 UTF-8 bytes, with the 255-byte identifier limits above applied more
strictly. The normalized JSON envelope is at most 8,388,608 JCS bytes and has
the same nesting limit. Depth counts each table/array or JSON object/array on
the root-to-value path, inclusive; the root table or envelope object is depth
1. The strict reader accepts each exact maximum and rejects the first
byte, element, string byte, aggregate member, or nesting level above it while
decoding, before retaining that excess value or allocating an unbounded
canonical buffer. Canonical byte limits are enforced with a bounded JCS sink.

For example, a collection-owned declaration can be written as the following
typed TOML-like value. The logical id and source/take witnesses must agree with
the collection manifest; they are repeated here to make a stale or reordered
member explicit rather than silently resolving by name:

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
family-record fields:

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

After strict validation, the normalized JSON representation has exactly one of
the following two closed envelope shapes. The collection form is:

```json
{
  "schema": "urn:animsmith:schema:transition-family:1",
  "schema_version": 1,
  "scope": "collection",
  "collection_id": "com.example.pack",
  "manifest_input_identity": {"sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789", "bytes": 9876},
  "families": [{
    "family_id": "com.example.pack/transitions/walk-to-run",
    "boundary": "both",
    "basis": {"translation": "skeleton-local-metres", "rotation": "skeleton-local-degrees", "time": "normalized-clip"},
    "tolerances": {"translation_m": 0.05, "rotation_deg": 5.0, "time_normalized": 0.02},
    "members": [
      {"logical_id": "com.example.pack/locomotion/walk-forward-in-place", "source": "walk-forward", "take_index": 0, "take_name": "Take 001"},
      {"logical_id": "com.example.pack/locomotion/run-forward-in-place", "source": "run-forward", "take_index": 0, "take_name": "Take 001"}
    ]
  }]
}
```

The document form lifts the repeated table contract fields into one envelope;
the table key becomes `family_id`:

```json
{
  "schema": "urn:animsmith:schema:transition-family:1",
  "schema_version": 1,
  "scope": "document",
  "families": [{
    "family_id": "walk_to_run",
    "boundary": "entry",
    "basis": {"translation": "skeleton-local-metres", "rotation": "skeleton-local-degrees", "time": "normalized-clip"},
    "tolerances": {"translation_m": 0.05, "rotation_deg": 5.0, "time_normalized": 0.02},
    "members": [{"take_index": 0, "take_name": "Walk"}, {"take_index": 1, "take_name": "Run"}]
  }]
}
```

No owner or source-input identity is injected into the document form. A V1
evaluation record binds three distinct facts: the exact declaration-source
`InputIdentity` (the whole config for document scope or the declaration TOML
for collection scope), the normalized declaration `InputIdentity` computed
over the JCS bytes above, and the evaluated document or collection-manifest
`InputIdentity`. The collection form additionally retains its embedded exact
manifest binding. These identities are never interchangeable.

The strict reader rejects malformed/duplicate declaration members, and the
document evaluator rejects missing or ambiguous document witnesses before it
emits a result. The collection adapter additionally rejects stale or
cross-scope members. The collection
envelope's manifest digest/bytes binding makes a manifest reorder or edit
stale even when its collection id and logical ids are unchanged.

Canonical serialization first sorts decoded family declarations by their
stable `family_id` and preserves declared member order, then serializes that
typed JSON representation with RFC 8785 JCS. The exact source TOML
`InputIdentity` remains a separate binding and is never replaced by the
normalized JSON identity. The strict reader validates owner/scope, member
syntax, basis, units, finite tolerances, and canonicalization before a
declaration is accepted. The document and collection commands produce V1
findings/reports; required gameplay metadata, inferred edges, state machines,
blend trees, and runtime transition generation remain follow-up work.

### F.12 Directional-speed policy/evaluation V1 (#552)

The directional-speed policy is a separate, manifest-bound declaration for
directional-blend runtime sets. It is not a field in collection-manifest V1,
does not revise collection-output V2, and does not infer membership from
filenames, paths, engine controller types, or other host metadata. The typed
contract and strict TOML reader feed the evaluator and its JSON-only CLI
consumer; no new manifest or collection-output authority is introduced.

The schema identity is
`urn:animsmith:schema:collection-directional-speed-policy:1`, with
`schema_version = 1`. Its wire envelope has exactly these top-level fields:
`schema`, `schema_version`, `manifest`, `runtime_set_id`, `source_basis`,
`diagonal_behavior`, `direction_tolerance_deg`, `mode`, the mode-specific
fields, and `members`. `manifest` has exactly the collection-manifest V1
schema/version, `collection_id`, and `input = { sha256, bytes }`; that input
identity is the exact manifest identity, not a digest of normalized values.
`runtime_set_id` is a logical id that must bind to a manifest runtime set of
kind `directional-blend`.

The explicit `source_basis` contains `x` and `z`, each a semantic 2-D
orientation witness for the raw collection-output V3 +X/+Z endpoint
displacement. Their magnitudes are nonsemantic; the evaluator uses unit axes
for heading comparison. Every component is finite and has absolute value
at most `1,000,000`. Each axis is nonzero. Perpendicularity is judged by the
normalized dot product

`abs(dot(x / ||x||, z / ||z||)) <= 1e-9`.

The normalized calculation uses `hypot` lengths and component-wise division,
so its decision is independent of axis scale and does not square tiny values
or overflow large bounded values. The `1e-9` cosine threshold is the frozen
`COLLECTION_DIRECTIONAL_SPEED_POLICY_V1_MAX_AXIS_COSINE` bound. It rejects
near-parallel and diagonal axes, including tiny vectors, while accepting
orthogonal subnormal-scale axes when their finite representation has a
nonzero length.

`members` is an ordered sequence. Each member has exactly one logical `id`
and one semantic `coordinate = [x, z]`; it may additionally carry
`speed_mps` only for authored mode or `expected_ratio` only for ratios mode.
Coordinates are finite, bounded by the same component limit, nonzero by
component value, and unique by exact finite bit identity after canonicalizing
both signed zeros to one zero key. The reader preserves declared order. A
consumer binds the policy only when the manifest identity, runtime-set id,
runtime-set kind, and complete member sequence all match exactly; missing,
extra, or reordered members are binding errors.

The closed `diagonal_behavior` tokens are `preserve` and `normalize`. The
existing `speed_mps` and `expected_ratio` fields are unit-input/base targets.
For a semantic coordinate `c`, `preserve` defines gain `g(c) = hypot(c)` and
`normalize` defines gain `g(c) = 1`. Therefore uniform expected speed is
`base * g(c)`, authored expected speed is the member base `speed_mps * g(c)`,
and ratios compare
`expected_measured_ratio_i_to_ref = declared_expected_ratio_i * g(c_i) /
g(c_ref)`, with the reference declaration fixed at `1.0`. This gain does not
affect direction comparison. The declaration carries finite
non-negative `direction_tolerance_deg`, bounded inclusively by `180` degrees.
The closed speed modes are:

* `uniform`: requires finite non-negative bounded `uniform_speed_mps` and
  `speed_tolerance_mps`; member speed and ratio fields are forbidden.
* `authored`: requires finite non-negative bounded `speed_tolerance_mps` and
  exactly one finite non-negative bounded `speed_mps` per member; uniform and
  ratio fields are forbidden.
* `ratios`: requires finite non-negative bounded `ratio_tolerance`, a
  `reference_member`, and exactly one finite non-negative bounded
  `expected_ratio` per member. The reference member must be present and its
  expected ratio must be exactly `1.0`; speed fields are forbidden.

All scalar bounds are inclusive. Unknown fields, duplicate TOML keys,
unknown mode or diagonal tokens, missing or mode-inapplicable fields,
malformed lowercase SHA-256 identities, invalid manifest schema/version,
non-finite numbers, out-of-range components, invalid bases, invalid member
coordinates, duplicate member ids, and duplicate coordinates are strict
control errors. The source reader is capped at 8,388,608 bytes and 4,096
members; each exact limit is accepted and the first excess member is rejected
before it is retained. The reader first decodes only `schema` and
`schema_version` so unsupported declarations receive stable unsupported-schema
or unsupported-version classifications; it then performs the complete
deny-unknown-fields decode and typed invariant validation. This two-pass
lifecycle is classification, not a second policy authority.

There is no evaluator inference or fallback. The pure, format-neutral
`collection-directional-speed-evaluation:1` result and typed adapter consume
already strictly decoded collection-output V3 evidence; they do not revise V3.
The immutable result binds the raw policy TOML
and evidence JSON `InputIdentity` values as well as the manifest, and retains
every member's coordinate, raw root-travel values, projected heading,
comparison values, pass/violation state, lifecycle, and gaps.

For complete evidence, finite endpoint X/Z displacement is normalized before
the unit source X/Z axes form its semantic heading. The declared coordinate is
also normalized; the included angle is `atan2(abs(cross), dot)` in degrees.
Speed uses published `speed_mps`, never travel. Every tolerance is inclusive,
so a finding requires strict excess. A zero endpoint, zero ratio reference, or
unrepresentable derived product/ratio yields a complete typed not-evaluated
result (`zero_net_displacement`, `zero_reference_speed`, or `numeric_range`)
without a subset or fabricated finding. Preserve uses `hypot(coordinate)` as
the expected-speed gain; normalize uses one, and the ratio reference remains
exactly one. Findings retain manifest order, direction before speed/ratio.

`animsmith collection evaluate-directional-speed --policy POLICY.toml --evidence
COLLECTION-OUTPUT.json --format json` captures policy bytes under the 8 MiB
cap and evidence bytes under the 256 MiB cap before retaining them, invokes
the strict readers/adapter/evaluator, and serializes the result once to stdout.
Invalid, stale, wrong-kind, unreadable, malformed, or over-budget control
inputs exit 2 with no result. Incomplete/not-evaluable evidence and policy
findings emit the result and exit 1; only a complete passing policy exits 0.
There is no filesystem publication, controller generation, retiming, stride
rewriting, or universal speed rule.
