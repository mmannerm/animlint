use animsmith_core::InputIdentity;
use animsmith_testkit::closed_stream::ClosedStream;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

fn animsmith() -> Command {
    Command::new(env!("CARGO_BIN_EXE_animsmith"))
}

struct FootCycleFixture {
    _directory: TempDir,
    root: PathBuf,
    manifest: PathBuf,
    parameterization: PathBuf,
    destination: PathBuf,
}

impl FootCycleFixture {
    fn create() -> Self {
        Self::create_with_cyclic_contacts(false)
    }

    fn create_with_cyclic_contacts(cyclic_contacts: bool) -> Self {
        Self::create_sampled(cyclic_contacts, SIXTEENTHS)
    }

    /// The same two-member locomotion set sampled at 30 fps.
    ///
    /// The stance windows stay at their normalized positions, so every stance
    /// boundary still lands on an authored frame, but the authored times are
    /// no longer exact binary32 fractions of the clip duration.
    fn create_at_thirty_fps(frames: usize) -> Self {
        Self::create_at(frames, 30.0)
    }

    fn create_at(frames: usize, rate: f32) -> Self {
        Self::create_sampled(false, Sampling { frames, rate })
    }

    fn create_sampled(cyclic_contacts: bool, sampling: Sampling) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        fs::create_dir(root.join("assets")).unwrap();
        fs::create_dir(root.join("contacts")).unwrap();
        fs::create_dir(root.join("generated")).unwrap();
        if cyclic_contacts {
            write_source(
                &root,
                "a",
                sampling,
                |index| index <= 4 || index >= 12,
                |index| (3..=13).contains(&index),
            );
            write_source(
                &root,
                "b",
                sampling,
                |index| index <= 5 || index >= 13,
                |index| (4..=14).contains(&index),
            );
        } else {
            write_stance_source(&root, "a", sampling, 0.0);
            write_stance_source(&root, "b", sampling, MEMBER_B_PHASE_OFFSET);
        }
        fs::write(
            root.join("config.toml"),
            "[rig]\nprofile = \"auto\"\nroles = { root = \"R0\", hips = \"H0\", left_foot = \"LF0\", right_foot = \"RF0\" }\n\n[clips.\"Take 001\"]\nloop = true\n",
        )
        .unwrap();

        let a = fs::read(root.join("assets/a.gltf")).unwrap();
        let b = fs::read(root.join("assets/b.gltf")).unwrap();
        let manifest = root.join("collection.toml");
        fs::write(
            &manifest,
            format!(
                r#"schema = "urn:animsmith:schema:collection-manifest:1"
schema_version = 1
collection_id = "com.example"
input_root = "assets"

[[sources]]
key = "a"
path = "a.gltf"
config = "config.toml"
expected_sha256 = "{}"

[[sources]]
key = "b"
path = "b.gltf"
config = "config.toml"
expected_sha256 = "{}"

[[clips]]
id = "com.example/a"
source = "a"
take_index = 0
take_name = "Take 001"

[[clips]]
id = "com.example/b"
source = "b"
take_index = 0
take_name = "Take 001"

[[runtime_sets]]
id = "com.example/sets/walk"
kind = "gait-group"
members = ["com.example/a", "com.example/b"]
"#,
                InputIdentity::from_bytes(&a).sha256(),
                InputIdentity::from_bytes(&b).sha256(),
            ),
        )
        .unwrap();

        for member in ["a", "b"] {
            let output = root.join(format!("contacts/{member}.json"));
            let result = animsmith()
                .args([
                    "collection",
                    "generate-contact-fragment",
                    manifest.to_str().unwrap(),
                    "--clip",
                    &format!("com.example/{member}"),
                    "--output",
                    output.to_str().unwrap(),
                    "--format",
                    "json",
                ])
                .output()
                .unwrap();
            assert_success(&result);
            assert_eq!(result.stdout, fs::read(output).unwrap());
        }

