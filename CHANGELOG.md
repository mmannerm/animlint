# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.2](https://github.com/mmannerm/animsmith/compare/v0.13.1...v0.13.2) - 2026-09-12

### Fixed

- *(foot-cycle)* let the proof derive every stored time, and refuse a knot pair that names one instant through a key
- *(foot-cycle)* make the span edges and the knot-collision rule symmetric
- *(foot-cycle)* decide authored-key coincidence once, in the emitted binary32 domain
- *(foot-cycle)* compare a knot against both instants of the knot before it
- *(foot-cycle)* keep the map's exact endpoints authoritative at a track's span ends

## [0.13.1](https://github.com/mmannerm/animsmith/compare/v0.13.0...v0.13.1) - 2026-09-03

### Fixed

- *(docs)* refuse a staging root override that names the repository

## [0.13.0](https://github.com/mmannerm/animsmith/compare/v0.12.0...v0.13.0) - 2026-09-03

### Added

- *(report)* play two clips of one document side by side at a shared phase
- *(report)* overlay a gait group's members on one phase-aligned figure
- *(cli)* name the compiled features in --version
- *(report)* overlay the after skeleton on the before pane

### Fixed

- *(core)* validate a gait group's tolerances on the embedding path too
- *(report)* carry gait-group membership as data, not a delimited attribute
- *(report)* keep each comparison pane's own label under the overlay

## [0.12.0](https://github.com/mmannerm/animsmith/compare/v0.11.1...v0.12.0) - 2026-09-03

### Added

- *(report)* derive the guidance from what each clip declares, and mark every trail
- *(report)* play the comparison's shared phase
- *(report)* caption what to look for, and mark and size the root path

### Fixed

- *(foot-cycle)* preserve phase-zero proof identity
- *(foot-cycle)* admit cyclic contact topology
- *(foot-cycle)* deduplicate authored binary32 knots
- *(report)* allocate end-mark lanes from where marks are plotted
- *(report)* pause on a deep link, mark both root tracks, narrow the trail claim
- *(report)* fit the comparison's root panel and state its scale
- *(report)* keep a root track's end marks visible on a closed loop
- *(report)* drop the comparison root panel's empty strip

## [0.11.1](https://github.com/mmannerm/animsmith/compare/v0.11.0...v0.11.1) - 2026-09-02

### Fixed

- *(test)* match config identifiers whole instead of by prefix
- *(docs)* repair the contract page's examples and pin what the review found
- *(report)* hide the playhead dot on a frame with no sampled position
- *(report)* plot the root path from jointly finite samples
- *(report)* sample deterministically, and read captions from the document
- *(report)* make the comparison panels legible and their pairs distinct
- *(report)* scale the gait chart's two signals on their own axes

## [0.11.0](https://github.com/mmannerm/animsmith/compare/v0.10.0...v0.11.0) - 2026-09-02

### Added

- *(report)* add --evidence-only reports without the sampled motion
- *(report)* make the metric charts legible on their own
- *(report)* honour URL fragment options in both viewers
- *(report)* theme every report surface from one token set

### Fixed

- *(report)* unify the document surfaces and unbind evidence-only work
- *(report)* free evidence-only comparisons from the embedded pose budget
- *(report)* restore documented defaults and notice every omitted panel
- *(report)* keep pinned switches and read every fragment pair

## [0.10.0](https://github.com/mmannerm/animsmith/compare/v0.9.0...v0.10.0) - 2026-09-01

### Added

