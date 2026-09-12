# animsmith examples

A cookbook of runnable workflows. Each section is a self-contained task
you can copy into your own project — several double as CI/acceptance
gates. Each workflow addresses a runtime failure described in
[what game-ready means](../docs/game-ready-clips.md), which explains
*why* the checks behind these commands exist. For the larger process
that turns raw source assets into engine-facing outputs, see the
[pipeline scenario guide](../docs/pipeline-scenarios.md).

Commands that reference [`examples/assets/`](assets/) run
against small assets committed there, so you can follow along from a
source checkout with no downloads; the assets are procedurally
generated (see [their README](assets/README.md) for
provenance and how to regenerate them). The conversion and reporting
section operates on assets you supply, using placeholder filenames —
`export.fbx`, `old.glb` — for your own exports and baselines.

For complete contract semantics, see the
[configuration reference](../docs/configuration-reference.md).

Console transcripts are real command output, with long finding messages
elided as `...`. JSON envelopes are abridged illustrative projections with
placeholder digests, byte counts, and build provenance; capture those
identity fields from your own run. Exit-code annotations remain `# comments`.

## Running the commands

Examples use the installed CLI form, `animsmith <command>`. From a
source checkout, prefix each command with `cargo run -p animsmith --`:

```console
animsmith lint examples/assets/clip.glb
cargo run -p animsmith -- lint examples/assets/clip.glb   # source checkout
```

Two examples need the default build's feature-gated commands (`report`,
`convert`); they are marked **default features only**. Everything else
works in the pure-Rust `--no-default-features` build too.

## Exit codes

Every example relies on the same convention, so scripts can gate on it:

| Code | Meaning |
|---:|---|
| 0 | No failing findings and no required-unavailable engine prediction facets; warnings, notes, and ordinary coverage gaps may remain. |
| 1 | A failing finding, any `required_prediction_unavailable` facet, a significant `diff`, or pending `fix --dry-run` repairs. |
| 2 | Operator error: unreadable input, bad config, bad flags. |

---

## 1. A first CLI gate