        let manifest_input = InputIdentity::from_bytes(&fs::read(&manifest).unwrap());
        let parameterization = root.join("foot-cycle.toml");
        fs::write(
            &parameterization,
            format!(
                r#"schema = "urn:animsmith:schema:foot-cycle-parameterization:1"
schema_version = 1
runtime_set_id = "com.example/sets/walk"
reference_member = "com.example/a"
output_directory = "generated/aligned"
minimum_segment_slope = 0.25
maximum_segment_slope = 4.0

[proof]
max_gait_phase_spread = 0.08
min_lr_amplitude_m = 0.05
max_contact_boundary_phase_error = 0.01

[manifest]
schema = "urn:animsmith:schema:collection-manifest:1"
schema_version = 1
collection_id = "com.example"

[manifest.input]
sha256 = "{}"
bytes = {}

[[members]]
id = "com.example/a"
contact_fragment = "contacts/a.json"

[[members]]
id = "com.example/b"
contact_fragment = "contacts/b.json"
"#,
                manifest_input.sha256(),
                manifest_input.bytes(),
            ),
        )
        .unwrap();
        let destination = root.join("generated/aligned");
        Self {
            _directory: directory,
            root,
            manifest,
            parameterization,
            destination,
        }
    }

    fn run(&self) -> Output {
        transform_command(&self.manifest, &self.parameterization)
            .output()
            .unwrap()
    }
}

fn transform_command(manifest: &Path, parameterization: &Path) -> Command {
    let mut command = animsmith();
    command.args([
        "collection",
        "transform-foot-cycle",
        manifest.to_str().unwrap(),
        "--parameterization",
        parameterization.to_str().unwrap(),
        "--format",
        "json",
    ]);
    command
}