- publish declared foot-cycle generations ([#643](https://github.com/mmannerm/animsmith/pull/643))
- add no-replace generation directories ([#640](https://github.com/mmannerm/animsmith/pull/640))
- prove serialized foot-cycle candidates
- bind foot-cycle proof policy ([#638](https://github.com/mmannerm/animsmith/pull/638))
- prepare source-bound foot-cycle candidates ([#636](https://github.com/mmannerm/animsmith/pull/636))
- plan declared foot-cycle parameterization
- compare declared skeleton compatibility
- build foot-cycle clip candidates ([#634](https://github.com/mmannerm/animsmith/pull/634))
- implement strict contact transform contract
- bound strict in-memory GLB candidates

### Fixed

- share scene-root shape validation across transform producers ([#639](https://github.com/mmannerm/animsmith/pull/639))
- transfer proved foot-cycle artifacts into publication
- close foot-cycle reciprocal audit blockers
- close foot-cycle producer audit gaps
- harden foot-cycle producer contracts
- complete foot-cycle proof audit coverage
- close foot-cycle proof audit gaps
- preflight proof work before serialization
- repair merged Windows and contact integration
- complete skeleton comparison report
- scope skeleton evidence facets
- keep skeleton comparison panic-free
- tighten skeleton comparison row contract
- canonicalize skeleton comparison identity bytes
- canonicalize skeleton comparison evidence
- bind skeleton mappings to selectors
- make skeleton comparison selection explicit
- *(docs)* publish animation pack report pages
- *(core)* bound foot-cycle evidence comparability
- *(core)* bound and gate foot-cycle planning
- enforce contact transform proof context
- *(gltf)* harden strict GLB candidate preflight
- *(engine)* pin exact Bevy 0.19.0 readback graph

## [0.9.0](https://github.com/mmannerm/animsmith/compare/v0.8.0...v0.9.0) - 2026-08-30

### Added

- *(report)* complete visual acceptance matrix
- *(report)* add synchronized visual comparison
- *(report)* add collection dashboard
- *(core)* report canonical rest-world rotation
- *(engine)* add isolated Bevy readback probe

### Fixed

- preflight Pages composition paths
- *(engine)* remove unreachable non-Windows fallback
- *(report)* bind comparison evidence authorities
- *(report)* harden comparison authority boundaries
- *(report)* preflight and publish comparisons safely
- make configuration reference maintenance package-safe
- *(report)* retain unmapped dashboard prediction evidence
- *(report)* reconcile dashboard take identity
- *(report)* retain physical take evidence
- *(report)* preserve dashboard source evidence
- *(report)* keep dashboard reader tests typed
- *(report)* harden dashboard authority readback
- *(report)* align transition dashboard readback
- *(report)* avoid dashboard schema runtime dependency
- harden Pages staging and pinned builds
- contain Pages staged links
- harden Pages external reference proxies
- proxy external Pages navigation links
- bind check reference docs to implementations
- *(core)* harden rest rotation projection
- *(core)* report polar rest-world rotation
- report pinned measurement reader expectations
- *(engine)* harden Windows Bevy snapshot intake
- *(engine)* harden Bevy readback boundaries
- *(engine)* finalize verified Bevy snapshots
- *(engine)* harden exact Bevy readback
- *(engine)* tighten Bevy readback audit coverage
- *(report)* keep testkit dependency package-local
- *(report)* bound comparison context rendering
- *(report)* make comparison finding anchors semantic

### Changed

- add canonical proper-orthogonal-polar `rotation_xyzw` rest-world and
  skin-bind linear facts in measurements-v18; make output-v19 and
  collection-output-v11 the current contracts while retaining their pinned
  historical readers

## [0.8.0](https://github.com/mmannerm/animsmith/compare/v0.7.0...v0.8.0) - 2026-08-29

### Fixed

- *(report)* bind evaluations to current collection output
- preserve finite per-bone loop-continuity evidence when another bone is non-finite

### Changed

- make measurements-v17, output-v18, and collection-output-v10 the current contracts

## [0.7.0](https://github.com/mmannerm/animsmith/compare/v0.6.0...v0.7.0) - 2026-08-26

### Added

- *(generate)* emit glTF addressability v2
- integrate revisioned import advice
- predict Unity Generic root-motion routing
- predict Bevy animation import gates
- *(lint)* predict Bevy unit scale
- add primitive and image evidence
- retain raw glTF addressability evidence
- *(core)* version exact timing prediction contracts
- *(fbx)* retain exact timing evidence
- project rich Bevy glTF addressability
- add standalone engine import advice V2
- add Godot and Unreal advice profiles
- *(engine)* add current V3 addressability prediction
- *(engine)* check exact Unreal clip boundaries
- add measurements-v16 per-primitive geometry and unsupported-image evidence
- add revisioned output-v13 through output-v17 and collection-output-v5 through collection-output-v9 contracts
- add output-v12 bounded engine prediction provenance and partial settings evidence
- preserve collection-output-v4 embedding the historical output-v12 lint envelope

### Fixed

- close rich addressability audit gaps
- close import advice refusal lifecycle
- bound root-motion evidence projection
- preserve root-motion N/A in collections
- close Unity root-motion lifecycle gaps
- bound overflow prediction evidence
- *(collection)* bind evidence to dependency closure
- *(core)* bind clip-boundary provenance
- *(core)* rederive clip-boundary applicability
- *(core)* validate exact clip-boundary readback
- enforce bounded prediction evaluation
- *(fbx)* fix exact timing test iterator lint
- use native values for V2 import advice
- bind import advice projections
- bind root trajectory display identity

### Changed

- make measurements-v16, output-v17, and collection-output-v9 the current contracts
- retain output-v12 through output-v16 and their collection envelopes as immutable historical contracts

## [0.6.0](https://github.com/mmannerm/animsmith/compare/v0.5.0...v0.6.0) - 2026-08-25

### Added

- add collection transition-pose evaluation
- add transition-pose document CLI
- render evaluation model views
- admit transition-family declarations in config
- add strict evaluation model v1
- add transition-family declaration v1
- evaluate collection directional speed policies
- generate strict contact fragments
- evaluate directional speed policies
- add contact fragment v1 core reader
- add directional speed policy v1 reader
- add transition-pose core evaluation v1

### Fixed

- bound collection transition admission
- bind collection transition manifests
- harden collection transition-pose bounds
- bind collection transition poses to source closures
- bind transition-pose CLI to source closure
- harden evaluation model renderer
- validate collection closure state
- bind transition evaluation to dependency closure
- align transition-pose evaluator with v1 contract
- reject mismatched standalone FBX scale correspondence

## [0.5.0](https://github.com/mmannerm/animsmith/compare/v0.4.4...v0.5.0) - 2026-08-24

### Added

- add role-aware rig alias resolution
- publish collection root-travel evidence
- publish collection gait phase evidence
- execute collection lint manifests ([#550](https://github.com/mmannerm/animsmith/pull/550))
- *(cli)* parse and resolve collection manifests
- *(core)* define collection manifest values

### Fixed

- bind rig role policy provenance
- make glTF clip admission role-aware
- fail closed rig profile resolution
- keep explicit-only auto rig resolution
- *(core)* classify unresolved gait roles consistently

## [0.4.4](https://github.com/mmannerm/animsmith/compare/v0.4.3...v0.4.4) - 2026-08-24

### Fixed

- *(assemble)* preserve pruning evidence namespace
- *(assemble)* project document-local reference identities
- *(assemble)* keep FBX selection refusals recipe-facing
- *(assemble)* carry FBX mesh selection through staging
- *(assemble)* project clip-only source domains
- *(fbx)* admit display layer metadata
- *(assemble)* admit fully keyed clip rest
- *(assemble)* scope skinless clip basis to its rig
- *(assemble)* rebase skinless clip tracks ([#524](https://github.com/mmannerm/animsmith/pull/524))
- *(assemble)* compose FBX scale with node removal
- correct 'undirivable' to 'underivable' in loop_seam_ratio docstring

## [0.4.3](https://github.com/mmannerm/animsmith/compare/v0.4.2...v0.4.3) - 2026-08-22

### Fixed

- *(assemble)* resolve ancestor-owned skins

## [0.4.2](https://github.com/mmannerm/animsmith/compare/v0.4.1...v0.4.2) - 2026-08-22

### Fixed

- *(fbx)* admit scale-invariant rest-bind fidelity

## [0.4.1](https://github.com/mmannerm/animsmith/compare/v0.4.0...v0.4.1) - 2026-08-22

### Fixed

- *(engine)* print config keys in diagnostics
- *(fbx)* reconcile bind poses for rest-bind scale

## [0.4.0](https://github.com/mmannerm/animsmith/compare/v0.3.1...v0.4.0) - 2026-08-21

### Added

- admit nonbearing FBX node attributes
- add bounded engine import advice ([#497](https://github.com/mmannerm/animsmith/pull/497))
- *(generate)* add glTF addressability inventory
- *(engine)* predict Bevy animation labels
- *(engine)* publish prediction provenance contract
- *(source)* capture dependency closure identities
- *(engine)* add strict profile registry and resolution
- *(core)* project importer-sensitive source facts
- *(config)* declare per-axis movement ownership
- *(measure)* expose channel and root trajectory evidence
- *(assembly)* select rest-bind scale by name
- *(assembly)* stage eligible FBX rest-bind inputs
- *(fbx)* enable proved rest-bind scale
- *(assemble)* compose rest-bind scaling with assembly transforms
- *(contract)* advance to measurements-v14/output-v8

### Fixed

- *(publish)* compare destination names by filesystem
- *(fbx)* compare missing dependency names by filesystem
- *(fbx)* reject truncated dependency publication
- *(fbx)* fail closed on unsafe dependency paths
- *(fbx)* protect every retained dependency path
- *(fbx)* protect captured dependencies from publication
- *(fbx)* unify rest-bind admission authority
- *(fbx)* preserve admitted linked textures
- *(fbx)* narrow rest-bind capability refusals
- *(generate)* reject truncated animation inventories
- *(generate)* bound addressability readback allocation
- *(engine)* validate addressability evidence
- *(release)* normalize inventory target paths
- *(release)* validate docs.rs target metadata
- *(measure)* correct loop-seam availability and widen diff coverage
- *(core)* bound inherited contract decoding
- *(measure)* classify flat gait phase as not applicable
- *(core)* close prediction readback gaps
- *(core)* satisfy current clippy
- *(loaders)* preserve redacted resource presence
- *(config)* reject invalid check tolerances
- *(measure)* report loop_seam_ratio as not_applicable without a real stride
- *(measure)* use sibling availability fields instead of ClipFact<T>
- *(gltf)* detect undeclared extension payloads
- *(source)* keep dependency closure conservative
- *(gltf)* honor explicit resource root capability
- *(fbx)* model derived texture file aliases

## [0.3.1](https://github.com/mmannerm/animsmith/compare/v0.3.0...v0.3.1) - 2026-08-18

### Fixed

- *(transform)* support vertical gait heading bases

## [0.3.0](https://github.com/mmannerm/animsmith/compare/v0.2.1...v0.3.0) - 2026-08-17

### Added

- *(measure)* harden inverse-bind evidence
- *(cli)* type producer asset refusals
- *(fbx)* inventory scale capabilities without enablement
- *(assemble)* add rest-bind scale integration
- *(scale)* support glTF position morph scaling

### Fixed

- *(release)* verify generated version docs
- *(gltf)* reject unnormalized integer attributes
- *(cli)* make producer load policy universal
- *(cli)* preserve typed producer load failures
- *(cli)* preserve checked transcript semantics
- *(cli)* stream checked command transcripts
- *(cli)* check parser and fix report delivery
- *(cli)* check text stdout writes
- *(fbx)* make capability inventory fail closed
- *(assemble)* apply clip operations in rebased basis
- *(assemble)* harden rest-bind scale evidence
- *(core)* close gait anchor verification gaps
- *(transform)* close gait-anchor safety gaps
- *(transform)* require lossless gait trajectory grids
- *(transform)* refuse unsafe gait anchoring
- *(scale)* union early morph refusals
- *(scale)* preserve located morph refusals
- *(scale)* publish complete morph capability
- *(fbx)* preserve incomplete source evidence
- *(core)* make gait anchoring lossless across frame rates
- *(transform)* close gait trajectory sampling gaps
- *(scale)* compare effective unaffected binds
- *(gltf)* handle compact matrix accessor ranges
- *(gltf)* preserve unreferenced accessor payloads
- *(fbx)* preserve shared mesh definitions
- *(fbx)* inventory empty mesh definitions
- *(fbx)* preserve omitted source identities
- *(fbx)* inventory omitted mesh payloads

## [0.2.1](https://github.com/mmannerm/animsmith/compare/v0.2.0...v0.2.1) - 2026-08-16

### Fixed

- *(gltf)* preserve exact raw JSON numbers ([#403](https://github.com/mmannerm/animsmith/pull/403))

## [0.2.0](https://github.com/mmannerm/animsmith/compare/v0.1.0...v0.2.0) - 2026-08-16

### Added

- *(assemble)* [**breaking**] remove selected node subtrees
- *(assemble)* [**breaking**] expose constant-track pruning
- *(scale)* [**breaking**] rebase rest-bind scale animation
- *(cli)* serialize each evidence record once and give assemble --format ([#340](https://github.com/mmannerm/animsmith/pull/340))
- *(core)* refuse a document whose two parent chains disagree ([#331](https://github.com/mmannerm/animsmith/pull/331))
- *(core)* count what each residual actually compared ([#322](https://github.com/mmannerm/animsmith/pull/322))
- *(cli)* publish scale artifacts and evidence as an atomic pair
- *(core)* add scale planning and proof contracts
- *(lint)* validate selected rest-world scale
- *(measure)* expose transform scale domains
- prune provably constant animation tracks
- detect time-complement sync pairs
- add sync-group timing diagnostics
- add required bone presence check
- add angular loop seam continuity
- detect duplicate loop endpoints
- add loop continuity checks
- measure mesh geometry centroid
- measure skeleton rest-pose domains
- report material and image measurements
- report additional skin influence sets
- add pinned clip duration expectations
- *(inspect)* inventory mesh instances
- *(cli)* verify complete assembly publication
- support PBR material texture slots
- *(cli)* add recipe-driven character assembly
- *(convert)* apply material texture recipes
- *(convert)* bake static mesh transforms
- *(convert)* preserve normal textures
- *(measure)* define static asset bound domains
- [**breaking**] finalize result contract v2
- add provisional v2 evaluation results
- *(core)* [**breaking**] share affine classification
- *(core)* make the proof record correct before it is frozen
- *(core)* publish the observed factor and prove unaffected binds
- *(core)* derive the scale tolerance policy and bound proof work
- *(gltf)* rewrite whole-document linear units on raw bytes
- *(core)* canonicalize skinned bind poses
- *(core)* add character assembly clip operations
- *(gltf)* reparameterize animated rest and bind hierarchies on raw bytes
- *(gltf)* add raw scale capability preflight
- bind reports to input bytes

### Fixed

- *(scale)* pair proof residual measurements
- *(measure)* close final audit gaps
- *(measure)* [**breaking**] reconcile shared affine facts
- *(core)* [**breaking**] canonicalize the affine scale mean
- *(scale)* make artifact diagnostics structural
- *(scale)* expose artifact proof differences
- *(scale)* accumulate parent-chain rounding provenance
- *(scale)* prove unchanged skeleton placement
- *(scale)* reject negative primary skin weights
- *(core)* tolerate f32 rounding a rotation hid from the comparison base ([#333](https://github.com/mmannerm/animsmith/pull/333))
- *(cli)* flush a published temp through a writable handle
- *(cli)* a symlinked input must not be read through and then overwritten
- *(core)* close fail-open and precision defects in scale proof
- report emissive material texture bindings
- honor inclusive loop continuity caps
- align skeleton schema states
- simplify skeleton coverage states
- harden skeleton measurement validation
- resolve encoded external resource paths
- preserve per-primitive influence mismatches
- reject invalid duration pins
- *(inspect)* emit copyable TOML selectors
- *(inspect)* mirror assembly name ambiguity
- *(cli)* reject linked assembly textures
- *(cli)* gate assembly config provenance
- *(cli)* simplify loaded config evidence
- *(cli)* bind assembly config evidence
- *(cli)* satisfy assembly lint gates
- *(lint)* separate scale diagnostic ownership
- *(convert)* harden recipe path and image contracts
- *(convert)* preserve normal texture bake state
- *(convert)* harden static bake validation
- *(cli)* stream human-readable output
- preserve diff cardinality error precedence
- address final result-contract audit
- close final contract audit gaps
- enforce schema-valid embedded results
- validate embedded rig evidence
- close final result contract audit gaps
- close audit coverage gaps
- address result contract review findings
- harden final result contract
- close result contract audit gaps
- preserve v1 contracts in evaluation preview
- *(core)* widen connector planning spans
- *(core)* widen bridged linear rebase
- *(core)* widen bridged translation sum
- *(core)* close connector proof gaps
- *(core)* compose rest-bind through static connectors
- *(scale)* bind calibration to production comparisons
- *(scale)* prove mesh-instance placement, not just its payloads ([#324](https://github.com/mmannerm/animsmith/pull/324))
- *(core)* retract the impossibility claim and pin what it hid
- *(core)* bound the candidate side and correct the policy's own arithmetic
- *(core)* make the closure property true and charge the work proof does
- *(core)* report the rotation residual in the unit it declares
- *(lint)* preserve large scale ordering
- *(lint)* quantize derived scale evidence
- *(lint)* honor authored scale boundaries
- stabilize milliradian track comparisons
- preserve empty clip warning behavior
- retain expected duration on degenerate clips
- *(core)* derive skinned geometry bind space
- *(convert)* order bake evidence by source node
- bind rig evidence to resolved skeleton
- *(gltf)* preserve valid weight encodings
- *(gltf)* reject short resolved primitive buffers
- *(gltf)* refuse truncated primitive accessors
- *(gltf)* refuse geometry accessors the reader cannot decode ([#326](https://github.com/mmannerm/animsmith/pull/326))
- *(gltf)* stop reading JSON null as a declared member, and wire the guards
- *(gltf)* move two source-validity checks into the capability gate
- *(gltf)* refuse out-of-contract nodes and pin the proof's claims
- keep image dimensions policy-neutral

### Added

- *(convert)* preserve normal textures through FBX ingestion and glTF scene round-trips, including glTF normal scale ([#222](https://github.com/mmannerm/animsmith/issues/222))
- *(gltf)* add `rewrite_scale_plan`, a plan-taking raw scale writer for callers
  that reuse one compiled core plan across rewrite and artifact proof
  ([#374](https://github.com/mmannerm/animsmith/issues/374))

### Fixed

- *(gltf)* refuse primitive accessors whose dense or sparse byte extent their
  declared buffer view or resolved buffer bytes cannot satisfy, instead of
  silently retaining a primitive with empty positions or indices
  ([#329](https://github.com/mmannerm/animsmith/issues/329))
- *(gltf)* reject animation sampler accessors whose declared type or component
  type does not match the property-selected reader, preventing malformed
  inputs from panicking or silently reinterpreting same-sized elements while
  retaining all five legal quaternion and morph-weight component encodings
  ([#327](https://github.com/mmannerm/animsmith/issues/327))
- *(core)* allow rest/bind scale planning to compose through finite static
  source-node connectors between projected joints while preserving each
  connector local transform, rebasing only its projected successor, and
  independently proving that connector span's exact raw write set
  ([#332](https://github.com/mmannerm/animsmith/issues/332))

### Changed

- *(docs)* consolidate the shipped scale workflow in a user-facing guide,
  retain Appendix D as the normative algebra and safety contract, move
  calibration/history to a separate reproducibility note, and update crate
  rustdoc, README, embedding, CLI, and output references to the current
  plan-before-rewrite and emitted-artifact proof pipeline
  ([#343](https://github.com/mmannerm/animsmith/issues/343))
- *(core)* continue the behavior-neutral scale module split by moving the
  public facade to `scale/mod.rs`, its complete lib-target test and calibration
  graph to private `scale/tests.rs`, and the shared policy-neutral matrix
  rewrite, residual, and arithmetic-provenance leaves to private
  `scale/numeric.rs`; shared scale-input and candidate validation, canonical
  source topology/domain derivation, world-pose and inverse-bind readers, and
  one-time affected-skin classification now live in private
  `scale/validation.rs`; operation planning, typed field/payload/obligation
  ledger compilation, sampled-evidence declaration, and complete structural
  replay now live in private `scale/planning.rs`; the candidate wrapper,
  analytic fixture builder, and writer-owned direct/connector rebase arithmetic
  now live in private `scale/reference.rs`; proof-owned source/connector and
  animation expectations, exact field discharge, sampled work budgeting,
  semantic residuals, and skin/bounds checks now live in private
  `scale/proof.rs`, with residual mutation confined to its nested private
  `residual` module, without changing public paths, ordering, policy, evidence,
  or proof behavior
  ([#383](https://github.com/mmannerm/animsmith/issues/383))
- [**breaking**] *(core)* remove the production-looking public
  `build_scale_candidate` API. Format frontends remain responsible for exact
  source rewriting and pass the emitted reload through
  `ScaleCandidate::from_document` to independent core proof; analytic tests
  can opt into the renamed reference constructor under the existing
  non-default `fixtures` feature. Scale behavior, evidence v1/v2/v3, and the
  `appendix-d-v6` policy remain unchanged
  ([#381](https://github.com/mmannerm/animsmith/issues/381))
- [**breaking**] *(core)* replace each public `ScaleProof` maximum/count field
  pair with one read-only `ScaleProofResidual`. Its `max()`, `comparisons()`,
  and `evaluated()` accessors keep a claim's measurements mechanically paired;
  scale-evidence v1/v2/v3 JSON and tolerance behavior remain unchanged
  ([#323](https://github.com/mmannerm/animsmith/issues/323))
- [**breaking**] *(core)* replace the public scale domain-rewrite and
  proof-obligation boolean bags/accessors with a non-exhaustive read-only typed
  plan ledger covering global canonical topology, numeric-value-free payload
  shape, explicit write-ownership dispositions with structural rewrite rules,
  and derived proof obligations. Scale-evidence v1/v2/v3 (including v3's five
  domain booleans), `ScaleProof` serialization, and the `appendix-d-v6` policy
  remain unchanged ([#374](https://github.com/mmannerm/animsmith/issues/374))
- *(gltf)* drive both raw scale writers from the compiled typed plan, pass one
  immutable plan through CLI rewrite and artifact proof, cross-check raw node
  hierarchy against canonical source topology, validate the complete plan
  inventory before raw replay, and keep proof-owned component selection and
  numeric derivations independent. At factor one, length-bearing node fields,
  accessor payloads, and authored bounds are now excluded from the raw write
  set under their compiled `PreserveExact` disposition. Parsed JSON numeric
  values therefore avoid narrowing through `f32`, and accessor bytes remain
  authored; JSON reserialization can still canonicalize lexical spelling.
  Otherwise equivalent artifact bytes and their publication digest can
  therefore change, and artifact proof checks factor-one node JSON and raw
  complement values exactly. Evidence v1/v2/v3 schema identities and shapes,
  selectors, tolerance policy, and refusal kinds remain unchanged; factor-one
  v3 rewrite inventories and `rewritten_accessor_count` now report the empty
  compiled write set
  ([#374](https://github.com/mmannerm/animsmith/issues/374))
- [**breaking**] *(assemble)* advance the character-assembly recipe and
  evidence contracts to immutable v3. Recipes can exact-name base nodes for
  fail-closed subtree removal after animation transforms, and evidence records
  every removed node in original hierarchy order. Recipe/evidence v1 and v2
  remain immutable historical contracts ([#350](https://github.com/mmannerm/animsmith/issues/350))
- [**breaking**] *(assemble)* advance the character-assembly recipe and
  evidence contracts to immutable v2. Recipes can opt into constant-track
  pruning after all other transforms; evidence records each removed track by
  original index, exact output bone identity, TRS property, interpolation, and
  key count. Recipe/evidence v1 remain immutable historical contracts, and the
  new recipe option defaults to `false` ([#349](https://github.com/mmannerm/animsmith/issues/349))
- [**breaking**] *(scale)* advance scale evidence to immutable v3: rest/bind
  now rebases valid affected-node scale animation (including cubic-spline
  tangents), publishes `result.domain_rewrites.scale_animation`, and raw glTF
  preflight refuses animation channels targeting a node authored with `matrix`.
  The immutable v1/v2 schemas remain historical, and the pre-1.0 public
  `ScaleError::AffectedScaleAnimation` variant is removed ([#352](https://github.com/mmannerm/animsmith/issues/352))
- [**breaking**] *(scale)* advance the Appendix D tolerance policy to
  `appendix-d-v6`: finite widened affine axis lengths are sorted ascending
  before averaging, making classification and observed factors independent of
  authored axis order; the pre-1.0 `APPENDIX_D_V5` associated constant is
  removed ([#361](https://github.com/mmannerm/animsmith/issues/361))
- [**breaking**] *(output)* advance measure, lint, and diff JSON to output v6
  and publish corrected affine linear-transform observations as measurements
  v12. The JSON shapes and vocabularies are unchanged, but the outer immutable
  identity advances because its schema statically pins the nested measurement
  URN; current `diff` rejects v5/v11 reports and directs operators to regenerate
  them from the original asset ([#355](https://github.com/mmannerm/animsmith/issues/355))
- [**breaking**] *(core)* expose typed `validate_document_shape` snapshot
  validation and collapse scale's former structural error variants into
  `ScaleError::InvalidDocumentShape` ([#293](https://github.com/mmannerm/animsmith/issues/293))
- [**breaking**] *(core)* move `AffineDomainViolation` to its canonical root/model path and remove the former `scale` module path ([#354](https://github.com/mmannerm/animsmith/pull/354))

## [0.1.0](https://github.com/mmannerm/animsmith/releases/tag/v0.1.0) - 2026-07-11

### Added

- *(measure)* mesh-level measurements from SceneAssets ([#16](https://github.com/mmannerm/animsmith/pull/16))
- *(gltf)* parse meshes/skins/materials into SceneAssets ([#16](https://github.com/mmannerm/animsmith/pull/16))
- [**breaking**] fix --dry-run is the repair check mode; drop repair groups
- polish public api and release readiness
- weld converted meshes into indexed primitives and embed textures
- convert carries meshes, skins, and materials (FBX2glTF replacement)
- add transform subcommand — slice, hold-extend, gait-anchor rotation
- add fix subcommand with quat hemisphere normalization
- M2 part 2 — self-contained HTML report with WebGL skeleton viewer
- M1 — rig profiles, config, and reference-parity semantic checks
- bootstrap animlint M0 — workspace, core model, glTF ingest, mechanical checks

### Fixed

- address writer summary review
- report converted artifact counts
- address release target audit findings
- *(examples)* make the walk fixture byte-stable across platforms
- sanitize non-finite mesh measurements; skip non-triangle prims ([#16](https://github.com/mmannerm/animsmith/pull/16))
- *(gltf)* guard mesh/skin accessor reads against count-0 panic ([#16](https://github.com/mmannerm/animsmith/pull/16))
- [**breaking**] unify SceneAssets into Document so transform keeps meshes
- compose selected repairs in fix pipeline
- honor min stride step config
- [**breaking**] reject malformed track data at load; panic-free sampling; GLB external-buffer fix writes
- treat sub-0.05 gait-phase drift as diff noise
- enforce schema_version in diff ingestion; honest FixError classification
- reject --dry-run with a write target; pin removed-flag and skip semantics
- address publishing audit findings