When a clip enters CI, the first question is not just whether the file
parses — it is whether the motion is safe to ship. A gate built from
`inspect`, `measure`, and `lint` catches the gap described in
[a valid file is not a usable clip](../docs/game-ready-clips.md#a-valid-file-is-not-a-usable-clip):
valid containers can still hide content problems that only show up
after import, bake, or playtest. `inspect` summarizes structure;
`measure` reports metrics without judgment; `lint` runs the checks and
sets the exit code.

```console
$ animsmith inspect examples/assets/clip.glb
examples/assets/clip.glb
rig profile: none detected
skeleton: 3 bones
  root
    spine
      chest
materials: 0
mesh instances: 0
clips: 1
  swing: 1.000s, 1 tracks, 5 keys max

$ animsmith lint examples/assets/clip.glb
examples/assets/clip.glb:
  coverage[bind-pose] insufficient_rotation_evidence ×1 (scopes: first_frame_rest_delta; ...
0 error(s), 0 warning(s), 0 note(s), 1 coverage gap(s), 0 available prediction facet(s), ...   # exits 0
```

A defective asset produces findings and a non-zero exit:

```console
$ animsmith lint examples/assets/clip-dirty.glb
examples/assets/clip-dirty.glb:
  error[quat-norm] clip 'swing' bone 'spine' @0.500s: non-unit rotation key ...
  warning[quat-flip] clip 'swing' bone 'spine' @0.750s: 2 hemisphere flip(s) ...
  coverage[bind-pose] insufficient_rotation_evidence ×1 (scopes: first_frame_rest_delta; ...
1 error(s), 1 warning(s), 0 note(s), 1 coverage gap(s), 0 available prediction facet(s), ...   # exits 1
```

Warnings alone keep the exit code at 0. Use `--deny-warnings` when CI
should fail on warnings too:

```console
$ animsmith lint --deny-warnings examples/assets/clip-dirty.glb   # exits 1
```

For machine consumption, `--format json` emits the current v19 result envelope
(see [output.md](../docs/output.md)). This `jq` projection keeps the example
short while showing where retained/promotion evidence, content findings, and
independently versioned measurement evidence live:

```console
$ animsmith lint --format json examples/assets/clip-dirty.glb | jq \
    '{schema_version, schema, command,
      prediction_facets: .summary.prediction_facets,
      prediction_provenance: .files[0].prediction_provenance,
      input: .files[0].input,
      check: (.files[0].checks[] | select(.check_id == "quat-norm") |
        {check_id, selection, configuration, applicability, evaluation, findings}),
      measurements: (.files[0].measurements | {schema_version, schema})}'
{
  "schema_version": 19,
  "schema": "urn:animsmith:schema:output:19",
  "command": "lint",
  "prediction_facets": {
    "available": 0,
    "required_prediction_unavailable": 0
  },
  "prediction_provenance": null,
  "input": {
    "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "bytes": 123456
  },
  "check": {
    "check_id": "quat-norm",
    "selection": "selected",
    "configuration": "enabled",
    "applicability": "applicable",
    "evaluation": "complete",
    "findings": [
      { "check_id": "quat-norm", "severity": "error", "clip": "swing",
        "bone": "spine", "time_s": 0.5, "measured": 1.05, "expected": 1.0,
        "message": "non-unit rotation key (worst at key 2)" }
    ]
  },
  "measurements": {
    "schema_version": 18,
    "schema": "urn:animsmith:schema:measurements:18"
  }
}
```

### As a CI gate

Lint every clip and fail the job on any error (add `--deny-warnings` to
also fail on warnings):

```yaml
# .github/workflows/animate.yml
- run: cargo install animsmith --no-default-features
- run: animsmith lint --deny-warnings clips/*.glb
```

The step's exit code is the gate: 1 fails the job, 0 passes it.

---

## 2. Repairing an asset

Use repair when a clip plays as a sudden flicker, spin, or explosion,
but the authored motion is still recoverable. The symptoms in
[the pose flickers, spins, or explodes](../docs/symptoms/pose-flickers.md)
are often quaternion representation problems, so `fix --dry-run` lets a
gate report the exact lossless repairs before writing anything.

`quat-norm` and `quat-flip` are not just checks — they are lossless,
idempotent repairs. `fix --dry-run` is the check mode: it reports what
it *would* repair and exits 1 if anything is pending, writing nothing.

```console
$ animsmith fix examples/assets/clip-dirty.glb --dry-run
  would fix[quat-norm] clip 'swing' bone 'spine': 1 key(s) unit-normalized
1 key(s) would be fixed across 1 track(s) -> no output written
  would fix[quat-flip] clip 'swing' bone 'spine': 1 key(s) hemisphere-normalized
1 key(s) would be fixed across 1 track(s) -> no output written   # exits 1
```

Write the repaired asset with `-o` (or `--in-place`), then confirm it
lints clean:

```console
$ animsmith fix examples/assets/clip-dirty.glb -o fixed.glb
  fixed[quat-norm] clip 'swing' bone 'spine': 1 key(s) unit-normalized
1 key(s) fixed across 1 track(s) -> fixed.glb
  fixed[quat-flip] clip 'swing' bone 'spine': 1 key(s) hemisphere-normalized
1 key(s) fixed across 1 track(s) -> fixed.glb

$ animsmith lint fixed.glb
fixed.glb:
  coverage[bind-pose] insufficient_rotation_evidence ×1 (scopes: first_frame_rest_delta; ...
0 error(s), 0 warning(s), 0 note(s), 1 coverage gap(s), 0 available prediction facet(s), ...   # exits 0
```

Because the repairs are lossless, `diff` confirms no measurement moved —
the fix changed representation, not motion:

```console
$ animsmith diff examples/assets/clip-dirty.glb fixed.glb
no significant movement
0 significant change(s)                      # exits 0
```

Pin an exact repair set with `--repair id[,id]` (`animsmith fix --help`
lists the ids). Repairs that cannot be applied byte-surgically — data-URI
`.gltf` buffers, cubic tracks, quantized rotations — are reported as
`skipped[...]` and do not fail the check. Gate on `lint` when detection
alone should fail CI.

---

## 3. Editing a clip

Use editing when the animation content is right but the pipeline cut is
wrong: a capture has junk at the head, a one-shot needs a final hold, or
a loop's stride anchor lands in the wrong place. Those are the cases in
[the clip is the wrong length or freezes at the end](../docs/symptoms/wrong-length.md)
and [the loop pops](../docs/symptoms/loop-pops.md), where a
mechanical transform can make the clip conform without touching
geometry.

`transform` applies mechanical pipeline edits — slice a window, hold the
final pose, re-anchor a gait cycle, remove an eligible duplicate loop
endpoint, or prune provably constant multi-key tracks. Geometry passes through
unchanged.

Slice a sub-window (retimed to start at 0):

```console
$ animsmith transform examples/assets/clip.glb -o sliced.glb --slice 0.5:1.0
  sliced 'swing' to [0.5:1]s (3 keys max)
wrote sliced.glb (3 node(s), 1 clip(s), 0 mesh(es) / 0 position(s), 0 material(s))

$ animsmith diff examples/assets/clip.glb sliced.glb
  swing duration_s: moved 1.0000 -> 0.5000
  swing frame_count: moved 5.0000 -> 3.0000
  swing loop_continuity.bones[1].rotation_delta_deg: moved 22.9183 -> 11.4592
  swing loop_continuity.bones[2].position_delta_m: moved 0.1987 -> 0.0998
  swing loop_continuity.bones[2].rotation_delta_deg: moved 22.9183 -> 11.4592
  swing loop_continuity.bones[2].seam_velocity_delta_mps: moved 0.0598 -> 0.0200
  swing bone_rotation_range_deg[spine]: moved 22.9183 -> 11.4591
7 significant change(s)                       # exits 1
```

Extend the final pose (useful for hold frames at the end of a one-shot):

```console
$ animsmith transform examples/assets/clip.glb -o held.glb --hold-extend 0.5
  hold-extended 'swing' by 0.5s
wrote held.glb (3 node(s), 1 clip(s), 0 mesh(es) / 0 position(s), 0 material(s))
```

Other transforms: `--gait-anchor` rotates a cyclic clip so its stride
anchor lands at t=0. It is an explicit in-place declaration and needs
resolvable Root (or Hips fallback), Hips, and feet roles; accumulating root
translation/yaw refuses without writing the output or emitting buffered success
stdout. Every nonconstant selected trajectory channel must contain exactly one
key at each `--fps` whole-frame sample over the clip duration; sparse,
differently framed, duplicate-time, or off-grid evidence and duplicate `(bone,
property)` channels refuse. Declared frames × skeleton bones, declared frames ×
tracks, and maximum authored keys × skeleton bones are each capped at 1,000,000
samples. Retain root motion or use runtime phase offsets instead. `--fps
N` sets the grid used for retiming. See
[cli.md](../docs/cli.md#commands) for the full flag list.

For a loop exported with frame 0 copied again at the inclusive final frame,
use `--drop-duplicate-loop-endpoint` only after the default warning identifies
the strict authored-key case:

```console
$ animsmith transform cycle.glb -o cycle-open.glb --drop-duplicate-loop-endpoint
```

The transform removes the same closing key count from every channel and re-pins
the duration; it does not repair a stationary hold, root-travel/nonclosing
clip, mismatched timelines, endpoint tangents, or retargeting damage. The open
cycle is for engines that wrap over duration, so its inclusive
`loop-closure` result is expected to change.

For exported clips with redundant “key everything” or baked-control channels,
start with the default `constant-track` note, inspect the DCC/engine result,
then make the explicit mechanical edit:

```console
$ animsmith lint exported.glb --select constant-track
$ animsmith transform exported.glb -o compact.glb --prune-constant-tracks
  constant-track removed 'idle': track index 3 bone 'control' scale Linear 90 key(s)
```

The transform keeps a deterministic text record for every removal and every
candidate it declines, including the source track index. It leaves an
`animates_bones` target in place so the declared motion contract can still be
checked; it does not protect `[rig] required_bones`, which only declares that
the skeleton must contain a name. Re-lint and preview `compact.glb` in the
target engine before replacing the export.

---

## 4. A project contract config

Use a contract config when the failure depends on what your game expects,
not on file validity alone. [The character glides or runs in
place](../docs/symptoms/character-glides.md),
[feet skate when clips blend](../docs/symptoms/blend-skate.md),
and [feet slide within one
clip](../docs/symptoms/feet-slide.md) all need
declared locomotion or blend assumptions before animsmith can judge
them.

Mechanical checks run with no config. Contract-aware checks run only for the
expectations you declare. `duplicate-loop-endpoint`, `loop-closure`,
`loop-seam-vel`, and `loop-seam-rot` need only `loop = true`; role-dependent checks such as
`loop-seam`, `gait-group`, `root-motion-speed`, `in-place`, and `foot-slide`
also need resolvable rig roles. Without those roles they report a typed coverage
gap rather than guess, so a config that pins a `[rig] profile` (or inline
`[rig.roles]`) is what makes them fire.

`sync-group` is role-independent: use it for same-time or absolute-sync blend
sets, where compatible duration, longest-channel key count, validated declared
FPS grid, and loop endpoint mode matter. Each group declares non-negative
duration, key-count, and FPS spread tolerances; incompatible findings retain a
machine-readable row for every configured member.

The optional nested `time_complement` policy activates the separate
warning-level `time-complement` check for every unordered pair in that sync
group. It uses the already measured left-minus-right foot-height gait phase,
so it needs resolvable hips plus left and right foot/toe roles. The
exact-zero signal has no phase subject even when `min_lr_amplitude_m = 0`;
positive near-idle or noisy phase evidence below that floor is also excluded.
Among the remaining pairs, a warning appears only when reflected-time
similarity beats same-time similarity by more than
`min_reflected_time_advantage`. The finding's member rows retain both scores,
their advantage, each phase, and each amplitude.

`examples/assets/walk.glb` is a committed rig for this: a hips + two-foot
skeleton with a one-second walk cycle. Its bone names resolve a built-in
profile, so `inspect` binds the rig with no config at all:

```console
$ animsmith inspect examples/assets/walk.glb
examples/assets/walk.glb
rig profile: ue-mannequin (3 roles)
  hips         -> pelvis (exact)
  left_foot    -> foot_l (exact)
  right_foot   -> foot_r (exact)
skeleton: 3 bones
  pelvis
    foot_l
    foot_r
materials: 0
mesh instances: 0
clips: 1
  walk: 1.000s, 2 tracks, 33 keys max
```

`measure` reports the semantic metrics the checks judge — per-bone C0 pose
closure and C1 seam velocity, the feet-relative loop-seam ratio (≈ 0 here,
since this cycle returns its feet exactly), gait phase, L/R foot amplitude,
and sampled Hips-fallback trajectory. This fixture's pelvis stays fixed, so
all trajectory values are zero while the selected source remains explicit:

```console
$ animsmith measure --format json examples/assets/walk.glb
{
  "schema_version": 19,
  "schema": "urn:animsmith:schema:output:19",
  "tool": { "name": "animsmith", "version": "0.13.2",
            "source": { "revision": null, "dirty": null } },
  "command": "measure",
  "summary": { "files": 1 },
  "files": [
    {
      "path": "examples/assets/walk.glb",
      "input": {
        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "bytes": 123456
      },
      "rig": { "profile": "ue-mannequin", "resolution_outcome": "coverage",
        "resolved_roles": { "hips": "pelvis", "left_foot": "foot_l", "right_foot": "foot_r" },
        "resolved_role_policies": { "hips": "exact", "left_foot": "exact", "right_foot": "exact" } },
      "measurements": {
        "schema_version": 18,
        "schema": "urn:animsmith:schema:measurements:18",
        "clips": { "walk": {
          "duration_s": 1.0, "frame_count": 33,
          "animated_bones": ["foot_l", "foot_r"],
          "bone_channels": [
            { "bone_index": 1, "bone_name": "foot_l",
              "properties": ["translation"] },
            { "bone_index": 2, "bone_name": "foot_r",
              "properties": ["translation"] }
          ],
          "bone_rotation_range_deg": {},
          "loop_continuity": { "bones": [
            { "bone_index": 0, "bone_name": "pelvis",
              "availability": "measured",
              "position_delta_m": 0.0, "rotation_delta_deg": 0.0,
              "seam_velocity_delta_mps": 0.0,
              "seam_angular_velocity_delta_degps": 0.0 },
            { "bone_index": 1, "bone_name": "foot_l",
              "availability": "measured",
              "position_delta_m": 3.7e-17, "rotation_delta_deg": 0.0,
              "seam_velocity_delta_mps": 0.0,
              "seam_angular_velocity_delta_degps": 0.0 },
            { "bone_index": 2, "bone_name": "foot_r",
              "availability": "measured",
              "position_delta_m": 3.7e-17, "rotation_delta_deg": 0.0,
              "seam_velocity_delta_mps": 0.0,
              "seam_angular_velocity_delta_degps": 0.0 }
          ] },
          "loop_continuity_availability": "measured",
          "loop_endpoint_mode_availability": "not_applicable",
          "frame_grid_availability": "not_applicable",
          "loop_seam_ratio": 1.2e-15,
          "loop_seam_ratio_availability": "measured",
          "gait": { "phase": 0.75, "phase_availability": "measured", "lr_amplitude_m": 0.2 },
          "gait_availability": "measured",
          "root_trajectory": {
            "bone_index": 0, "bone_name": "pelvis",
            "source_role": "hips_fallback",
            "translation": {
              "horizontal_displacement_x_m": 0.0,
              "horizontal_displacement_z_m": 0.0,
              "horizontal_travel_m": 0.0,
              "vertical_displacement_m": 0.0,
              "vertical_min_displacement_m": 0.0,
              "vertical_max_displacement_m": 0.0
            },
            "translation_availability": "measured",
            "yaw": {
              "heading_axis": "positive_z", "net_yaw_deg": 0.0,
              "unwrapped_yaw_deg": 0.0, "yaw_travel_deg": 0.0
            },
            "yaw_availability": "measured"
          },
          "root_trajectory_availability": "measured",
          "speed_mps": 0.0,
          "speed_mps_availability": "measured"
        } },
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
    }
  ]
}
```

[`examples/walk.animsmith.toml`](walk.animsmith.toml) is the
contract: it declares the clip a loop (which arms the loop checks) and
in-place, and caps their tolerances. Against the clean rig every semantic check
passes, and the run exits 0. The one warning is not a defect in the motion: a
cycle whose last key repeats its first is a representation an engine can trim,
which [editing a clip](#3-editing-a-clip) covers with
`transform --drop-duplicate-loop-endpoint`.

```console
$ animsmith lint --config examples/walk.animsmith.toml examples/assets/walk.glb
examples/assets/walk.glb:
  warning[duplicate-loop-endpoint] clip 'walk' @1.000s: declared loop repeats its first pose ...
  coverage[bind-pose] insufficient_rotation_evidence ×1 (scopes: first_frame_rest_delta; ...
0 error(s), 1 warning(s), 0 note(s), 1 coverage gap(s), 0 available prediction facet(s), ...   # exits 0
```

`examples/assets/walk-dirty.glb` is the same rig with the clip cut a
quarter-cycle short, so the feet never return to their first-frame pose —
the classic popped loop seam. The same contract catches it:

```console
$ animsmith lint --config examples/walk.animsmith.toml examples/assets/walk-dirty.glb
examples/assets/walk-dirty.glb:
  error[loop-closure] clip 'walk' bone 'foot_r' @1.000s: loop does not close in position: ...
  error[loop-seam] clip 'walk' @1.000s: loop seam pops: wrap discontinuity is 6.82× ...
  error[loop-seam-vel] clip 'walk' bone 'foot_r' @1.000s: loop velocity changes at the seam: ...
  coverage[bind-pose] insufficient_rotation_evidence ×1 (scopes: first_frame_rest_delta; ...
3 error(s), 0 warning(s), 0 note(s), 1 coverage gap(s), 0 available prediction facet(s), ...   # exits 1
```

The contract is load-bearing: a bare `animsmith lint examples/assets/walk-dirty.glb`
(no config) reports no findings — with no `loop = true` declared, the three
loop checks are explicitly not applicable. Semantic checks
enforce *your* declared expectations, not a guess.

`walk-dirty.glb` is one of a set. `examples/assets/` commits one mutation
of this same walk per runtime symptom — a channel that ends early, a clip
that travels, a blend ring whose members disagree, a frozen arm, an
animated non-uniform scale — each alongside the contract config that arms
its checks. The [asset table](assets/README.md) lists what every fixture
fires and which config to pass.

### Keeping the exported rig shape stable

`animates_bones` is deliberately per clip: it says that a named bone must
exist **and carry animation keys** in that take. That is the right contract
for a swinging arm or a locomotion root, but it is too strict for a static
weapon socket, an IK target, a retargeting mask bone, or another runtime
attachment point. Put those structural requirements on the rig instead:

```toml
[rig]
profile = "auto"
required_bones = ["root", "weapon_socket", "ik_hand_l"]
```

`required-bones` checks only that each declared name identifies exactly one
bone in the input skeleton. A present-but-static bone is clean; a missing name
is an error. Duplicate skeleton names are ambiguous rather than silently
matched, and an export with no skeleton cannot satisfy the contract. This is
also useful for a static mesh or a take with no clips: the structural check
does not need animation data to run.

It is not a repair tool. AnimSmith does not add sockets, rename a duplicate,
retarget a rig, or decide which bones your engine needs; fix the source rig or
its export in the DCC, then lint it again. Use `[clips.<name>]
animates_bones = [...]` when the same bones must move in a particular clip,
and `frozen-bone` when keyed rotation is required to exceed a meaningful
motion floor.

Presence and transform policy are separate. To require a runtime-facing node
to inherit a unit effective rest scale, select it explicitly:

```toml
[runtime_nodes]
selectors = ["weapon_socket", "ik_*_target"]

[checks.rest-world-scale]
expected_uniform_scale = 1.0
uniform_scale_tolerance = 0.0001
```

An exact name or `*` glob must match exactly one source node. A miss or
multiple matches produces explicit coverage instead of guessing. The finding's
`node` path includes source indices and ancestors, so a local scale of `1` can
still be traced to a non-unit effective scale inherited from a parent. This
does not inspect animation scale keys, infer units from geometry, or rescale
the file. The legacy `[checks.rest-world-scale].node_selectors` spelling is an
alias; declaring it together with `[runtime_nodes]` is a configuration error.

### Scaling up to a full character

[`examples/character.animsmith.toml`](character.animsmith.toml)
is the full game-character shape the small walk contract grows into:

```toml
[rig]
profile = "auto"            # or mixamo / ue-mannequin / humanoid

[checks.loop-seam]
max_ratio = 1.6             # per-check tuning
[checks.loop-closure]
max_position_delta_m = 0.01
max_rotation_delta_deg = 1.0
[checks.loop-seam-vel]
max_velocity_delta_mps = 0.1
[checks.loop-seam-rot]
max_angular_velocity_delta_degps = 5.0
[runtime_nodes]
selectors = ["weapon_socket", "ik_*_target"]
[checks.rest-world-scale]
expected_uniform_scale = 1.0
uniform_scale_tolerance = 0.0001
[checks.frozen-bone]
min_rotation_deg = 0.5
[checks.quat-flip]
severity = "note"           # demote while an upstream fix lands

[clips."run_*"]             # glob: every run clip loops
loop = true
movement_owner_xz = "animation" # extracted root motion owns horizontal travel
movement_owner_y = "gameplay"   # controller owns vertical motion
movement_owner_yaw = "animation"
max_loop_position_delta_m = 0.04  # per-family pose/velocity seam caps
max_loop_rotation_delta_deg = 2.0
max_loop_velocity_delta_mps = 0.2
max_loop_angular_velocity_delta_degps = 200.0
[clips.run_forward]
max_loop_rotation_delta_deg = 0.5 # exact entry wins for this dimension
duration_s = { value = 1.033, tolerance = 0.02 } # authored clip-length contract
speed_mps = { value = 3.1, tolerance = 0.25 }   # root-motion contract

[gait_groups.run-ring]      # a directional blend ring
clips = ["run_forward", "run_backward", "run_left", "run_right"]
max_gait_phase_spread = 0.15   # members must stride in phase
min_lr_amplitude_m = 0.03      # exclude near-idle members

[sync_groups.run-ring]
clips = ["run_forward", "run_backward", "run_left", "run_right"]
max_duration_delta_s = 0.001
max_frame_count_delta = 0
max_fps_delta = 0.01

[sync_groups.run-ring.time_complement]
min_reflected_time_advantage = 0.25 # reflected score must beat same-time score
min_lr_amplitude_m = 0.03           # skip low-confidence gait signals
```

A `gait_groups` block is the payoff for a real character: it holds every
clip in a directional blend ring to the same stride phase, so runtime
blends between them don't skate. `animsmith.toml` is auto-loaded from the
working directory, so committing one next to your assets makes every bare
`animsmith lint` enforce the contract.

Movement ownership is independent on XZ, Y, and yaw. Each field is either
`"gameplay"` (the entity/controller owns that component) or `"animation"`
(extracted root motion owns it); omitted axes stay absent. `in_place = true`
and `false` remain compatibility aliases for XZ gameplay and animation,
respectively, but one selector entry cannot use both `in_place` and
`movement_owner_xz`. Exact/glob layers normalize the alias before applying the
usual field-by-field precedence.

The four role-free loop-continuity caps (`max_loop_position_delta_m`,
`max_loop_rotation_delta_deg`, `max_loop_velocity_delta_mps`, and
`max_loop_angular_velocity_delta_degps`) refine the
corresponding global check caps in a `[clips.<exact-name>]` entry or
`[clips."glob_*"]` family. An exact name wins over matching globs and an
omitted field keeps its inherited cap. This is useful when root-motion runs
need a different position allowance but idles must stay tightly closed. All
four values must be finite and non-negative.

### Predicting a Bevy animation selector

The small [`bevy.animsmith.toml`](bevy.animsmith.toml) profile selects Bevy
0.19.0's exact glTF loader contract. Its first production prediction maps each
completely inventoried source animation index to the canonical typed subasset
label spelling `Animation{index}`:

```console
$ animsmith --config examples/bevy.animsmith.toml lint \
    --select engine-addressability examples/assets/walk.glb
```

The available facet's subject is the predicted display label, such as
`Animation0`. Names do not control this selector: unnamed animations work, and
duplicate names remain distinct indices. This predicts the selector spelling,
not that Bevy loaded the file, retained its targets, or wired it into an
animation graph. Incomplete source-animation inventory produces a blocking
`required_prediction_unavailable` facet instead of guessing from the retained
prefix. Prediction provenance v1 separately caps actual clip settings at
4,096, so a 4,097-clip file is a bounded operator error rather than a truncated
prediction; issue #485 owns a future overflow representation.

Revision 2 uses [`bevy-v2.animsmith.toml`](bevy-v2.animsmith.toml) for the
unit/effective-scale rule. The example selects the closed empty-handler
environment, declares the compiled animation feature, and demonstrates shared
runtime-node selectors:

```console
$ animsmith --config examples/bevy-v2.animsmith.toml lint \
    --select engine-unit-scale examples/assets/walk.glb --format json
```

The available file result is an exact 1:1 glTF-metre to Bevy world-length-unit
mapping, not a claim that arbitrary application world state uses metres. Scene,
primitive-child, and selected-node facets remain distinct; incomplete evidence
is blocking rather than inferred.

Revision 3 is the current example in
[`bevy-v3.animsmith.toml`](bevy-v3.animsmith.toml). It selects the narrow
`engine-track-support` gate-support prediction:

```console
$ animsmith --config examples/bevy-v3.animsmith.toml lint \
    --select engine-track-support examples/assets/walk.glb --format json
```

The prediction binds raw animation rows and independent per-animation channel
coverage from the same load. The compiled `bevy_animation` feature gate takes
precedence over `load_animations`; a disabled gate yields only negative dropped
animation/channel outcomes, never a content finding. If both gates allow
loading, runtime survival is required-unavailable because this contract does
not run Bevy. Complete-empty inventory is not applicable; partial or
unavailable inventory yields one subjectless, unsuppressible inventory facet
and no retained-prefix prediction, including bounded N+1 overflow. Extensions,
other animation constructs, and positive runtime survival are outside this
slice. V5 provenance remains immutable inside the current output-v19 envelope;
output-v17, output-v16, the rev1/rev2 profiles, and output-v15 contracts remain
preserved.

Generate a reusable engine-neutral inventory as canonical JSON without a
profile:

```console
$ animsmith generate addressability examples/assets/walk.glb \
    > walk.addressability.json
```

Select the exact profile to add the unchanged Bevy evaluation to the same
document; the neutral inventory and its identity stay unchanged:

```console
$ animsmith --config examples/bevy.animsmith.toml generate addressability \
    examples/assets/walk.glb --format markdown
```

The inventory keeps source animation and channel indices, optional source
names, raw targets and accessors, and dependency-closure identity. It does not
infer scenes, skins, named-map winners, target UUIDs, extension support, or
successful runtime loading.

The richer exact-Bevy path is a separate V2 contract,
`urn:animsmith:schema:gltf-addressability:2`; see
[`gltf-addressability-v2.schema.json`](../docs/schemas/gltf-addressability-v2.schema.json).
It retains bounded same-load scene, node, skin, attachment, root-to-node path,
default-scene, named-map, and unique-target evidence while preserving this V1
animation inventory. Complete empty coverage proves absence; partial prefixes
and unavailable domains do not. Explicit `skin.skeleton` is retained as
source evidence, not inferred into a Bevy root, and scene-instantiated
`SkinnedMesh` attachment is not claimed.

The V2 adapter is pinned to Bevy 0.19.0 revision 3 and commit
`c6f634ca9f406d68ba5109d921247b654cb42c10` (`bevy_gltf 0.19.0`, locked
`gltf 1.4.1`, and root `Cargo.lock`). It reuses one `engine-addressability` lifecycle and the
existing `Animation{i}` primitive. `Scene{i}` is emitted for every declared
scene, and `Gltf.default_scene` routes only to an existing scene; there is no
`DefaultScene` label or fabricated `Scene0`. Bevy eagerly emits
`Skin{i}/InverseBindMatrices` for every source skin, including unreferenced
skins, and emits `Skin{i}` when any source node references it. Named maps are
separate source-order last-write-wins projections.

Exact target UUIDs require explicitly selecting 32- or 64-bit target pointer
width because Bevy hashes segment lengths using the target pointer width;
AnimSmith never infers width from the host. Missing feature/settings,
incomplete or unreachable targets, multiple paths, and path/UUID collisions are typed
`required_unavailable`, not guessed. `target_coverage` distinguishes complete
(including empty) target coverage from incomplete raw/animation evidence and
`target_domain_truncated`; other rich projection budget failures use
`projection_bounds_exceeded`. Each V2 domain is capped at 4,096 rows,
references at 65,536, path segments at 1,024 bytes and 256 segments, retained
text at 1 MiB, and reports at 256 MiB. This remains prediction evidence only,
not runtime-load, spawning, target-survival, graph, or playback evidence.

### Predicting Unity Generic root motion

Select an exact engine profile and fully materialize every required importer
setting before generating advice. For Unity Generic, for example:

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

[clips."*"]
loop = true
movement_owner_xz = "animation"

[clips."*".engine_settings]
root_rotation = "extract"
root_position_y = "bake"
root_position_xz = "extract"
```

```console
$ animsmith --config unity.animsmith.toml lint \
    --select engine-root-motion export.fbx
```

The current Generic revision-2 lint prediction compares the three declared
movement-owner axes with the three `bake | extract` clip settings. `bake`
means `baked_into_pose`; `extract` means `stored_as_root_motion`. It requires a
case-sensitive, byte-exact `Reference/Root` path to the explicitly resolved
Root role and never uses Hips as a fallback. The path grammar has no escaping
or Unicode normalization, rejects empty/reserved/control/format segments, and
limits segments, paths, and depth to 1,024 bytes, 4,096 bytes, and 256
segments. Raw path evidence is projected from the same FBX bytes; the implicit
root and generated helpers cannot match.

An available facet records a typed routing result; a conflict is an error
finding. Missing path, Root, intent, settings, raw inventory, or measured
translation/yaw availability is `required_prediction_unavailable`, not a
content finding, and exits 1. Measurement magnitude does not affect routing:
zero and nonzero travel use the same comparison. This is prediction evidence
only—no Unity editor execution, imported-asset readback, runtime playback, or
engine certification occurs. The contract uses prediction provenance V6 and
output V19; historical output V9 and other readers remain immutable. The
checked-in/CI FBX inputs for this slice are self-authored synthetic fixtures.

### Generating engine import advice

The separate historical revision-1 advice profile remains the route for
projecting `Convert Units`, `Bake Axis Conversion`, Generic `Root Motion
Source`, and the three `ModelImporterClipAnimation.lockRoot*` booleans. Use a
revision-1 Unity advice config for this command:

```console
$ animsmith --config unity-advice.animsmith.toml generate import-advice export.fbx \
    > export.import-advice.json
```

Unreal 5.8 and Godot 4.7 revision 1 deliberately exit 1 with
`profile_settings_unmodeled`; their frozen profiles do not yet define importer
settings. That command does not guess frame ranges, sample rates, unit
conversion, or root-motion behavior.

Revision 2 provides the narrow document-setting projection for the exact
Godot and Unreal tuples:

```toml
# Godot 4.7 / revision 2 / resource-importer-scene (glTF JSON or GLB)
[engine]
profile = "godot"
profile_revision = 2
engine_version = "4.7"
importer = "resource-importer-scene"

[engine.settings]
animation_fps = 30
animation_trimming = false
```

Godot's only projected keys are `animation/fps` (1..120, default 30) and
`animation/trimming` (default false). Unreal 5.8 / revision 2 / `fbx-importer`
accepts FBX and requires one explicit document setting:

```toml
[engine]
profile = "unreal"
profile_revision = 2
engine_version = "5.8"
importer = "fbx-importer"

[engine.settings]
sample_rate = "default_30" # or "source_determined" / "custom_hz(60)"
```

The V2 JSON contract is
`urn:animsmith:schema:engine-import-advice:2`. It contains one available
native import-setting projection or one typed refusal; it does not claim importer
execution, frame ranges, units, skeleton/retargeting, compression, root
motion, runtime behavior, or project-file changes.

### Steering a run without a config

You can also shape a run from the command line. `--select` restricts the
run set, `--allow` suppresses findings, and `[checks.<id>] severity`
(including `"off"`) reshapes how hard each check fails:

```console
$ animsmith lint --select quat-norm examples/assets/clip-dirty.glb   # only quat-norm
$ animsmith lint --allow quat-flip examples/assets/clip-dirty.glb    # hide quat-flip
```

Demote a check while an upstream fix is pending (a `[checks.quat-flip]`
`severity = "note"` override turns the warning into a note):

```console
$ cat demote.toml
[checks.quat-flip]
severity = "note"

$ animsmith lint --config demote.toml examples/assets/clip-dirty.glb
  error[quat-norm] clip 'swing' bone 'spine' @0.500s: non-unit rotation key ...
  note[quat-flip] clip 'swing' bone 'spine' @0.750s: 2 hemisphere flip(s) ...
1 error(s), 0 warning(s), 1 note(s), 1 coverage gap(s), 0 available prediction facet(s), ...   # exits 1
```

See the [configuration reference](../docs/configuration-reference.md)
for the full key reference.

---

## 5. Converting exports and generating reports _(default features only)_

Use this workflow when a DCC or marketplace export reaches your importer
but brings bloated constant tracks, stray scale keys, or a rig the
retargeter cannot trust. That is the hygiene side of [the file is
bloated, or the retargeter
chokes](../docs/symptoms/file-bloat.md)
and the rig-contract side of [a limb is T-posed, or a bone never
moves](../docs/symptoms/limb-frozen.md):
convert to glTF, measure/lint/report, then compare against the previous
asset before committing the migration.

`convert` normalizes an FBX (or glTF) export into a clean glTF, and
`report` renders a self-contained HTML report with skeleton playback and
metric charts. Both are in the default build; a `--no-default-features`
binary omits them. `report --evidence-only` writes the same findings,
coverage gaps and engine predictions without the sampled pose grid — keeping
the charts in a single-clip report, though not in a comparison, whose panels
are drawn from that grid — for sharing evidence where the clip itself cannot
travel.

```console
$ animsmith convert export.fbx -o clip.glb
$ animsmith measure clip.glb
$ animsmith report clip.glb -o report.html
$ animsmith diff old.glb clip.glb
```

The repository also carries a byte-checked, self-authored visual-comparison
pair. It exercises the report's acceptance context without a commercial or
private source asset:

```console
$ animsmith --config examples/report-comparison.animsmith.toml report \
    examples/assets/report-comparison-before.glb \
    --compare-after examples/assets/report-comparison-after.glb \
    --before-clip acceptance-matrix --after-clip acceptance-matrix \
    -o comparison.html
```

In `comparison.html`, select the before-side loop finding to see the exact
first and last judged poses overlaid with the affected bone highlighted.
The trajectory panels disclose root, hips, and bilateral-foot trails; the
gait panel shades the typed `foot-slide` stance intervals and names the
selected role. The combined root chart uses one metres scale and distinct
before/after legends. The before-side `constant-track` row is explicitly
structural evidence: its quaternion keys can be removed while the sampled
poses look unchanged. Neither the fixture nor the report certifies artistic,
gameplay, importer, or engine acceptance.

`cargo run -p animsmith --example gen_example_assets` regenerates both inputs.
The `example_assets_match_generator_output` integration test rebuilds every
committed example and compares its bytes, so CI catches hand edits or generator
drift.

When one authoritative skinned base and its animation takes live in separate
files, use the versioned [`character-assembly.toml`](character-assembly.toml)
recipe instead. `assemble` produces one GLB and evidence pair; source-package
extraction and project publication stay outside animsmith.

The example uses current recipe v7 without its optional rest/bind operation.
For glTF/GLB or inventory-complete normalized/baked FBX assembly, add
`[rest_bind_scale]` with the required exact `root_node_name` and
`expected_factor` fields. The base must resolve that name to one source node
and exactly one non-empty skin whose every joint is that node or a descendant.
Every distinct FBX clip is projected to its normalized skeleton and takes. A
glTF/GLB clip retains its existing successful full rest/bind or meshless
track-only path; its clip-track gate additionally admits only failures in
unused geometry, deformation, material, or bind domains while retaining
framing, dependency, raw-coverage, named-skeleton, and animation
accessor/layout safety. The basis proof covers the selected base skin joints, exact
root, their named ancestry, and actual track targets; unreferenced base-only
geometry descendants need not appear in the clip. Every input passes basis
compatibility before remapping.

```console
$ animsmith assemble examples/character-assembly.toml \
    -o character.glb --evidence character.assembly.json
```

### Getting a test asset

We do not ship third-party assets. To try this on a real rig:

- **Mixamo** — free with an Adobe ID and royalty-free for personal and
  commercial use. Download a character + animation as FBX, then
  `convert` it. Mixamo is also a built-in rig profile, so
  `[rig] profile = "mixamo"` resolves its roles for the semantic checks.
  Check Adobe's current terms before redistributing any downloaded
  asset; the safe path is to keep them out of your repo. The
  [Mixamo tutorial](../docs/mixamo-tutorial.md) takes one download
  end-to-end, from this convert step through a contract config.
- **CC0 / public-domain sources** for assets you want to commit.
- Or **generate your own** — see the
  [asset generator](../crates/animsmith/examples/gen_example_assets.rs)
  this repo uses for its own fixtures.

---

## 6. Embedding animsmith as a library gate

Use embedding when the team already has an importer or build pipeline and
wants animsmith's checks as a library step instead of a separate shell
command. The pipeline-library use case is the payoff in
[why animsmith](../docs/why-animsmith.md): load your
asset once, map your own contract into `Config`, and surface findings in
the same gate that owns the rest of your asset rules.

Pipelines can skip the CLI and drive the check catalog directly: load a
document, resolve rig roles, build a `Config` from your own contract
format, measure, run the checks, and map findings to your gate.

The runnable walkthrough is
[`crates/animsmith/examples/embed.rs`](../crates/animsmith/examples/embed.rs),
paired with [embedding.md](../docs/embedding.md):

```console
$ cargo run -p animsmith --example embed
```

It exits 1 on purpose — the example declares a deliberately wrong
expectation to demonstrate a failing gate, not an accidental error.

---

## Feature matrix

| Example | Needs |
|---|---|
| 1 first gate, 2 repair, 3 transform, 4 config | any build (incl. `--no-default-features`) |
| 5 convert / report | default features (`fbx`, `report`) |
| 6 embedding | library crates |

## Asset policy

Assets committed to this repo are procedurally generated or CC0, with
provenance recorded. Third-party assets (Mixamo and similar) are used
via documented download steps, not checked-in bytes or download scripts,
unless their terms clearly permit redistribution.