fn assert_success(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

/// Authored sampling of one synthetic locomotion source: `frames` keys at
/// `index as f32 / rate`, so the clip lasts `(frames - 1) / rate` seconds.
#[derive(Clone, Copy)]
struct Sampling {
    frames: usize,
    rate: f32,
}

/// 17 keys at 16 fps: every authored time is an exact binary32 sixteenth of
/// the one-second duration, so a control point at a stance boundary is the
/// exact authored instant in every arithmetic domain.
const SIXTEENTHS: Sampling = Sampling {
    frames: 17,
    rate: 16.0,
};

/// Normalized stance windows of the reference member, as `[start, end]`.
const LEFT_STANCE: [f64; 2] = [0.10, 0.28];
/// Normalized right-foot stance window of the reference member.
const RIGHT_STANCE: [f64; 2] = [0.60, 0.78];
/// Phase by which member `b` runs the same gait later than member `a`, which
/// is what makes `b`'s planned time warp non-identity.
const MEMBER_B_PHASE_OFFSET: f64 = 0.12;

/// Write one member whose stances occupy the shared normalized windows shifted
/// later by `phase_offset`.
fn write_stance_source(root: &Path, stem: &str, sampling: Sampling, phase_offset: f64) {
    let stance = |window: [f64; 2]| {
        move |index: usize| {
            let phase = index as f64 / (sampling.frames - 1) as f64 - phase_offset;
            (window[0]..=window[1]).contains(&phase)
        }
    };
    write_source(
        root,
        stem,
        sampling,
        stance(LEFT_STANCE),
        stance(RIGHT_STANCE),
    );
}

fn write_source(
    root: &Path,
    stem: &str,
    sampling: Sampling,
    left_contact: impl Fn(usize) -> bool,
    right_contact: impl Fn(usize) -> bool,
) {
    let Sampling { frames, rate } = sampling;
    let mut bytes = Vec::new();
    for index in 0..frames {
        bytes.extend_from_slice(&(index as f32 / rate).to_le_bytes());
    }
    let root_translation_offset = bytes.len();
    for _ in 0..frames {
        for value in [0.0_f32; 3] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let root_rotation_offset = bytes.len();
    for _ in 0..frames {
        for value in [0.0_f32, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let left_offset = bytes.len();
    for index in 0..frames {
        let y = if left_contact(index) {
            0.0_f32
        } else {
            0.1_f32
        };
        for value in [0.0_f32, y, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let right_offset = bytes.len();
    for index in 0..frames {
        let y = if right_contact(index) {
            0.0_f32
        } else {
            0.1_f32
        };
        for value in [0.0_f32, y, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let buffer_name = format!("{stem}.bin");
    fs::write(root.join("assets").join(&buffer_name), &bytes).unwrap();
    let source = json!({
        "asset": {"version": "2.0"},
        "buffers": [{"uri": buffer_name, "byteLength": bytes.len()}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": root_translation_offset},
            {"buffer": 0, "byteOffset": root_translation_offset, "byteLength": root_rotation_offset - root_translation_offset},
            {"buffer": 0, "byteOffset": root_rotation_offset, "byteLength": left_offset - root_rotation_offset},
            {"buffer": 0, "byteOffset": left_offset, "byteLength": right_offset - left_offset},
            {"buffer": 0, "byteOffset": right_offset, "byteLength": bytes.len() - right_offset}
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": frames, "type": "SCALAR", "min": [0.0], "max": [(frames - 1) as f32 / rate]},
            {"bufferView": 1, "componentType": 5126, "count": frames, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5126, "count": frames, "type": "VEC4"},
            {"bufferView": 3, "componentType": 5126, "count": frames, "type": "VEC3"},
            {"bufferView": 4, "componentType": 5126, "count": frames, "type": "VEC3"}
        ],
        "nodes": [
            {"name": "R0", "children": [1]},
            {"name": "H0", "children": [2, 3]},
            {"name": "LF0"},
            {"name": "RF0"}
        ],
        "animations": [{
            "name": "Take 001",
            "samplers": [
                {"input": 0, "output": 1, "interpolation": "LINEAR"},
                {"input": 0, "output": 2, "interpolation": "LINEAR"},
                {"input": 0, "output": 3, "interpolation": "LINEAR"},
                {"input": 0, "output": 4, "interpolation": "LINEAR"}
            ],
            "channels": [
                {"sampler": 0, "target": {"node": 0, "path": "translation"}},
                {"sampler": 1, "target": {"node": 0, "path": "rotation"}},
                {"sampler": 2, "target": {"node": 2, "path": "translation"}},
                {"sampler": 3, "target": {"node": 3, "path": "translation"}}
            ]
        }],
        "scenes": [{"nodes": [0]}],
        "scene": 0
    });
    fs::write(
        root.join("assets").join(format!("{stem}.gltf")),
        serde_json::to_vec(&source).unwrap(),
    )
    .unwrap();
}

fn count_files(root: &Path) -> usize {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| if path.is_dir() { count_files(&path) } else { 1 })
        .sum()
}

fn rewrite_canonical_json(path: &Path, mutate: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    mutate(&mut value);
    fs::write(path, serde_jcs::to_vec(&value).unwrap()).unwrap();
}

fn assert_identity_matches_bytes(identity: &Value, bytes: &[u8]) {
    let actual = InputIdentity::from_bytes(bytes);
    assert_eq!(identity["sha256"], actual.sha256());
    assert_eq!(identity["bytes"], actual.bytes());
}

#[cfg(not(feature = "fbx"))]
fn rebind_parameterization(fixture: &FootCycleFixture) {
    let input = InputIdentity::from_bytes(&fs::read(&fixture.manifest).unwrap());
    let text = fs::read_to_string(&fixture.parameterization).unwrap();
    let start = text.find("[manifest.input]").unwrap();
    let member_start = text[start..].find("\n[[members]]").unwrap() + start;
    let replacement = format!(
        "[manifest.input]\nsha256 = \"{}\"\nbytes = {}\n",
        input.sha256(),
        input.bytes(),
    );
    let mut rebound = String::with_capacity(text.len());
    rebound.push_str(&text[..start]);
    rebound.push_str(&replacement);
    rebound.push_str(&text[member_start + 1..]);
    fs::write(&fixture.parameterization, rebound).unwrap();
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

#[test]
fn public_command_publishes_exact_two_member_generation_and_stdout() {
    let fixture = FootCycleFixture::create();
    let result = fixture.run();
    assert_success(&result);
    let aggregate_path = fixture.destination.join("aggregate-evidence.json");
    assert_eq!(result.stdout, fs::read(&aggregate_path).unwrap());
    assert_eq!(count_files(&fixture.destination), 7);
    let expected = [
        "members/000000/artifact.glb",
        "members/000000/contact-fragment.json",
        "members/000000/evidence.json",
        "members/000001/artifact.glb",
        "members/000001/contact-fragment.json",
        "members/000001/evidence.json",
        "aggregate-evidence.json",
    ];
    for path in expected {
        assert!(fixture.destination.join(path).is_file(), "missing {path}");
    }
    let aggregate: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(aggregate["resources"]["members"], 2);
    assert_eq!(aggregate["resources"]["files"], 7);
    assert_eq!(aggregate["members"][0]["member_id"], "com.example/a");
    assert_eq!(aggregate["members"][1]["member_id"], "com.example/b");
    assert_eq!(
        aggregate["members"][0]["artifact_path"],
        "members/000000/artifact.glb"
    );
    for index in 0..2 {
        let member_root = fixture.destination.join(format!("members/{index:06}"));
        let artifact = fs::read(member_root.join("artifact.glb")).unwrap();
        assert!(animsmith_gltf::load_source_bytes(Path::new("artifact.glb"), &artifact).is_ok());

        let fragment_bytes = fs::read(member_root.join("contact-fragment.json")).unwrap();
        let evidence_bytes = fs::read(member_root.join("evidence.json")).unwrap();
        let fragment: Value = serde_json::from_slice(&fragment_bytes).unwrap();
        let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
        let aggregate_member = &aggregate["members"][index];
        let member_id = format!("com.example/{}", if index == 0 { "a" } else { "b" });

        assert_eq!(
            fragment["schema"],
            "urn:animsmith:schema:contact-fragment:1"
        );
        assert_eq!(fragment["clip"]["logical_id"], member_id);
        assert_eq!(
            evidence["schema"],
            "urn:animsmith:schema:foot-cycle-member-evidence:1"
        );
        assert_eq!(evidence["member_index"], index);
        assert_eq!(evidence["member_id"], member_id);
        assert_eq!(
            evidence["paths"]["contact_fragment"],
            format!("members/{index:06}/contact-fragment.json")
        );
        assert_eq!(
            evidence["paths"]["evidence"],
            format!("members/{index:06}/evidence.json")
        );
        assert_identity_matches_bytes(
            &aggregate_member["output_contact_fragment"],
            &fragment_bytes,
        );
        assert_identity_matches_bytes(&aggregate_member["evidence"], &evidence_bytes);
        assert_identity_matches_bytes(&evidence["output"]["contact_fragment"], &fragment_bytes);
    }
}

#[test]
fn public_command_admits_double_support_and_a_seam_split_stance() {
    let fixture = FootCycleFixture::create_with_cyclic_contacts(true);
    let result = fixture.run();
    assert_success(&result);
    assert!(
        fixture
            .destination
            .join("aggregate-evidence.json")
            .is_file()
    );

    let fragment: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .destination
                .join("members/000000/contact-fragment.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let source_fragment: Value =
        serde_json::from_slice(&fs::read(fixture.root.join("contacts/a.json")).unwrap()).unwrap();
    assert_eq!(
        fragment["events"], source_fragment["events"],
        "the identity reference transform preserves every physical event"
    );
    let left_windows = fragment["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["role"] == "left_foot" && event.get("window").is_some())
        .collect::<Vec<_>>();
    assert_eq!(
        left_windows.len(),
        2,
        "seam fragments stay physically linear"
    );
    assert!(
        left_windows
            .iter()
            .any(|event| event["window"]["start"] == 0.0)
    );
    assert!(
        left_windows
            .iter()
            .any(|event| event["window"]["end"] == 1.0)
    );
    assert_eq!(
        fragment["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["role"] == "left_foot" && event.get("time").is_some())
            .count(),
        2,
        "each physical seam fragment keeps its marker"
    );
}

#[test]
fn proof_stage_refusal_is_canonical_exit_one() {
    let fixture = FootCycleFixture::create();
    let parameterization = fs::read_to_string(&fixture.parameterization).unwrap();
    let stricter = parameterization.replacen(
        "max_gait_phase_spread = 0.08",
        "max_gait_phase_spread = 0.0",
        1,
    );
    assert_ne!(stricter, parameterization);
    fs::write(&fixture.parameterization, stricter).unwrap();

    let result = fixture.run();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty());
    let refusal: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(refusal["schema"], "urn:animsmith:schema:producer-refusal:1");
    assert_eq!(refusal["command"], "collection-transform-foot-cycle");
    assert_eq!(refusal["rejection"]["stage"], "proof");
    assert_eq!(refusal["rejection"]["kind"], "proof-failed");
    assert_eq!(
        refusal["rejection"]["detail"],
        "foot-cycle proof failed (GaitSpread)"
    );
    assert!(!fixture.destination.exists());
}

#[test]
fn public_command_splits_plan_and_source_refusals_from_operator_failures() {
    let noncanonical = FootCycleFixture::create();
    let fragment = noncanonical.root.join("contacts/a.json");
    let mut bytes = fs::read(&fragment).unwrap();
    bytes.push(b' ');
    fs::write(&fragment, bytes).unwrap();
    let result = noncanonical.run();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("contact-invalid"));
    assert!(!noncanonical.destination.exists());

    let mismatch = FootCycleFixture::create();
    rewrite_canonical_json(&mismatch.root.join("contacts/a.json"), |fragment| {
        fragment["clip"]["logical_id"] = json!("com.example/b");
    });
    let result = mismatch.run();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("plan-binding-mismatch"));
    assert!(!mismatch.destination.exists());

    let unsupported = FootCycleFixture::create();
    rewrite_canonical_json(&unsupported.root.join("contacts/a.json"), |fragment| {
        fragment["extensions"][0]["schema"] = json!("urn:example:unsupported-contact-extension");
    });
    let result = unsupported.run();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty());
    let refusal: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(refusal["schema"], "urn:animsmith:schema:producer-refusal:1");
    assert_eq!(refusal["command"], "collection-transform-foot-cycle");
    assert_eq!(refusal["rejection"]["stage"], "analysis");
    assert_eq!(refusal["rejection"]["kind"], "asset-recipe-mismatch");
    assert!(!unsupported.destination.exists());

    let invalid_topology = FootCycleFixture::create();
    rewrite_canonical_json(&invalid_topology.root.join("contacts/a.json"), |fragment| {
        for event in fragment["events"].as_array_mut().unwrap() {
            if event["role"] == "right_foot" {
                event["role"] = json!("left_foot");
            }
        }
    });
    let result = invalid_topology.run();
    assert_eq!(
        result.status.code(),
        Some(1),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    assert!(result.stderr.is_empty());
    let refusal: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(refusal["rejection"]["stage"], "analysis");
    assert_eq!(refusal["rejection"]["kind"], "asset-recipe-mismatch");
    assert!(!invalid_topology.destination.exists());

    let invalid_asset = FootCycleFixture::create();
    fs::write(invalid_asset.root.join("assets/a.gltf"), b"{}").unwrap();
    let result = invalid_asset.run();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty());
    let refusal: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(refusal["rejection"]["stage"], "load");
    assert_eq!(refusal["rejection"]["kind"], "invalid-asset-structure");
    assert!(!invalid_asset.destination.exists());

    let missing_config = FootCycleFixture::create();
    fs::remove_file(missing_config.root.join("config.toml")).unwrap();
    let result = missing_config.run();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert_eq!(
        result.stderr,
        b"animsmith: foot-cycle source preparation failed (control)\n"
    );
    assert!(!missing_config.destination.exists());
}

#[test]
fn public_runs_are_deterministic_and_destination_race_preserves_the_winner() {
    let first = FootCycleFixture::create();
    let second = FootCycleFixture::create();
    let first_result = first.run();
    let second_result = second.run();
    assert_success(&first_result);
    assert_success(&second_result);
    assert_eq!(first_result.stdout, second_result.stdout);
    assert_eq!(snapshot(&first.destination), snapshot(&second.destination));

    let winner = snapshot(&first.destination);
    let race = first.run();
    assert_eq!(race.status.code(), Some(2));
    assert!(race.stdout.is_empty());
    assert_eq!(snapshot(&first.destination), winner);
}

/// `collection transform-foot-cycle` with a stdout nobody is reading.
///
/// The reader-less stdout comes from [`ClosedStream::closed_stdout`], which
/// explains why it is built in the child rather than here.
#[test]
fn closed_stdout_after_publication_keeps_success_and_recovery_files() {
    let fixture = FootCycleFixture::create();
    let result = transform_command(&fixture.manifest, &fixture.parameterization)
        .closed_stdout()
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("cannot write JSON output to stdout"));
    assert_eq!(count_files(&fixture.destination), 7);
    let aggregate = fs::read(fixture.destination.join("aggregate-evidence.json")).unwrap();
    assert!(serde_json::from_slice::<Value>(&aggregate).is_ok());
}

#[cfg(not(feature = "fbx"))]
#[test]
fn no_default_fbx_source_reaches_format_admission_as_operator_error() {
    let fixture = FootCycleFixture::create();
    fs::copy(
        fixture.root.join("assets/a.gltf"),
        fixture.root.join("assets/a.fbx"),
    )
    .unwrap();
    let manifest = fs::read_to_string(&fixture.manifest).unwrap().replacen(
        "path = \"a.gltf\"",
        "path = \"a.fbx\"",
        1,
    );
    fs::write(&fixture.manifest, manifest).unwrap();
    rebind_parameterization(&fixture);
    let result = fixture.run();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("source-load-operator"),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!fixture.destination.exists());
}

#[test]
fn exact_json_only_command_is_present_without_defaults_or_output_override() {
    let missing = tempfile::tempdir().unwrap();
    let manifest = missing.path().join("collection.toml");
    let parameterization = missing.path().join("foot-cycle.toml");
    let result = animsmith()
        .args([
            "collection",
            "transform-foot-cycle",
            manifest.to_str().unwrap(),
            "--parameterization",
            parameterization.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("foot-cycle source preparation failed (control)")
    );

    for (extra, diagnostic) in [
        (vec!["--output", "other"], "unexpected argument '--output'"),
        (vec!["--format", "text"], "invalid value 'text'"),
    ] {
        let mut command = animsmith();
        command.args([
            "collection",
            "transform-foot-cycle",
            manifest.to_str().unwrap(),
            "--parameterization",
            parameterization.to_str().unwrap(),
        ]);
        command.args(extra);
        let result = command.output().unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .contains(diagnostic)
        );
    }

    let omitted_format = animsmith()
        .args([
            "collection",
            "transform-foot-cycle",
            manifest.to_str().unwrap(),
            "--parameterization",
            parameterization.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(omitted_format.status.code(), Some(2));
    assert!(omitted_format.stdout.is_empty());
    assert!(
        String::from_utf8(omitted_format.stderr)
            .unwrap()
            .contains("required arguments were not provided:\n  --format <FORMAT>")
    );
}

#[test]
fn collection_transform_refuses_global_document_config_before_loading() {
    let result = animsmith()
        .args([
            "--config",
            "config.toml",
            "collection",
            "transform-foot-cycle",
            "collection.toml",
            "--parameterization",
            "foot-cycle.toml",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("--config is not accepted by collection commands")
    );
}

/// The reported 18-key, 0.567 s locomotion set.
///
/// On the shipped builder this set refused at the transform stage with
/// `TimeCollision { track_index: 0 }`, because member `b`'s interior control
/// points narrow onto authored keys and the builder then demanded that
/// recomputing the map through the authored time reproduce the control
/// point's own narrowed output bit for bit.
#[test]
fn eighteen_key_thirty_fps_set_publishes() {
    assert_thirty_fps_set_publishes(18);
}

/// The reported 40-key, 1.300 s locomotion set.
///
/// On the shipped builder this set took the other branch: the extra knot was
/// emitted and the `ClipMap` proof then refused, because the proof decided
/// coincidence by comparing a widened authored time against the unrounded
/// binary64 product and so expected knots the builder had not emitted.
#[test]
fn forty_key_thirty_fps_set_publishes() {
    assert_thirty_fps_set_publishes(40);
}

/// Publish one 30 fps locomotion set and check both members' emitted key times.
///
/// The reference member `a` carries the identity map, so its published clip
/// keeps the authored times verbatim; member `b` runs the same gait
/// [`MEMBER_B_PHASE_OFFSET`] later, so its published clip must carry a
/// different, still strictly increasing, set of times over the same interval.
fn assert_thirty_fps_set_publishes(frames: usize) {
    let fixture = FootCycleFixture::create_at_thirty_fps(frames);
    let result = fixture.run();
    assert_success(&result);
    assert_eq!(count_files(&fixture.destination), 7);
    let aggregate: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(aggregate["resources"]["members"], 2);
    assert_eq!(aggregate["members"][1]["member_id"], "com.example/b");

    let authored = (0..frames)
        .map(|index| index as f32 / 30.0)
        .collect::<Vec<_>>();
    let duration = authored[frames - 1];
    for (index, identity) in [(0_usize, true), (1, false)] {
        let member_root = fixture.destination.join(format!("members/{index:06}"));
        assert!(member_root.join("evidence.json").is_file());
        let tracks = published_track_times(&fs::read(member_root.join("artifact.glb")).unwrap());
        assert!(!tracks.is_empty());
        for times in &tracks {
            assert_eq!(times.first(), Some(&0.0));
            assert_eq!(times.last(), Some(&duration));
            assert!(
                times.windows(2).all(|pair| pair[0] < pair[1]),
                "member {index} emitted non-increasing times {times:?}"
            );
            if identity {
                assert_eq!(
                    times, &authored,
                    "the reference member's identity map copies its authored keys"
                );
            } else {
                assert_ne!(
                    times, &authored,
                    "member b's non-identity time warp must move its keys"
                );
            }
        }
    }
}

/// Every published clip track's key times, in source order.
fn published_track_times(artifact: &[u8]) -> Vec<Vec<f32>> {
    let document = animsmith_gltf::load_source_bytes(Path::new("artifact.glb"), artifact)
        .unwrap()
        .into_document();
    document
        .clips
        .iter()
        .flat_map(|clip| clip.tracks.iter().map(|track| track.times.clone()))
        .collect()
}

/// Every ordinary key count from the reported range publishes.
///
/// Stance boundaries come from authored frame indices, so each interior
/// control point is an authored key; whether reconstructing its instant lands
/// on that key or one binary32 place beside it varies with the key count, and
/// before this contract it decided between a transform refusal, a `ClipMap`
/// proof failure, and publication. This is the only test that carries the
/// whole range through the independent proof and the publication transaction;
/// the builder's own sweep over three frame rates is a core unit test.
#[test]
fn every_key_count_from_eighteen_to_sixty_one_publishes() {
    for frames in 18..=61 {
        let fixture = FootCycleFixture::create_at_thirty_fps(frames);
        let result = fixture.run();
        assert_eq!(
            result.status.code(),
            Some(0),
            "{frames} keys: {}",
            String::from_utf8_lossy(&result.stdout),
        );
        assert!(
            fixture
                .destination
                .join("members/000001/artifact.glb")
                .is_file()
        );
    }
}
