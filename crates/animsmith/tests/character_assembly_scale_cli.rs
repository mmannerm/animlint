//! Public-boundary coverage for character-assembly rest/bind scale contracts.

#![cfg(feature = "fbx")]

use animsmith_core::glam::{Mat4, Quat, Vec3};
use animsmith_core::model::{
    Document, Interpolation, Property, SourceNodeLocalRest, Track, TrackValues, Transform,
};
use animsmith_core::scale::{
    AssemblyScaleBasis, ScaleOperation, ScaleRequest, assembly_scale_basis, plan_scale,
};
use animsmith_core::sha256_hex;
use animsmith_testkit::{
    rest_bind_scale_rig_glb, rest_bind_scale_rig_gltf, unaffected_bind_scale_rig_glb,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Output};

const RECIPE_SCHEMA: &str =
    include_str!("../../../docs/schemas/character-assembly-recipe-v4.schema.json");
const EVIDENCE_SCHEMA: &str =
    include_str!("../../../docs/schemas/character-assembly-evidence-v4.schema.json");
const RECIPE_SCHEMA_V5: &str =
    include_str!("../../../docs/schemas/character-assembly-recipe-v5.schema.json");
const EVIDENCE_SCHEMA_V5: &str =
    include_str!("../../../docs/schemas/character-assembly-evidence-v5.schema.json");
const RECIPE_SCHEMA_V6: &str =
    include_str!("../../../docs/schemas/character-assembly-recipe-v6.schema.json");
const EVIDENCE_SCHEMA_V6: &str =
    include_str!("../../../docs/schemas/character-assembly-evidence-v6.schema.json");
const RECIPE_SCHEMA_V7: &str =
    include_str!("../../../docs/schemas/character-assembly-recipe-v7.schema.json");
const EVIDENCE_SCHEMA_V7: &str =
    include_str!("../../../docs/schemas/character-assembly-evidence-v7.schema.json");
const RIGGED_TRIANGLE_FBX: &str = include_str!("../../animsmith-fbx/testdata/rigged_triangle.fbx");
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
    0x00, 0x05, 0xFE, 0x02, 0xFE, 0xA7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn recipe(clip: &str) -> String {
    format!(
        r#"schema_version = 4
schema = "urn:animsmith:schema:character-assembly-recipe:4"
input_root = "inputs"
base_input = "base.glb"
fps = 30.0

[rest_bind_scale]
source_skin_index = 0
source_root_node_index = 0
expected_factor = 0.01

[[clips]]
name = "walk"
input = "{clip}"
take = "clip"
"#
    )
}

fn recipe_v5(clip: &str) -> String {
    recipe(clip)
        .replacen("schema_version = 4", "schema_version = 5", 1)
        .replacen(
            "urn:animsmith:schema:character-assembly-recipe:4",
            "urn:animsmith:schema:character-assembly-recipe:5",
            1,
        )
        .replacen(
            "fps = 30.0",
            "fps = 30.0\ncanonicalize_skin = true\nground_and_center = true\nremove_nodes = [\"attach\"]",
            1,
        )
}

fn fbx_recipe_v6(clip: &str) -> String {
    format!(
        r#"schema_version = 6
schema = "urn:animsmith:schema:character-assembly-recipe:6"
input_root = "inputs"
base_input = "base.fbx"
fps = 30.0

[rest_bind_scale]
source_skin_index = 0
source_root_node_index = 1
expected_factor = 0.01

[[clips]]
name = "walk"
input = "{clip}"
take = "take"
"#
    )
}

fn fbx_recipe_v7(clip: &str) -> String {
    format!(
        r#"schema_version = 7
schema = "urn:animsmith:schema:character-assembly-recipe:7"
input_root = "inputs"
base_input = "base.fbx"
fps = 30.0

[rest_bind_scale]
root_node_name = "root"
expected_factor = 0.01

[[clips]]
name = "walk"
input = "{clip}"
take = "take"
"#
    )
}

fn user_property_fbx() -> String {
    RIGGED_TRIANGLE_FBX.replacen(
        "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1",
        "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1\n\t\t\tP: \"PipelineTag\", \"KString\", \"\", \"U\",\"unsupported\"",
        1,
    )
}

fn nonbearing_node_attributes_fbx(include_pose: bool) -> String {
    let mut objects = concat!(
        "\tNodeAttribute: 5101, \"NodeAttribute::marker\", \"FKEffector\" {}\n",
        "\tNodeAttribute: 5102, \"NodeAttribute::lod\", \"LodGroup\" {}\n",
        "\tNodeAttribute: 5103, \"NodeAttribute::stereo\", \"CameraStereo\" {}\n",
        "\tNodeAttribute: 5104, \"NodeAttribute::switcher\", \"CameraSwitcher\" {}\n",
    )
    .to_owned();
    if include_pose {
        objects.push_str(concat!(
            "\tPose: 5105, \"Pose::unsupported\", \"BindPose\" {\n",
            "\t\tType: \"BindPose\"\n",
            "\t\tVersion: 100\n",
            "\t\tNbPoseNodes: 0\n",
            "\t}\n",
        ));
    }
    RIGGED_TRIANGLE_FBX.replace("\r\n", "\n").replacen(
        "\tAnimationStack: 3001",
        &format!("{objects}\tAnimationStack: 3001"),
        1,
    )
}

fn display_layer_fbx() -> String {
    let display_layer = concat!(
        "\tCollectionExclusive: 5401, \"DisplayLayer::animsmith-test\", \"DisplayLayer\" {\n",
        "\t\tProperties70: {\n",
        "\t\t\tP: \"Color\", \"ColorRGB\", \"Color\", \"\",0.1,0.2,0.3\n",
        "\t\t\tP: \"Show\", \"bool\", \"\", \"\",0\n",
        "\t\t\tP: \"Freeze\", \"bool\", \"\", \"\",1\n",
        "\t\t}\n",
        "\t}\n",
    );
    RIGGED_TRIANGLE_FBX
        .replace("\r\n", "\n")
        .replacen(
            "\tAnimationStack: 3001",
            &format!("{display_layer}\tAnimationStack: 3001"),
            1,
        )
        .replacen("Connections: {", "Connections: {\n\tC: \"OO\",1001,5401", 1)
}

const IDENTITY_FBX_MATRIX: &str = "1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1";

fn bind_pose_with_shader_fbx(root_matrix: &str) -> String {
    let objects = format!(
        concat!(
            "\tImplementation: 5201, \"Implementation::shader\", \"\" {{}}\n",
            "\tBindingTable: 5202, \"BindingTable::binding\", \"\" {{}}\n",
            "\tPose: 5203, \"Pose::bind\", \"BindPose\" {{\n",
            "\t\tType: \"BindPose\"\n",
            "\t\tVersion: 100\n",
            "\t\tNbPoseNodes: 2\n",
            "\t\tPoseNode: {{\n",
            "\t\t\tNode: 1001\n",
            "\t\t\tMatrix: *16 {{ a: {} }}\n",
            "\t\t}}\n",
            "\t\tPoseNode: {{\n",
            "\t\t\tNode: 1002\n",
            "\t\t\tMatrix: *16 {{ a: {} }}\n",
            "\t\t}}\n",
            "\t}}\n",
        ),
        root_matrix, IDENTITY_FBX_MATRIX,
    );
    RIGGED_TRIANGLE_FBX.replace("\r\n", "\n").replacen(
        "\tAnimationStack: 3001",
        &format!("{objects}\tAnimationStack: 3001"),
        1,
    )
}

fn external_normal_texture_with_unmodeled_pose_fbx() -> String {
    external_normal_texture_fbx().replacen(
        "\tAnimationStack: 3001",
        concat!(
            "\tPose: 6001, \"Pose::unsupported\", \"BindPose\" {\n",
            "\t\tType: \"BindPose\"\n",
            "\t\tVersion: 100\n",
            "\t\tNbPoseNodes: 0\n",
            "\t}\n",
            "\tAnimationStack: 3001"
        ),
        1,
    )
}

fn external_normal_texture_fbx() -> String {
    let source = RIGGED_TRIANGLE_FBX.replace("\r\n", "\n");
    let source = source.replacen(
        "\tObjectType: \"Deformer\" { Count: 2 }\n}",
        "\tObjectType: \"Deformer\" { Count: 2 }\n\tObjectType: \"Material\" { Count: 1 }\n\tObjectType: \"Texture\" { Count: 1 }\n\tObjectType: \"Video\" { Count: 1 }\n}",
        1,
    );
    let objects = r#"	Material: 5001, "Material::normal_mat", "" {
		Version: 102
		ShadingModel: "phong"
		MultiLayer: 0
	}
	Texture: 5002, "Texture::normal", "" {
		Type: "TextureVideoClip"
		Version: 202
		TextureName: "Texture::normal"
		Media: "Video::normal"
		FileName: "normal.png"
		RelativeFilename: "normal.png"
		ModelUVTranslation: 0,0
		ModelUVScaling: 1,1
		Texture_Alpha_Source: "None"
		Cropping: 0,0,0,0
	}
	Video: 5003, "Video::normal", "Clip" {
		Type: "Clip"
		Properties70: {
			P: "Path", "KString", "XRefUrl", "", "normal.png"
		}
		FileName: "normal.png"
		RelativeFilename: "normal.png"
	}
}
Connections: {"#;
    source
        .replacen("}\nConnections: {", objects, 1)
        .replacen(
            "Connections: {",
            "Connections: {\n\tC: \"OO\",5001,1002\n\tC: \"OP\",5002,5001,\"NormalMap\"\n\tC: \"OO\",5003,5002",
            1,
        )
}

fn write_normalized_fbx_glb(path: &Path) {
    let document =
        animsmith_fbx::load_bytes(Path::new("source.fbx"), RIGGED_TRIANGLE_FBX.as_bytes())
            .expect("analytic FBX fixture loads");
    animsmith_gltf::write::write(&document, path).expect("normalized FBX fixture stages as GLB");
}

fn normalized_fbx_gltf_value(path: &Path) -> Value {
    write_normalized_fbx_glb(path);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn remap_gltf_node_indices(value: &mut Value, old_to_new: &[usize]) {
    let map = |index: &mut Value| {
        let old = index.as_u64().unwrap() as usize;
        *index = Value::from(old_to_new[old]);
    };
    for node in value["nodes"].as_array_mut().unwrap() {
        if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
            for child in children {
                map(child);
            }
        }
    }
    for scene in value["scenes"].as_array_mut().unwrap() {
        for node in scene["nodes"].as_array_mut().unwrap() {
            map(node);
        }
    }
    for skin in value["skins"].as_array_mut().unwrap() {
        for joint in skin["joints"].as_array_mut().unwrap() {
            map(joint);
        }
        if let Some(skeleton) = skin.get_mut("skeleton") {
            map(skeleton);
        }
    }
    for animation in value["animations"].as_array_mut().unwrap() {
        for channel in animation["channels"].as_array_mut().unwrap() {
            map(&mut channel["target"]["node"]);
        }
    }
}

fn write_reindexed_and_reskinned_glb(path: &Path) {
    let source = unaffected_bind_scale_rig_glb();
    let source_json_len = u32::from_le_bytes(source[12..16].try_into().unwrap()) as usize;
    let source_bin_header = 20 + source_json_len;
    let source_bin_len = u32::from_le_bytes(
        source[source_bin_header..source_bin_header + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let source_bin = &source[source_bin_header + 8..source_bin_header + 8 + source_bin_len];
    let mut value: Value = serde_json::from_slice(&source[20..20 + source_json_len]).unwrap();
    let node_count = value["nodes"].as_array().unwrap().len();
    value["nodes"].as_array_mut().unwrap().swap(0, 1);
    let mut old_to_new = (0..node_count).collect::<Vec<_>>();
    old_to_new.swap(0, 1);
    remap_gltf_node_indices(&mut value, &old_to_new);
    value["skins"].as_array_mut().unwrap().swap(0, 1);
    for node in value["nodes"].as_array_mut().unwrap() {
        if let Some(skin) = node.get_mut("skin") {
            *skin = Value::from(1 - skin.as_u64().unwrap());
        }
    }
    let mut json = serde_json::to_vec(&value).unwrap();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let total_len = 12 + 8 + json.len() + 8 + source_bin.len();
    let mut glb = Vec::with_capacity(total_len);
    glb.extend_from_slice(&0x4654_6c67u32.to_le_bytes());
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());
    glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f_534au32.to_le_bytes());
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(source_bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e_4942u32.to_le_bytes());
    glb.extend_from_slice(source_bin);
    std::fs::write(path, glb).unwrap();
}

fn normalized_fbx_stage_bytes(dir: &Path, name: &str, source: &[u8]) -> Vec<u8> {
    let document = animsmith_fbx::load_bytes(Path::new("source.fbx"), source)
        .expect("analytic FBX fixture loads");
    let stage = dir.join(format!("expected-private-{name}-stage.glb"));
    animsmith_gltf::write::write(&document, &stage).expect("normalized FBX fixture stages as GLB");
    std::fs::read(stage).expect("reads independently staged normalized FBX GLB")
}

fn translated_fbx_clip() -> String {
    RIGGED_TRIANGLE_FBX.replacen(
        "KeyValueFloat: *2 { a: 0,100 }",
        "KeyValueFloat: *2 { a: 0,200 }",
        1,
    )
}

fn dual_quaternion_animation_fbx() -> String {
    with_dual_quaternion_skin(rigged_limb_triangle_fbx())
}

fn dual_quaternion_material_animation_fbx() -> String {
    with_dual_quaternion_skin(external_normal_texture_fbx()).replace(
        "Vertices: *9 { a: 0,0,0,100,0,0,0,100,0 }",
        "Vertices: *9 { a: 0,0,0,900,0,0,0,900,0 }",
    )
}

fn with_dual_quaternion_skin(source: String) -> String {
    source.replacen(
        "\t\tLink_DeformAcuracy: 50",
        concat!(
            "\t\tLink_DeformAcuracy: 50\n",
            "\t\tSkinningType: \"Blend\"\n",
            "\t\tIndexes: *3 { a: 0,1,2 }\n",
            "\t\tBlendWeights: *3 { a: 1,1,1 }",
        ),
        1,
    )
}

fn rigged_limb_triangle_fbx() -> String {
    RIGGED_TRIANGLE_FBX
        .replace(
            "Model: 1002, \"Model::tri\", \"Mesh\"",
            "Model: 1002, \"Model::tri\", \"Limb\"",
        )
        .replace("\tC: \"OO\",1001,4002", "\tC: \"OO\",1002,4002")
}

fn rigged_limb_triangle_with_unskinned_prop_fbx() -> String {
    rigged_limb_triangle_fbx()
        .replace("\r\n", "\n")
        .replace(
            "ObjectType: \"Model\" { Count: 2 }",
            "ObjectType: \"Model\" { Count: 3 }",
        )
        .replace(
            "ObjectType: \"Geometry\" { Count: 1 }",
            "ObjectType: \"Geometry\" { Count: 2 }",
        )
        .replacen(
            "\tAnimationStack: 3001",
            concat!(
                "\tGeometry: 2002, \"Geometry::prop\", \"Mesh\" {\n",
                "\t\tVertices: *9 { a: 0,0,0,10,0,0,0,10,0 }\n",
                "\t\tPolygonVertexIndex: *3 { a: 0,1,-3 }\n",
                "\t}\n",
                "\tModel: 1003, \"Model::prop\", \"Mesh\" {\n",
                "\t\tVersion: 232\n",
                "\t\tProperties70: {\n",
                "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",50,0,0\n",
                "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0\n",
                "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1\n",
                "\t\t}\n",
                "\t}\n",
                "\tAnimationStack: 3001",
            ),
            1,
        )
        .replacen(
            "Connections: {",
            concat!(
                "Connections: {\n",
                "\tC: \"OO\",1003,1001\n",
                "\tC: \"OO\",2002,1003",
            ),
            1,
        )
}

fn skinless_animation_fbx() -> String {
    rigged_limb_triangle_fbx()
        .replace("\r\n", "\n")
        .replace(
            concat!(
                "\tGeometry: 2001, \"Geometry::tri\", \"Mesh\" {\n",
                "\t\tVertices: *9 { a: 0,0,0,100,0,0,0,100,0 }\n",
                "\t\tPolygonVertexIndex: *3 { a: 0,1,-3 }\n",
                "\t}\n",
            ),
            "",
        )
        .replace(
            concat!(
                "\tDeformer: 4001, \"Deformer::skin\", \"Skin\" {\n",
                "\t\tVersion: 101\n",
                "\t\tLink_DeformAcuracy: 50\n",
                "\t}\n",
                "\tDeformer: 4002, \"SubDeformer::root_cluster\", \"Cluster\" {\n",
                "\t\tVersion: 100\n",
                "\t\tIndexes: *3 { a: 0,1,2 }\n",
                "\t\tWeights: *3 { a: 1,1,1 }\n",
                "\t\tTransform: *16 { a: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1 }\n",
                "\t\tTransformLink: *16 { a: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1 }\n",
                "\t}\n",
            ),
            "",
        )
        .replace("\tC: \"OO\",2001,1002\n", "")
        .replace("\tC: \"OO\",4001,2001\n", "")
        .replace("\tC: \"OO\",4002,4001\n", "")
        .replace("\tC: \"OO\",1001,4002\n", "")
        .replace("\tC: \"OO\",1002,4002\n", "")
        .replace(
            "\tC: \"OP\",3003,1001,\"Lcl Translation\"",
            "\tC: \"OP\",3003,1002,\"Lcl Translation\"",
        )
}

fn document_local_bone_order_fbx() -> String {
    let node = |id: u32, name: &str| {
        format!(
            concat!(
                "\tModel: {id}, \"Model::{name}\", \"Null\" {{\n",
                "\t\tVersion: 232\n",
                "\t\tProperties70: {{\n",
                "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0\n",
                "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0\n",
                "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1\n",
                "\t\t}}\n",
                "\t}}\n",
            ),
            id = id,
            name = name,
        )
    };
    let branch = node(1003, "branch");
    let remove = node(1004, "remove");
    let retained_second = node(1005, "retained-second");
    rigged_limb_triangle_fbx()
        .replace("\r\n", "\n")
        .replacen("\tCount: 8", "\tCount: 11", 1)
        .replacen(
            "ObjectType: \"Model\" { Count: 2 }",
            "ObjectType: \"Model\" { Count: 5 }",
            1,
        )
        .replacen("\tGeometry: 2001", &format!("{branch}\tGeometry: 2001"), 1)
        .replacen(
            "Model: 1002, \"Model::tri\", \"Limb\"",
            "Model: 1002, \"Model::retained-first\", \"Limb\"",
            1,
        )
        .replacen(
            "\tDeformer: 4001",
            &format!("{remove}{retained_second}\tDeformer: 4001"),
            1,
        )
        .replacen(
            "Connections: {",
            concat!(
                "Connections: {\n",
                "\tC: \"OO\",1003,1001\n",
                "\tC: \"OO\",1004,1001\n",
                "\tC: \"OO\",1005,1003",
            ),
            1,
        )
}

fn write_document_local_order_clip(
    path: &Path,
    source: &str,
    animated: &[&str],
    constant_channels_for: Option<&str>,
) {
    let mut document = animsmith_fbx::load_bytes(Path::new("source.fbx"), source.as_bytes())
        .expect("document-local-order FBX fixture loads");
    document.assets.instances.clear();
    document.assets.meshes.clear();
    document.assets.source_skeleton.skins.clear();
    for bone in &mut document.skeleton.bones {
        bone.inverse_bind = None;
    }
    document.clips[0].tracks = animated
        .iter()
        .enumerate()
        .map(|(ordinal, name)| {
            let bone = document
                .skeleton
                .bones
                .iter()
                .position(|bone| bone.name == *name)
                .expect("animated fixture bone exists");
            Track {
                bone,
                property: Property::Translation,
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: TrackValues::Vec3s(vec![
                    Vec3::ZERO,
                    Vec3::new(100.0 + ordinal as f32, 0.0, 0.0),
                ]),
            }
        })
        .collect();
    if let Some(name) = constant_channels_for {
        let bone = unique_bone_id(&document, name);
        document.clips[0].tracks.extend([
            Track {
                bone,
                property: Property::Rotation,
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: TrackValues::Quats(vec![Quat::IDENTITY; 2]),
            },
            Track {
                bone,
                property: Property::Scale,
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: TrackValues::Vec3s(vec![Vec3::ONE; 2]),
            },
        ]);
    }
    document.clips[0].duration_s = 1.0;
    animsmith_gltf::write::write(&document, path)
        .expect("writes document-local-order skinless clip");
}

fn unique_bone_id(document: &Document, name: &str) -> usize {
    let matches = document
        .skeleton
        .bones
        .iter()
        .enumerate()
        .filter_map(|(index, bone)| (bone.name == name).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "fixture bone {name:?} is exact and unique"
    );
    matches[0]
}

fn assert_document_local_bone_id_alias(dir: &Path, source: &str) {
    let authoritative =
        animsmith_fbx::load_bytes(Path::new("base.fbx"), source.as_bytes()).unwrap();
    let staged_path = dir.join("document-local-order-private-stage.glb");
    animsmith_gltf::write::write(&authoritative, &staged_path).unwrap();
    let reference = animsmith_gltf::load(&staged_path).unwrap();
    let authoritative_retained = unique_bone_id(&authoritative, "retained-second");
    let reference_removed = unique_bone_id(&reference, "remove");
    assert_eq!(
        authoritative_retained, reference_removed,
        "fixture must alias a retained authoritative BoneId to a removed reference BoneId"
    );
    assert_ne!(
        authoritative_retained,
        unique_bone_id(&reference, "retained-second"),
        "the stable retained identity must move in the reference namespace"
    );
}

fn two_centimetre_unit_fbx(source: &str) -> String {
    source
        .replace(
            "P: \"UnitScaleFactor\", \"double\", \"Number\", \"\",1",
            "P: \"UnitScaleFactor\", \"double\", \"Number\", \"\",2",
        )
        .replace(
            "P: \"OriginalUnitScaleFactor\", \"double\", \"Number\", \"\",1",
            "P: \"OriginalUnitScaleFactor\", \"double\", \"Number\", \"\",2",
        )
}

fn write_skinless_cubic_clip(path: &Path) {
    let mut document = animsmith_fbx::load_bytes(
        Path::new("source.fbx"),
        rigged_limb_triangle_fbx().as_bytes(),
    )
    .expect("analytic FBX fixture loads");
    document.assets.instances.clear();
    document.assets.meshes.clear();
    document.assets.source_skeleton.skins.clear();
    for bone in &mut document.skeleton.bones {
        bone.inverse_bind = None;
    }
    let track = &mut document.clips[0].tracks[0];
    track.bone = 2;
    track.interpolation = Interpolation::CubicSpline;
    track.times = vec![0.0, 1.0];
    track.values = TrackValues::Vec3s(vec![
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        Vec3::new(30.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(40.0, 0.0, 0.0),
    ]);
    document.clips[0].tracks.truncate(1);
    let root = document
        .skeleton
        .bones
        .iter()
        .position(|bone| bone.name == "root")
        .expect("fixture has root");
    let limb = document
        .skeleton
        .bones
        .iter()
        .position(|bone| bone.name == "tri")
        .expect("fixture has animated limb");
    document.clips[0].tracks.extend([
        Track {
            bone: root,
            property: Property::Scale,
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0],
            values: TrackValues::Vec3s(vec![Vec3::splat(2.0), Vec3::splat(3.0)]),
        },
        Track {
            bone: limb,
            property: Property::Rotation,
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0],
            values: TrackValues::Quats(vec![Quat::IDENTITY, Quat::from_rotation_z(0.5)]),
        },
    ]);
    document.clips[0].duration_s = 1.0;
    animsmith_gltf::write::write(&document, path).expect("writes skinless cubic clip");
}

fn write_skinless_take_pose_clip(path: &Path) {
    let mut document =
        animsmith_fbx::load_bytes(Path::new("source.fbx"), skinless_animation_fbx().as_bytes())
            .expect("analytic skinless FBX fixture loads");
    let limb = document
        .skeleton
        .bones
        .iter()
        .position(|bone| bone.name == "tri")
        .expect("fixture has animated limb");
    let take_pose = Transform {
        translation: Vec3::new(250.0, 0.0, 0.0),
        rotation: Quat::from_rotation_z(0.25),
        scale: Vec3::splat(2.0),
    };
    document.skeleton.bones[limb].rest = take_pose;
    let source_node = document
        .assets
        .source_skeleton
        .nodes
        .iter_mut()
        .find(|node| node.bone == Some(limb))
        .expect("fixture limb has a projected source node");
    source_node.local_rest = SourceNodeLocalRest::Trs {
        translation: take_pose.translation,
        rotation: take_pose.rotation,
        scale: take_pose.scale,
    };
    document.clips[0].tracks = vec![
        Track {
            bone: limb,
            property: Property::Translation,
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0],
            values: TrackValues::Vec3s(vec![take_pose.translation, Vec3::new(350.0, 0.0, 0.0)]),
        },
        Track {
            bone: limb,
            property: Property::Rotation,
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0],
            values: TrackValues::Quats(vec![take_pose.rotation, Quat::from_rotation_z(0.5)]),
        },
        Track {
            bone: limb,
            property: Property::Scale,
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0],
            values: TrackValues::Vec3s(vec![take_pose.scale, Vec3::splat(3.0)]),
        },
    ];
    document.clips[0].duration_s = 1.0;
    animsmith_gltf::write::write(&document, path).expect("writes skinless take-pose clip");
}

fn write_skinless_outside_domain_clip(base_path: &Path, clip_path: &Path) {
    let mut document = animsmith_gltf::load(base_path).expect("loads unaffected-domain fixture");
    document.assets.instances.clear();
    document.assets.meshes.clear();
    document.assets.source_skeleton.skins.clear();
    for bone in &mut document.skeleton.bones {
        bone.inverse_bind = None;
    }
    let outside = document
        .skeleton
        .bones
        .iter()
        .position(|bone| bone.name == "joint2")
        .expect("fixture has an unaffected sibling root");
    document.clips[0].tracks = vec![Track {
        bone: outside,
        property: Property::Translation,
        interpolation: Interpolation::Linear,
        times: vec![0.0, 1.0],
        values: TrackValues::Vec3s(vec![Vec3::ZERO, Vec3::X]),
    }];
    animsmith_gltf::write::write(&document, clip_path)
        .expect("writes a skinless out-of-domain clip");
}

fn unskinned_prop_fbx() -> String {
    let source = RIGGED_TRIANGLE_FBX.replace("\r\n", "\n");
    let prop = concat!(
        "\tModel: 1004, \"Model::prop-parent\", \"Null\" {\n",
        "\t\tVersion: 232\n",
        "\t\tProperties70: {\n",
        "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0\n",
        "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0\n",
        "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1\n",
        "\t\t}\n",
        "\t}\n",
        "\tGeometry: 2002, \"Geometry::prop\", \"Mesh\" {\n",
        "\t\tVertices: *9 { a: 0,0,0,10,0,0,0,10,0 }\n",
        "\t\tPolygonVertexIndex: *3 { a: 0,1,-3 }\n",
        "\t}\n",
        "\tModel: 1003, \"Model::prop\", \"Mesh\" {\n",
        "\t\tVersion: 232\n",
        "\t\tProperties70: {\n",
        "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0\n",
        "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0\n",
        "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1\n",
        "\t\t}\n",
        "\t}\n",
    );
    source
        .replacen("\tCount: 8", "\tCount: 11", 1)
        .replacen(
            "ObjectType: \"Model\" { Count: 2 }",
            "ObjectType: \"Model\" { Count: 4 }",
            1,
        )
        .replacen(
            "ObjectType: \"Geometry\" { Count: 1 }",
            "ObjectType: \"Geometry\" { Count: 2 }",
            1,
        )
        .replacen("\tDeformer: 4001", &format!("{prop}\tDeformer: 4001"), 1)
        .replacen(
            "Connections: {",
            "Connections: {\n\tC: \"OO\",1004,1001\n\tC: \"OO\",1003,1004\n\tC: \"OO\",2002,1003",
            1,
        )
}

fn renamed_skinned_holder_with_unselected_prop_fbx() -> String {
    let materials = concat!(
        "\tMaterial: 5001, \"Material::body-material\", \"\" {\n",
        "\t\tVersion: 102\n",
        "\t\tShadingModel: \"phong\"\n",
        "\t}\n",
        "\tMaterial: 5002, \"Material::prop-material\", \"\" {\n",
        "\t\tVersion: 102\n",
        "\t\tShadingModel: \"phong\"\n",
        "\t}\n",
    );
    unskinned_prop_fbx()
        .replacen("\tCount: 11", "\tCount: 13", 1)
        .replacen(
            "\tObjectType: \"Deformer\" { Count: 2 }",
            concat!(
                "\tObjectType: \"Deformer\" { Count: 2 }\n",
                "\tObjectType: \"Material\" { Count: 2 }",
            ),
            1,
        )
        .replacen(
            "\tModel: 1002, \"Model::tri\", \"Mesh\"",
            "\tModel: 1002, \"Model::body-instance\", \"Mesh\"",
            1,
        )
        .replacen(
            "\tDeformer: 4001",
            &format!("{materials}\tDeformer: 4001"),
            1,
        )
        .replacen(
            "Connections: {",
            concat!(
                "Connections: {\n",
                "\tC: \"OO\",5001,1002\n",
                "\tC: \"OO\",5002,1003",
            ),
            1,
        )
}

fn scale_invariant_fidelity_fbx() -> String {
    let payload = r#"
		Edges: *4 { a: 0,1,2,3 }
		LayerElementUV: 0 {
			Version: 101
			Name: "uv"
			MappingInformationType: "ByPolygonVertex"
			ReferenceInformationType: "Direct"
			UV: *8 { a: 0,0,1,0,1,1,0,1 }
		}
		LayerElementTangent: 0 {
			Version: 101
			Name: "tangent"
			MappingInformationType: "ByPolygonVertex"
			ReferenceInformationType: "Direct"
			Tangents: *12 { a: 1,0,0,1,0,0,1,0,0,1,0,0 }
		}
		LayerElementColor: 0 {
			Version: 101
			Name: "color"
			MappingInformationType: "ByPolygonVertex"
			ReferenceInformationType: "Direct"
			Colors: *16 { a: 1,0,0,1,0,1,0,1,0,0,1,1,1,1,1,1 }
		}
		Layer: 0 {
			Version: 100
			LayerElement: { Type: "LayerElementUV" TypedIndex: 0 }
			LayerElement: { Type: "LayerElementTangent" TypedIndex: 0 }
		}
"#;
    let bone_models = (3..=7)
        .map(|suffix| format!("\tModel: 100{suffix}, \"Model::bone{suffix}\", \"Limb\" {{}}\n"))
        .collect::<String>();
    let positive_clusters = (3..=6)
        .map(|suffix| {
            format!(
                concat!(
                    "\tDeformer: 400{0}, \"SubDeformer::bone{0}_cluster\", \"Cluster\" {{\n",
                    "\t\tVersion: 100\n",
                    "\t\tIndexes: *4 {{ a: 0,1,2,3 }}\n",
                    "\t\tWeights: *4 {{ a: 1,1,1,1 }}\n",
                    "\t\tTransform: *16 {{ a: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1 }}\n",
                    "\t\tTransformLink: *16 {{ a: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1 }}\n",
                    "\t}}\n",
                ),
                suffix
            )
        })
        .collect::<String>();
    let rejected_cluster = concat!(
        "\tDeformer: 4007, \"SubDeformer::rejected_cluster\", \"Cluster\" {\n",
        "\t\tVersion: 100\n",
        "\t\tIndexes: *4 { a: 0,1,2,3 }\n",
        "\t\tWeights: *4 { a: -0.25,-0.25,-0.25,-0.25 }\n",
        "\t\tTransform: *16 { a: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1 }\n",
        "\t\tTransformLink: *16 { a: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1 }\n",
        "\t}\n",
    );
    let extra_connections = (3..=7)
        .map(|suffix| {
            format!(
                concat!(
                    "\tC: \"OO\",400{0},4001\n",
                    "\tC: \"OO\",100{0},400{0}\n",
                    "\tC: \"OO\",100{0},1001\n",
                ),
                suffix
            )
        })
        .collect::<String>();
    RIGGED_TRIANGLE_FBX
        .replacen("\tCount: 8", "\tCount: 18", 1)
        .replacen(
            "ObjectType: \"Model\" { Count: 2 }",
            "ObjectType: \"Model\" { Count: 7 }",
            1,
        )
        .replacen(
            "ObjectType: \"Deformer\" { Count: 2 }",
            "ObjectType: \"Deformer\" { Count: 7 }",
            1,
        )
        .replacen(
            "Vertices: *9 { a: 0,0,0,100,0,0,0,100,0 }",
            "Vertices: *12 { a: 0,0,0,100,0,0,100,100,0,0,100,0 }",
            1,
        )
        .replacen(
            "PolygonVertexIndex: *3 { a: 0,1,-3 }",
            &format!("PolygonVertexIndex: *4 {{ a: 0,1,2,-4 }}{payload}"),
            1,
        )
        .replacen("Indexes: *3 { a: 0,1,2 }", "Indexes: *4 { a: 0,1,2,3 }", 1)
        .replacen("Weights: *3 { a: 1,1,1 }", "Weights: *4 { a: 2,2,2,2 }", 1)
        .replacen(
            "\tDeformer: 4001",
            &format!("{bone_models}\tDeformer: 4001"),
            1,
        )
        .replacen(
            "\tAnimationStack: 3001",
            &format!("{positive_clusters}{rejected_cluster}\tAnimationStack: 3001"),
            1,
        )
        .replacen(
            "\tC: \"OO\",4001,2001",
            &format!("{extra_connections}\tC: \"OO\",4001,2001"),
            1,
        )
}

fn write_cubic_asset_from(path: &Path, bytes: &[u8], offset: f32) {
    let mut document = animsmith_gltf::load_bytes(Path::new("source.glb"), bytes).unwrap();
    let track = document.clips[0]
        .tracks
        .iter_mut()
        .find(|track| track.property == Property::Translation)
        .expect("fixture translation track");
    track.interpolation = Interpolation::CubicSpline;
    track.times = vec![0.0, 1.0];
    track.values = TrackValues::Vec3s(vec![
        Vec3::new(offset + 2.0, 1.0, -1.0),
        Vec3::new(offset, 100.0, 2.0),
        Vec3::new(offset + 3.0, 2.0, -2.0),
        Vec3::new(offset + 4.0, 3.0, -3.0),
        Vec3::new(offset, 200.0, 4.0),
        Vec3::new(offset + 5.0, 4.0, -4.0),
    ]);
    animsmith_gltf::write::write(&document, path).expect("writes cubic fixture");
}

fn write_cubic_asset(path: &Path, offset: f32) {
    write_cubic_asset_from(path, &rest_bind_scale_rig_glb(), offset);
}

fn write_scale_sensitive_clip_asset(path: &Path, translation_end_y: f32) {
    let mut document =
        animsmith_gltf::load_bytes(Path::new("source.glb"), &rest_bind_scale_rig_glb()).unwrap();
    let translation = document.clips[0]
        .tracks
        .iter_mut()
        .find(|track| track.property == Property::Translation)
        .expect("fixture translation track");
    translation.values = TrackValues::Vec3s(vec![
        Vec3::new(0.0, 100.0, 0.0),
        Vec3::new(0.0, translation_end_y, 0.0),
    ]);
    let rotation = document.clips[0]
        .tracks
        .iter_mut()
        .find(|track| track.property == Property::Rotation)
        .expect("fixture rotation track");
    rotation.bone = 0;
    rotation.values = TrackValues::Quats(vec![Quat::IDENTITY, Quat::from_rotation_z(0.2)]);
    animsmith_gltf::write::write(&document, path).expect("writes scale-sensitive fixture");
}

fn factor_two_rig_glb() -> Vec<u8> {
    let mut bytes = rest_bind_scale_rig_glb();
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json = &mut bytes[20..20 + json_len];
    let authored = b"\"scale\": [0.01, 0.01, 0.01]";
    let replacement = b"\"scale\": [2.00, 2.00, 2.00]";
    let at = json
        .windows(authored.len())
        .position(|window| window == authored)
        .unwrap();
    json[at..at + authored.len()].copy_from_slice(replacement);
    let bin_start = 20 + json_len + 8;
    let inverse_bind: [f32; 16] = [
        0.5, 0.0, 0.0, 0.0, //
        0.0, 0.5, 0.0, 0.0, //
        0.0, 0.0, 0.5, 0.0, //
        0.0, -100.0, 0.0, 1.0,
    ];
    for (index, value) in inverse_bind.into_iter().enumerate() {
        bytes[bin_start + 108 + index * 4..bin_start + 112 + index * 4]
            .copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn factor_two_named_root_rig_glb() -> Vec<u8> {
    let mut bytes = factor_two_rig_glb();
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json = &mut bytes[20..20 + json_len];
    let authored = b"\"joints\": [1]";
    let replacement = b"\"joints\": [0]";
    let at = json
        .windows(authored.len())
        .position(|window| window == authored)
        .unwrap();
    json[at..at + authored.len()].copy_from_slice(replacement);
    let bin_start = 20 + json_len + 8;
    bytes[bin_start + 108 + 13 * 4..bin_start + 112 + 13 * 4]
        .copy_from_slice(&0.0f32.to_le_bytes());
    bytes
}

fn run(dir: &Path) -> Output {
    run_to(dir, "recipe.toml", "character.glb", "character.json")
}

fn run_to(dir: &Path, recipe: &str, output: &str, evidence: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_animsmith"))
        .current_dir(dir)
        .args([
            "assemble",
            recipe,
            "-o",
            output,
            "--evidence",
            evidence,
            "--format",
            "json",
        ])
        .output()
        .expect("runs assemble")
}

#[test]
fn missing_assembly_destinations_use_the_filesystems_case_semantics() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
    write_cubic_asset(&dir.path().join("inputs/clip.glb"), 10.0);
    std::fs::write(dir.path().join("recipe.toml"), recipe("clip.glb")).unwrap();

    let probe = tempfile::tempdir_in(dir.path()).expect("filesystem-semantics probe");
    std::fs::write(probe.path().join("character.glb"), b"probe").unwrap();
    let case_insensitive = probe.path().join("CHARACTER.GLB").exists();
    drop(probe);

    let output = run_to(dir.path(), "recipe.toml", "character.glb", "CHARACTER.GLB");
    if case_insensitive {
        assert_eq!(
            output.status.code(),
            Some(2),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("artifact and evidence outputs must resolve to different paths")
        );
        assert!(!dir.path().join("character.glb").exists());
        assert!(!dir.path().join("CHARACTER.GLB").exists());
    } else {
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert!(dir.path().join("character.glb").is_file());
        assert!(dir.path().join("CHARACTER.GLB").is_file());
        assert_ne!(
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            std::fs::read(dir.path().join("CHARACTER.GLB")).unwrap()
        );
    }
}

fn rest_worlds(document: &Document) -> Vec<Mat4> {
    let mut worlds = Vec::with_capacity(document.skeleton.bones.len());
    for bone in &document.skeleton.bones {
        let local = bone.rest.to_mat4();
        worlds.push(bone.parent.map_or(local, |parent| worlds[parent] * local));
    }
    worlds
}

fn refusal_detail(output: &Output) -> String {
    assert!(output.stderr.is_empty(), "JSON refusals are stdout-only");
    let record: Value = serde_json::from_slice(&output.stdout).expect("typed refusal JSON");
    assert_eq!(record["schema"], "urn:animsmith:schema:producer-refusal:1");
    assert_eq!(record["command"], "assemble");
    assert_eq!(record["outcome"], "rejected");
    record["rejection"]["detail"]
        .as_str()
        .expect("refusal detail")
        .to_owned()
}

fn assert_schema(instance: &Value, schema: &str) {
    let schema: Value = serde_json::from_str(schema).expect("schema JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let errors = validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "schema errors: {errors:#?}");
}

fn expected_basis_fingerprint(path: &Path, tool: &Value) -> String {
    let bytes = std::fs::read(path).unwrap();
    let source = animsmith_gltf::preflight_scale_source_bytes(path, &bytes).unwrap();
    let operation = ScaleOperation::RestBindUniformScale {
        source_skin_index: 0,
        source_root_node_index: 0,
        expected_factor: 0.01,
    };
    let facts = animsmith_gltf::operation_capability_facts(source.manifest(), operation).unwrap();
    let plan = plan_scale(&ScaleRequest {
        operation,
        document: source.document(),
        capability: &facts,
    })
    .unwrap();
    let basis = assembly_scale_basis(source.document(), &plan).unwrap();
    #[derive(Serialize)]
    struct ToolSource<'a> {
        revision: &'a Value,
        dirty: &'a Value,
    }
    #[derive(Serialize)]
    struct Tool<'a> {
        name: &'a Value,
        version: &'a Value,
        source: ToolSource<'a>,
    }
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        schema: &'static str,
        tool: Tool<'a>,
        input_sha256: String,
        basis: &'a AssemblyScaleBasis,
    }
    let fingerprint = Fingerprint {
        schema: "urn:animsmith:character-assembly-scale-basis:1",
        tool: Tool {
            name: &tool["name"],
            version: &tool["version"],
            source: ToolSource {
                revision: &tool["source"]["revision"],
                dirty: &tool["source"]["dirty"],
            },
        },
        input_sha256: sha256_hex(&bytes),
        basis: &basis,
    };
    sha256_hex(&serde_json::to_vec(&fingerprint).unwrap())
}

#[test]
fn v4_rebases_before_remap_then_proves_and_publishes_the_exact_final_artifact() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
    write_cubic_asset(&dir.path().join("inputs/clip.glb"), 10.0);
    write_cubic_asset(&dir.path().join("inputs/clip-two.glb"), 20.0);
    let recipe = format!(
        "{}\n[[clips]]\nname = \"run\"\ninput = \"clip-two.glb\"\ntake = \"clip\"\n",
        recipe("clip.glb")
    );
    std::fs::write(dir.path().join("recipe.toml"), &recipe).unwrap();
    let recipe_value: toml::Value = toml::from_str(&recipe).unwrap();
    assert_schema(&serde_json::to_value(recipe_value).unwrap(), RECIPE_SCHEMA);

    let first = run(dir.path());
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence_bytes = std::fs::read(dir.path().join("character.json")).unwrap();
    assert_eq!(first.stdout, evidence_bytes);
    let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA);
    let evidence_schema: Value = serde_json::from_str(EVIDENCE_SCHEMA).unwrap();
    let evidence_validator = jsonschema::validator_for(&evidence_schema).unwrap();
    for pointer in ["/schema", "/rest_bind_scale/inputs/0/basis_schema"] {
        let mut wrong_identity = evidence.clone();
        *wrong_identity.pointer_mut(pointer).unwrap() = Value::String("urn:wrong:identity".into());
        assert!(
            !evidence_validator.is_valid(&wrong_identity),
            "schema admitted mismatched identity at {pointer}"
        );
    }
    assert_eq!(evidence["schema_version"], 4);
    let scale = &evidence["rest_bind_scale"];
    assert_eq!(scale["source_skin_index"], 0);
    assert_eq!(scale["source_root_node_index"], 0);
    assert_eq!(scale["expected_factor"], 0.01);
    let inputs = scale["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 3);
    for (input, (role, declared, expected_sha256)) in inputs.iter().zip([
        (
            "base",
            "base.glb",
            "18234c3fe594fee87189431e694d8489991c9afc7dbfaca23081edf7fba1349a",
        ),
        (
            "clip:walk",
            "clip.glb",
            "9482839267ec7013579b9e868cffaf23f2f8611863913e15031ecb17573029dd",
        ),
        (
            "clip:run",
            "clip-two.glb",
            "366adfecd29c38a7af16876a72052b1ae2ac0fe0f0b673809deb1b45f1ee487a",
        ),
    ]) {
        let bytes = std::fs::read(dir.path().join("inputs").join(declared)).unwrap();
        assert_eq!(input["role"], role);
        assert_eq!(input["declared_path"], declared);
        assert_eq!(input["bytes"], bytes.len());
        assert_eq!(bytes.len(), 2516);
        assert_eq!(input["sha256"], sha256_hex(&bytes));
        assert_eq!(input["sha256"], expected_sha256);
        assert_eq!(
            input["basis_schema"],
            "urn:animsmith:character-assembly-scale-basis:1"
        );
        assert_eq!(
            input["basis_fingerprint"],
            expected_basis_fingerprint(
                &dir.path().join("inputs").join(declared),
                &evidence["tool"]
            )
        );
        assert_eq!(input["compatible"], true);
        assert_eq!(input["compatibility"], "compatible");
    }
    assert_ne!(inputs[0]["sha256"], inputs[1]["sha256"]);
    assert_ne!(inputs[1]["sha256"], inputs[2]["sha256"]);
    let mut different_tool = evidence["tool"].clone();
    different_tool["version"] = Value::String("999.0.0".into());
    assert_ne!(
        inputs[0]["basis_fingerprint"],
        expected_basis_fingerprint(&dir.path().join("inputs/base.glb"), &different_tool),
        "tool identity is fingerprint material"
    );
    let digest_only_variant = dir.path().join("inputs/clip-digest-only.gltf");
    let mut semantically_equal = rest_bind_scale_rig_gltf();
    semantically_equal.extend_from_slice(b"\n");
    std::fs::write(&digest_only_variant, semantically_equal).unwrap();
    assert_ne!(
        inputs[1]["basis_fingerprint"],
        expected_basis_fingerprint(&digest_only_variant, &evidence["tool"]),
        "exact input digest is fingerprint material"
    );
    assert_eq!(evidence["artifact"]["sha256"], sha256_hex(&artifact));
    assert_eq!(
        evidence["artifact"]["sha256"],
        "60374d0f894fcc3f620ca204c52f6f7a2f0fbc61f1b5fc0b65cb9f48b7ca5755"
    );
    assert_eq!(evidence["artifact"]["bytes"], 3424);
    assert_eq!(
        scale["staged_source_sha256"],
        "90d4b8ff4b22e47acb5890dba9a2da5f24c618cfd4eb5ab837d813e9a4f7fe9c"
    );
    assert_eq!(scale["read_back_sha256"], evidence["artifact"]["sha256"]);
    assert_eq!(
        scale["proof"]["artifact"]["sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(scale["proof"]["artifact"]["bytes"], artifact.len());
    assert_eq!(scale["proof"]["proof"]["read_back_digest_matches"], true);
    assert!(
        scale["proof"]["proof"]["residuals"]
            .as_object()
            .is_some_and(|residuals| residuals
                .values()
                .all(|value| value.get("evaluated").is_some()))
    );
    assert_eq!(
        scale["residual_comparison_counts"],
        serde_json::json!({
            "bounds": 42,
            "cubic_interior": 2,
            "key_translation": 4,
            "mesh_position": 3,
            "rest_rotation": 3,
            "rest_translation": 3,
            "skin_matrix": 7,
            "track_value": 16,
            "trajectory": 18,
            "transform_only_affine": 1,
            "unaffected_inverse_bind": 0,
            "unit_scale": 3
        })
    );
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.clips.len(), 2);
    for (clip, offset) in document.clips.iter().zip([10.0, 20.0]) {
        let track = clip
            .tracks
            .iter()
            .find(|track| track.property == Property::Translation)
            .expect("emitted translation track");
        assert_eq!(track.interpolation, Interpolation::CubicSpline);
        let TrackValues::Vec3s(values) = &track.values else {
            panic!("translation values")
        };
        assert_eq!(values.len(), 6);
        let expected = [
            Vec3::new(offset + 2.0, 1.0, -1.0),
            Vec3::new(offset, 100.0, 2.0),
            Vec3::new(offset + 3.0, 2.0, -2.0),
            Vec3::new(offset + 4.0, 3.0, -3.0),
            Vec3::new(offset, 200.0, 4.0),
            Vec3::new(offset + 5.0, 4.0, -4.0),
        ];
        for (slot, expected) in values.iter().zip(expected) {
            assert!(slot.abs_diff_eq(expected * 0.01, 1.0e-6));
        }
    }

    let first_artifact = artifact;
    let first_evidence = evidence_bytes;
    let second = run(dir.path());
    assert!(second.status.success());
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        first_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        first_evidence
    );
}

#[test]
fn v4_rebases_every_cubic_slot_for_a_factor_greater_than_one() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let authored = factor_two_rig_glb();
    write_cubic_asset_from(&dir.path().join("inputs/base.glb"), &authored, 0.0);
    write_cubic_asset_from(&dir.path().join("inputs/clip.glb"), &authored, 10.0);
    std::fs::write(
        dir.path().join("recipe.toml"),
        recipe("clip.glb").replace("expected_factor = 0.01", "expected_factor = 2.0"),
    )
    .unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let track = document.clips[0]
        .tracks
        .iter()
        .find(|track| track.property == Property::Translation)
        .unwrap();
    assert_eq!(track.interpolation, Interpolation::CubicSpline);
    let TrackValues::Vec3s(values) = &track.values else {
        panic!("translation values")
    };
    let expected = [
        Vec3::new(12.0, 1.0, -1.0),
        Vec3::new(10.0, 100.0, 2.0),
        Vec3::new(13.0, 2.0, -2.0),
        Vec3::new(14.0, 3.0, -3.0),
        Vec3::new(10.0, 200.0, 4.0),
        Vec3::new(15.0, 4.0, -4.0),
    ];
    for (slot, expected) in values.iter().zip(expected) {
        assert!(slot.abs_diff_eq(expected * 2.0, 1.0e-5));
    }
}

#[test]
fn v4_strip_bone_motion_evidence_uses_the_rebased_clip_basis() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_scale_sensitive_clip_asset(&dir.path().join("inputs/base.glb"), 300.0);
    write_scale_sensitive_clip_asset(&dir.path().join("inputs/clip.glb"), 300.0);
    let recipe = recipe("clip.glb").replace(
        "take = \"clip\"\n",
        "take = \"clip\"\nstrip_bones = [\"joint\"]\n",
    );
    std::fs::write(dir.path().join("recipe.toml"), &recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("character.json")).unwrap()).unwrap();
    assert_eq!(evidence["clips"][0]["stripped_tracks"], 1);
    assert_eq!(
        evidence["clips"][0]["stripped_bone_motion"],
        serde_json::json!([{
            "bone": "joint",
            "translation_start": [0.0, 1.0, 0.0],
            "translation_end": [0.0, 3.0, 0.0],
            "translation_delta": [0.0, 2.0, 0.0],
            "duration_s": 1.0
        }])
    );
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.clips.len(), 1);
    assert_eq!(document.clips[0].tracks.len(), 1);
    assert_eq!(document.clips[0].tracks[0].bone, 0);
    assert_eq!(document.clips[0].tracks[0].property, Property::Rotation);
}

#[test]
fn v4_prunes_constant_tracks_after_rebasing_the_clip() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_scale_sensitive_clip_asset(&dir.path().join("inputs/base.glb"), 100.005);
    write_scale_sensitive_clip_asset(&dir.path().join("inputs/clip.glb"), 100.005);
    let recipe =
        recipe("clip.glb").replacen("fps = 30.0", "fps = 30.0\nprune_constant_tracks = true", 1);
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("character.json")).unwrap()).unwrap();
    assert_eq!(
        evidence["clips"][0]["pruned_constant_tracks"],
        serde_json::json!([{
            "original_track_index": 0,
            "bone": "joint",
            "bone_index": 1,
            "property": "translation",
            "interpolation": "linear",
            "key_count": 2
        }])
    );
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.clips.len(), 1);
    assert_eq!(document.clips[0].tracks.len(), 1);
    assert_eq!(document.clips[0].tracks[0].property, Property::Rotation);
}

#[test]
fn v4_rejects_an_orientation_basis_mismatch_atomically() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.glb"),
        rest_bind_scale_rig_glb(),
    )
    .unwrap();
    let mut clip: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    clip["nodes"][1]["rotation"] = serde_json::json!([0.0, 0.0, 0.001, 0.9999995]);
    std::fs::write(
        dir.path().join("inputs/clip.gltf"),
        serde_json::to_vec(&clip).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("recipe.toml"), recipe("clip.gltf")).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(refusal_detail(&output).contains("named-orientation"));
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v5_missing_source_selectors_refuse_before_replacing_a_prior_pair() {
    for (replacement, expected_detail) in [
        ("source_skin_index = 9", "skin"),
        ("source_root_node_index = 9", "root"),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
        write_cubic_asset(&dir.path().join("inputs/walk.glb"), 10.0);
        let recipe = if replacement.starts_with("source_skin") {
            recipe_v5("walk.glb").replace("source_skin_index = 0", replacement)
        } else {
            recipe_v5("walk.glb").replace("source_root_node_index = 0", replacement)
        };
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
        let prior_artifact = b"prior artifact";
        let prior_evidence = b"prior evidence";
        std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
        std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(1));
        assert!(refusal_detail(&output).contains(expected_detail));
        assert_eq!(
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            prior_artifact
        );
        assert_eq!(
            std::fs::read(dir.path().join("character.json")).unwrap(),
            prior_evidence
        );
    }
}

#[test]
fn v5_ambiguous_post_assembly_skin_selector_refuses_atomically() {
    // Two source skins with the same selected, named joint topology are
    // individually valid glTF. The composed producer must not silently pick
    // the first staged skin after canonicalization shifts raw source indices.
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let mut ambiguous: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    let second_skin = ambiguous["skins"][0].clone();
    ambiguous["skins"].as_array_mut().unwrap().push(second_skin);
    ambiguous["nodes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "holder2",
            "mesh": 0,
            "skin": 1
        }));
    ambiguous["scenes"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!(4));
    let bytes = serde_json::to_vec(&ambiguous).unwrap();
    std::fs::write(dir.path().join("inputs/base.gltf"), &bytes).unwrap();
    std::fs::write(dir.path().join("inputs/walk.gltf"), bytes).unwrap();
    let recipe = recipe_v5("walk.gltf").replace("base.glb", "base.gltf");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(
        refusal_detail(&output).contains("exactly one skin with the selected named joint topology"),
        "the refusal identifies the ambiguous selected skin topology: {}",
        refusal_detail(&output)
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v4_accepts_quaternion_sign_and_in_band_rest_spelling_differences() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.glb"),
        rest_bind_scale_rig_glb(),
    )
    .unwrap();
    let mut clip: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    clip["nodes"][1]["rotation"] = serde_json::json!([-0.0, -0.0, -0.0, -1.0]);
    clip["nodes"][1]["translation"] = serde_json::json!([0.0, 100.00001, 0.0]);
    std::fs::write(
        dir.path().join("inputs/clip.gltf"),
        serde_json::to_vec(&clip).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("recipe.toml"), recipe("clip.gltf")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("character.glb").exists());
    assert!(dir.path().join("character.json").exists());
}

#[test]
fn v4_rejects_an_unsupported_clip_before_any_remap_or_publication() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.glb"),
        rest_bind_scale_rig_glb(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("inputs/clip.gltf"),
        rest_bind_scale_rig_gltf(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("inputs/later.gltf"),
        rest_bind_scale_rig_gltf(),
    )
    .unwrap();
    let recipe = format!(
        "{}\n[[clips]]\nname = \"later\"\ninput = \"later.gltf\"\ntake = \"clip\"\n",
        recipe("clip.gltf")
    );
    std::fs::write(dir.path().join("recipe.toml"), &recipe).unwrap();
    let published = run(dir.path());
    assert!(published.status.success());
    let prior_artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let prior_evidence = std::fs::read(dir.path().join("character.json")).unwrap();

    let mut clip: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    clip["nodes"][2]["extras"] = serde_json::json!({ "private": true });
    std::fs::write(
        dir.path().join("inputs/later.gltf"),
        serde_json::to_vec(&clip).unwrap(),
    )
    .unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(refusal_detail(&output).contains("preflight rejected input later.gltf"));
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v4_rejects_an_unsupported_base_before_publication() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let mut base: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    base["nodes"][2]["extras"] = serde_json::json!({ "private": true });
    std::fs::write(
        dir.path().join("inputs/base.glb"),
        serde_json::to_vec(&base).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("inputs/clip.glb"),
        rest_bind_scale_rig_glb(),
    )
    .unwrap();
    std::fs::write(dir.path().join("recipe.toml"), recipe("clip.glb")).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(refusal_detail(&output).contains("preflight rejected input base.glb"));
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v4_active_block_rejects_fbx_instead_of_claiming_complete_coverage() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    std::fs::write(
        dir.path().join("inputs/clip.glb"),
        rest_bind_scale_rig_glb(),
    )
    .unwrap();
    let recipe = recipe("clip.glb").replace("base.glb", "base.fbx");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("rest_bind_scale input base.fbx is not glTF/GLB"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v5_active_block_keeps_its_fbx_refusal_and_preserves_a_prior_pair() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    let recipe = fbx_recipe_v6("walk.fbx")
        .replacen("schema_version = 6", "schema_version = 5", 1)
        .replacen(
            "urn:animsmith:schema:character-assembly-recipe:6",
            "urn:animsmith:schema:character-assembly-recipe:5",
            1,
        );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("rest_bind_scale input base.fbx is not glTF/GLB")
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v5_active_block_refuses_an_fbx_clip_with_a_gltf_base() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.glb"),
        rest_bind_scale_rig_glb(),
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    let recipe = recipe_v5("walk.fbx");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("rest_bind_scale input walk.fbx is not glTF/GLB")
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v6_assembles_eligible_fbx_base_and_clip_through_one_proved_glb() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    let clip_fbx = translated_fbx_clip();
    std::fs::write(dir.path().join("inputs/walk.fbx"), &clip_fbx).unwrap();
    let recipe = fbx_recipe_v6("walk.fbx");
    let recipe_value: toml::Value = toml::from_str(&recipe).unwrap();
    assert_schema(
        &serde_json::to_value(recipe_value).unwrap(),
        RECIPE_SCHEMA_V6,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let first = run(dir.path());
    assert!(
        first.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stderr.is_empty());
    let artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence_bytes = std::fs::read(dir.path().join("character.json")).unwrap();
    assert_eq!(first.stdout, evidence_bytes);
    let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V6);
    assert_eq!(evidence["schema_version"], 6);
    assert_eq!(
        evidence["schema"],
        "urn:animsmith:schema:character-assembly-evidence:6"
    );
    assert_eq!(evidence["artifact"]["sha256"], sha256_hex(&artifact));
    let scale = &evidence["rest_bind_scale"];
    assert!(scale.get("declared_root_node_name").is_none());
    let expected_stages = [
        normalized_fbx_stage_bytes(dir.path(), "base", RIGGED_TRIANGLE_FBX.as_bytes()),
        normalized_fbx_stage_bytes(dir.path(), "clip", clip_fbx.as_bytes()),
    ];
    assert_ne!(
        sha256_hex(&expected_stages[0]),
        sha256_hex(&expected_stages[1])
    );
    assert_eq!(scale["read_back_sha256"], evidence["artifact"]["sha256"]);
    assert_eq!(scale["proof"]["proof"]["read_back_digest_matches"], true);
    assert_eq!(scale["inputs"].as_array().unwrap().len(), 2);
    for (input, role, declared, expected_stage) in [
        (&scale["inputs"][0], "base", "base.fbx", &expected_stages[0]),
        (
            &scale["inputs"][1],
            "clip:walk",
            "walk.fbx",
            &expected_stages[1],
        ),
    ] {
        assert_eq!(input["role"], role);
        assert_eq!(input["declared_path"], declared);
        assert_eq!(input["input_format"], "fbx");
        assert!(input.get("resolved_root_node_name").is_none());
        assert!(input.get("resolved_source_skin_index").is_none());
        assert!(input.get("resolved_source_root_node_index").is_none());
        let projection = &input["source_projection"];
        assert_eq!(projection["kind"], "normalized-baked-fbx");
        assert_eq!(projection["authored_curve_keys_preserved"], false);
        assert_eq!(projection["raw_source_spans_preserved"], false);
        assert_eq!(projection["capability"]["animation_takes_baked"], true);
        assert_eq!(
            projection["capability"]["authored_curve_keys_preserved"],
            false
        );
        assert_eq!(
            projection["capability"]["domains"]["translation_animation"],
            "baked"
        );
        assert_eq!(
            projection["staged_source"]["sha256"],
            sha256_hex(expected_stage)
        );
        assert_eq!(
            projection["staged_source"]["bytes"],
            u64::try_from(expected_stage.len()).unwrap()
        );
    }
    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(assembled.clips.len(), 1);
    assert_eq!(assembled.clips[0].name, "walk");
    assert!(
        assembled
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.rest.scale == Vec3::ONE),
        "the published rest hierarchy is fully reparameterized"
    );

    let second = run(dir.path());
    assert!(second.status.success());
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        artifact
    );
    assert_eq!(second.stdout, evidence_bytes);
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        evidence_bytes
    );
}

#[test]
fn v7_resolves_each_fbx_input_by_name_and_records_deterministic_selectors() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    let recipe = fbx_recipe_v7("walk.fbx");
    let recipe_value: toml::Value = toml::from_str(&recipe).unwrap();
    assert_schema(
        &serde_json::to_value(recipe_value).unwrap(),
        RECIPE_SCHEMA_V7,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let first = run(dir.path());
    assert!(
        first.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let first_evidence = std::fs::read(dir.path().join("character.json")).unwrap();
    assert_eq!(first.stdout, first_evidence);
    let evidence: Value = serde_json::from_slice(&first_evidence).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(evidence["schema_version"], 7);
    assert_eq!(
        evidence["schema"],
        "urn:animsmith:schema:character-assembly-evidence:7"
    );
    let scale = &evidence["rest_bind_scale"];
    assert_eq!(scale["declared_root_node_name"], "root");
    assert!(scale.get("source_skin_index").is_none());
    assert!(scale.get("source_root_node_index").is_none());
    for (index, input) in scale["inputs"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            input["application"],
            if index == 0 {
                "rest-bind"
            } else {
                "skinless-clip-tracks"
            }
        );
        assert_eq!(input["resolved_root_node_name"], "root");
        if index == 0 {
            assert_eq!(input["resolved_source_skin_index"], 0);
        } else {
            assert!(input.get("resolved_source_skin_index").is_none());
        }
        assert_eq!(input["resolved_source_root_node_index"], 1);
        assert_eq!(input["input_format"], "fbx");
    }
    assert_eq!(scale["read_back_sha256"], evidence["artifact"]["sha256"]);
    assert_eq!(
        scale["proof"]["artifact"]["sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(scale["proof"]["proof"]["read_back_digest_matches"], true);
    assert!(
        scale["residual_comparison_counts"]
            .as_object()
            .unwrap()
            .values()
            .filter_map(Value::as_u64)
            .sum::<u64>()
            > 0
    );

    let second = run(dir.path());
    assert!(second.status.success());
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        first_artifact
    );
    assert_eq!(second.stdout, first_evidence);
}

#[test]
fn v7_rebases_a_meshless_skinless_fbx_clip_from_the_skinned_base_plan() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let base_source = rigged_limb_triangle_fbx();
    let loaded_base =
        animsmith_fbx::load_bytes(Path::new("base.fbx"), base_source.as_bytes()).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), base_source).unwrap();
    let clip = skinless_animation_fbx();
    let loaded = animsmith_fbx::load_bytes(Path::new("walk.fbx"), clip.as_bytes()).unwrap();
    assert!(loaded.assets.source_skeleton.skins.is_empty());
    assert!(loaded.assets.instances.is_empty());
    assert_eq!(
        loaded_base
            .skeleton
            .bones
            .iter()
            .map(|bone| (&bone.name, bone.parent))
            .collect::<Vec<_>>(),
        loaded
            .skeleton
            .bones
            .iter()
            .map(|bone| (&bone.name, bone.parent))
            .collect::<Vec<_>>()
    );
    std::fs::write(dir.path().join("inputs/walk.fbx"), clip).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let inputs = evidence["rest_bind_scale"]["inputs"].as_array().unwrap();
    assert_eq!(inputs[0]["application"], "rest-bind");
    assert_eq!(inputs[0]["resolved_source_skin_index"], 0);
    assert_eq!(inputs[1]["application"], "skinless-clip-tracks");
    assert_eq!(
        inputs[1]["basis_schema"],
        "urn:animsmith:character-assembly-skinless-clip-scale-basis:1"
    );
    assert!(inputs[1].get("resolved_source_skin_index").is_none());
    assert_eq!(inputs[1]["resolved_root_node_name"], "root");
    assert_eq!(inputs[1]["resolved_source_root_node_index"], 1);

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let track = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| {
            assembled.skeleton.bones[track.bone].name == "tri"
                && track.property == Property::Translation
        })
        .expect("skinless clip translation survives assembly");
    assert_eq!(track.key_vec3(0), Some(Vec3::ZERO));
    assert_eq!(
        track.key_vec3(track.key_count() - 1),
        Some(Vec3::new(1.0, 0.0, 0.0))
    );
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );
}

#[test]
fn v7_projects_completion_targets_through_each_document_bone_namespace() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = document_local_bone_order_fbx();
    assert_document_local_bone_id_alias(dir.path(), &source);
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    write_document_local_order_clip(
        &dir.path().join("inputs/walk.glb"),
        &source,
        &["retained-second"],
        None,
    );
    let recipe = fbx_recipe_v7("walk.glb").replacen(
        "fps = 30.0",
        concat!(
            "fps = 30.0\n",
            "complete_tracks = true\n",
            "remove_nodes = [\"remove\"]",
        ),
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(evidence["clips"][0]["completed_tracks"], 5);
    assert_eq!(
        evidence["transforms"]["removed_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["remove"]
    );
    assert_eq!(
        evidence["rest_bind_scale"]["read_back_sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert!(
        assembled
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.name != "remove")
    );
    assert!(
        assembled.clips[0]
            .tracks
            .iter()
            .all(|track| { assembled.skeleton.bones[track.bone].name != "remove" })
    );
    let retained_second = unique_bone_id(&assembled, "retained-second");
    assert_eq!(
        assembled.clips[0]
            .tracks
            .iter()
            .filter(|track| track.bone == retained_second)
            .map(|track| track.property.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["rotation", "scale", "translation"])
    );
}

#[test]
fn v7_projects_authoritative_pruning_by_named_clip_channel_identity() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = document_local_bone_order_fbx();
    assert_document_local_bone_id_alias(dir.path(), &source);
    let authoritative =
        animsmith_fbx::load_bytes(Path::new("base.fbx"), source.as_bytes()).unwrap();
    let staged_retained_second = unique_bone_id(&authoritative, "retained-second");
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    write_document_local_order_clip(
        &dir.path().join("inputs/walk.glb"),
        &source,
        &["retained-first", "retained-second"],
        Some("retained-second"),
    );
    let recipe = fbx_recipe_v7("walk.glb")
        .replacen(
            "fps = 30.0",
            concat!(
                "fps = 30.0\n",
                "complete_tracks = true\n",
                "prune_constant_tracks = true\n",
                "remove_nodes = [\"remove\"]",
            ),
            1,
        )
        .replacen(
            "take = \"take\"",
            "take = \"take\"\nstrip_bones = [\"remove\"]",
            1,
        );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    std::fs::write(
        dir.path().join("animsmith.toml"),
        concat!("[clips.walk]\n", "animates_bones = [\"retained-first\"]\n",),
    )
    .unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let pruned = evidence["clips"][0]["pruned_constant_tracks"]
        .as_array()
        .unwrap();
    assert_eq!(pruned.len(), 2, "evidence: {evidence:#}");
    assert!(
        pruned
            .iter()
            .all(|track| track["bone"] == "retained-second")
    );
    assert!(
        pruned
            .iter()
            .all(|track| track["bone_index"] == staged_retained_second as u64)
    );
    assert_eq!(
        pruned
            .iter()
            .map(|track| track["original_track_index"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(
        pruned
            .iter()
            .map(|track| track["property"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["rotation", "scale"])
    );
    assert_eq!(
        evidence["rest_bind_scale"]["read_back_sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let retained_first = unique_bone_id(&assembled, "retained-first");
    let retained_second = unique_bone_id(&assembled, "retained-second");
    let properties = |bone| {
        assembled.clips[0]
            .tracks
            .iter()
            .filter(|track| track.bone == bone)
            .map(|track| track.property.as_str())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        properties(retained_first),
        BTreeSet::from(["rotation", "scale", "translation"])
    );
    assert_eq!(properties(retained_second), BTreeSet::from(["translation"]));
}

#[test]
fn v7_still_refuses_an_authored_track_into_the_removed_reference_closure() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = document_local_bone_order_fbx();
    assert_document_local_bone_id_alias(dir.path(), &source);
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    write_document_local_order_clip(
        &dir.path().join("inputs/walk.glb"),
        &source,
        &["remove"],
        None,
    );
    let recipe = fbx_recipe_v7("walk.glb").replacen(
        "fps = 30.0",
        concat!(
            "fps = 30.0\n",
            "complete_tracks = true\n",
            "remove_nodes = [\"remove\"]",
        ),
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    let refusal: Value = serde_json::from_slice(&output.stdout).expect("typed refusal JSON");
    assert_eq!(refusal["rejection"]["stage"], "transform");
    assert_eq!(refusal["rejection"]["kind"], "transform-refused");
    let detail = refusal_detail(&output);
    assert!(detail.contains("cannot remove selected reference nodes"));
    assert!(detail.contains("still targets selected node"));
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v7_accepts_skinless_clip_without_base_only_geometry_descendants() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let base_source = rigged_limb_triangle_with_unskinned_prop_fbx();
    let loaded_base =
        animsmith_fbx::load_bytes(Path::new("base.fbx"), base_source.as_bytes()).unwrap();
    assert_eq!(loaded_base.assets.instances.len(), 2);
    assert!(
        loaded_base
            .skeleton
            .bones
            .iter()
            .any(|bone| bone.name == "prop")
    );
    std::fs::write(dir.path().join("inputs/base.fbx"), base_source).unwrap();

    let clip_source = skinless_animation_fbx();
    let loaded_clip =
        animsmith_fbx::load_bytes(Path::new("walk.fbx"), clip_source.as_bytes()).unwrap();
    assert!(loaded_clip.assets.instances.is_empty());
    assert!(
        loaded_clip
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.name != "prop")
    );
    std::fs::write(dir.path().join("inputs/walk.fbx"), clip_source).unwrap();
    let recipe =
        fbx_recipe_v7("walk.fbx").replace("fps = 30.0", "fps = 30.0\nremove_nodes = [\"prop\"]");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(
        evidence["rest_bind_scale"]["inputs"][1]["application"],
        "skinless-clip-tracks"
    );
    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert!(
        assembled
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.name != "prop")
    );
    assert_eq!(assembled.clips.len(), 1);
    let translation = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| {
            assembled.skeleton.bones[track.bone].name == "tri"
                && track.property == Property::Translation
        })
        .expect("skinless clip translation survives assembly");
    assert_eq!(
        translation.key_vec3(translation.key_count() - 1),
        Some(Vec3::new(1.0, 0.0, 0.0))
    );
}

#[test]
fn v7_accepts_fully_keyed_skinless_clip_with_take_pose_rest() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    write_skinless_take_pose_clip(&dir.path().join("inputs/walk.glb"));
    let base = animsmith_fbx::load(&dir.path().join("inputs/base.fbx")).unwrap();
    let clip = animsmith_gltf::load(&dir.path().join("inputs/walk.glb")).unwrap();
    let base_limb = base
        .skeleton
        .bones
        .iter()
        .find(|bone| bone.name == "tri")
        .unwrap();
    let clip_limb = clip
        .skeleton
        .bones
        .iter()
        .find(|bone| bone.name == "tri")
        .unwrap();
    assert_ne!(base_limb.rest.translation, clip_limb.rest.translation);
    assert_ne!(base_limb.rest.rotation, clip_limb.rest.rotation);
    assert_ne!(base_limb.rest.scale, clip_limb.rest.scale);
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.glb")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(
        evidence["rest_bind_scale"]["inputs"][1]["application"],
        "skinless-clip-tracks"
    );

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let limb = assembled
        .skeleton
        .bones
        .iter()
        .position(|bone| bone.name == "tri")
        .expect("assembled fixture retains the animated limb");
    assert_eq!(
        assembled.skeleton.bones[limb].rest,
        Transform {
            translation: base_limb.rest.translation * 0.01,
            rotation: base_limb.rest.rotation,
            scale: base_limb.rest.scale,
        },
        "the clip take-pose rest must not replace the proved base rest"
    );
    let tracks = assembled.clips[0]
        .tracks
        .iter()
        .filter(|track| track.bone == limb)
        .map(|track| (track.property, track))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        tracks[&Property::Translation].key_vec3(0),
        Some(Vec3::new(2.5, 0.0, 0.0))
    );
    assert_eq!(
        tracks[&Property::Translation].key_vec3(1),
        Some(Vec3::new(3.5, 0.0, 0.0))
    );
    assert!(
        tracks[&Property::Rotation]
            .key_quat(0)
            .is_some_and(|value| value.dot(Quat::from_rotation_z(0.25)).abs() > 1.0 - 1.0e-6)
    );
    assert!(
        tracks[&Property::Rotation]
            .key_quat(1)
            .is_some_and(|value| value.dot(Quat::from_rotation_z(0.5)).abs() > 1.0 - 1.0e-6)
    );
    assert_eq!(tracks[&Property::Scale].key_vec3(0), Some(Vec3::splat(2.0)));
    assert_eq!(tracks[&Property::Scale].key_vec3(1), Some(Vec3::splat(3.0)));
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );
}

#[test]
fn v7_derives_skinless_translation_factors_from_the_selected_base_plan() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        two_centimetre_unit_fbx(&rigged_limb_triangle_fbx()),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("inputs/walk.fbx"),
        two_centimetre_unit_fbx(&skinless_animation_fbx()),
    )
    .unwrap();
    let recipe =
        fbx_recipe_v7("walk.fbx").replace("expected_factor = 0.01", "expected_factor = 0.02");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let translation = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| track.property == Property::Translation)
        .expect("translation track survives");
    assert_eq!(
        translation.key_vec3(translation.key_count() - 1),
        Some(Vec3::new(2.0, 0.0, 0.0))
    );
}

#[test]
fn v7_rebases_every_cubic_translation_value_and_tangent_in_a_skinless_clip() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    write_skinless_cubic_clip(&dir.path().join("inputs/walk.glb"));
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.glb")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let track = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| track.property == Property::Translation)
        .expect("translation track survives");
    assert_eq!(track.interpolation, Interpolation::CubicSpline);
    let TrackValues::Vec3s(values) = &track.values else {
        panic!("translation track must retain Vec3 storage")
    };
    assert_eq!(
        values,
        &[
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::new(0.2, 0.0, 0.0),
            Vec3::new(0.3, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.4, 0.0, 0.0),
        ]
    );
    let scale = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| track.property == Property::Scale)
        .expect("scale track survives");
    assert_eq!(scale.key_vec3(0), Some(Vec3::splat(200.0)));
    assert_eq!(scale.key_vec3(1), Some(Vec3::splat(300.0)));
    let rotation = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| track.property == Property::Rotation)
        .expect("rotation track survives");
    assert_eq!(rotation.key_quat(0), Some(Quat::IDENTITY));
    assert!(
        rotation
            .key_quat(1)
            .is_some_and(|value| value.dot(Quat::from_rotation_z(0.5)).abs() > 1.0 - 1.0e-6)
    );
}

#[test]
fn v7_projects_unused_dual_quaternion_clip_geometry_before_scale_preparation() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    let clip = dual_quaternion_material_animation_fbx();
    std::fs::write(dir.path().join("inputs/normal.png"), TINY_PNG).unwrap();
    let loaded = animsmith_fbx::load_scale_source_bytes_with_resource_root(
        Path::new("walk.fbx"),
        clip.as_bytes(),
        &dir.path().join("inputs"),
    )
    .expect("dual-quaternion analytic fixture loads");
    assert_eq!(loaded.inventory().dual_quaternion_skin_count, 1);
    assert_eq!(loaded.document().assets.source_skeleton.skins.len(), 1);
    assert_eq!(loaded.document().assets.instances.len(), 1);
    assert_eq!(loaded.document().assets.materials.len(), 1);
    let expected_clip_sha256 = sha256_hex(clip.as_bytes());
    let expected_clip_bytes = u64::try_from(clip.len()).unwrap();
    let expected_capability = serde_json::to_value(loaded.inventory()).unwrap();

    let baseline_dir = tempfile::tempdir().expect("baseline temporary directory");
    std::fs::create_dir(baseline_dir.path().join("inputs")).unwrap();
    std::fs::write(
        baseline_dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    let mut projected_clip = loaded.document().clone();
    projected_clip.assets.meshes.clear();
    projected_clip.assets.instances.clear();
    projected_clip.assets.materials.clear();
    projected_clip.assets.material_resources = Default::default();
    projected_clip.assets.source_skeleton.skins.clear();
    for bone in &mut projected_clip.skeleton.bones {
        bone.inverse_bind = None;
    }
    animsmith_gltf::write::write(
        &projected_clip,
        &baseline_dir.path().join("inputs/walk.glb"),
    )
    .unwrap();
    let expected_staged_clip = std::fs::read(baseline_dir.path().join("inputs/walk.glb")).unwrap();
    let expected_staged_sha256 = sha256_hex(&expected_staged_clip);
    let expected_staged_bytes = u64::try_from(expected_staged_clip.len()).unwrap();
    std::fs::write(
        baseline_dir.path().join("recipe.toml"),
        fbx_recipe_v7("walk.glb"),
    )
    .unwrap();
    let baseline_output = run(baseline_dir.path());
    assert!(baseline_output.status.success());
    let baseline_artifact = std::fs::read(baseline_dir.path().join("character.glb")).unwrap();

    std::fs::write(dir.path().join("inputs/walk.fbx"), clip).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let clip_input = &evidence["rest_bind_scale"]["inputs"][1];
    assert_eq!(clip_input["application"], "skinless-clip-tracks");
    assert_eq!(
        (
            clip_input["sha256"].as_str(),
            clip_input["bytes"].as_u64(),
            &clip_input["source_projection"]["capability"],
        ),
        (
            Some(expected_clip_sha256.as_str()),
            Some(expected_clip_bytes),
            &expected_capability,
        )
    );
    assert_eq!(
        (
            clip_input["source_projection"]["staged_source"]["sha256"].as_str(),
            clip_input["source_projection"]["staged_source"]["bytes"].as_u64(),
        ),
        (
            Some(expected_staged_sha256.as_str()),
            Some(expected_staged_bytes),
        ),
        "the recorded private stage must be the independently constructed animation-only projection"
    );
    assert!(clip_input.get("resolved_source_skin_index").is_none());

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        baseline_artifact,
        "discarding unused FBX domains must match an explicit animation-only source"
    );
    let output_positions =
        &assembled.assets.meshes[assembled.assets.instances[0].mesh].primitives[0].positions;
    assert!(
        output_positions
            .iter()
            .all(|position| position.x.abs() <= 100.0 && position.y.abs() <= 100.0),
        "the clip-only nine-metre geometry must not replace the base mesh"
    );
    assert_eq!(
        (
            assembled.assets.instances.len(),
            assembled.assets.source_skeleton.skins.len(),
            assembled.assets.materials.len(),
        ),
        (1, 1, 0),
        "only base-owned geometry, skin, and materials reach the output"
    );
    assert_eq!(assembled.clips.len(), 1);
    let translation = assembled.clips[0]
        .tracks
        .iter()
        .find(|track| {
            assembled.skeleton.bones[track.bone].name == "root"
                && track.property == Property::Translation
        })
        .expect("clip translation survives the animation-only projection");
    assert_eq!(
        translation.key_vec3(translation.key_count() - 1),
        Some(Vec3::new(1.0, 0.0, 0.0))
    );
}

#[test]
fn v7_reused_canonical_base_path_keeps_the_base_rest_bind_projection() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("./base.fbx")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let inputs = evidence["rest_bind_scale"]["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0]["application"], "rest-bind");
    assert_eq!(inputs[1]["application"], "rest-bind");
    assert_eq!(inputs[0]["sha256"], inputs[1]["sha256"]);
    assert_eq!(inputs[1]["declared_path"], "./base.fbx");
    assert!(inputs[1]["resolved_source_skin_index"].is_u64());
}

#[test]
fn v7_reused_canonical_clip_path_reuses_the_first_clip_tracks_projection() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), skinless_animation_fbx()).unwrap();
    std::fs::write(
        dir.path().join("recipe.toml"),
        format!(
            "{}\n[[clips]]\nname = \"run\"\ninput = \"./walk.fbx\"\ntake = \"take\"\n",
            fbx_recipe_v7("walk.fbx")
        ),
    )
    .unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let inputs = evidence["rest_bind_scale"]["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[1]["application"], "skinless-clip-tracks");
    assert_eq!(inputs[2]["application"], "skinless-clip-tracks");
    assert_eq!(inputs[1]["sha256"], inputs[2]["sha256"]);
    assert_eq!(inputs[1]["bytes"], inputs[2]["bytes"]);
    assert_eq!(
        inputs[1]["basis_fingerprint"],
        inputs[2]["basis_fingerprint"]
    );
    assert_eq!(
        inputs[1]["source_projection"],
        inputs[2]["source_projection"]
    );
    assert_eq!(inputs[2]["declared_path"], "./walk.fbx");
    assert!(inputs[1].get("resolved_source_skin_index").is_none());
    assert!(inputs[2].get("resolved_source_skin_index").is_none());

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(
        assembled
            .clips
            .iter()
            .map(|clip| clip.name.as_str())
            .collect::<Vec<_>>(),
        ["walk", "run"],
        "reusing one prepared source must retain both declared logical clips"
    );
}

#[test]
fn v7_still_refuses_dual_quaternion_geometry_on_the_base_atomically() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        dual_quaternion_animation_fbx(),
    )
    .unwrap();
    let clip = skinless_animation_fbx();
    let loaded = animsmith_fbx::load_bytes(Path::new("walk.fbx"), clip.as_bytes()).unwrap();
    assert!(loaded.assets.source_skeleton.skins.is_empty());
    assert!(loaded.assets.instances.is_empty());
    std::fs::write(dir.path().join("inputs/walk.fbx"), clip).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(refusal_detail(&output).contains("dual_quaternion_skin_count=1"));
    assert_eq!(
        (
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            std::fs::read(dir.path().join("character.json")).unwrap(),
        ),
        (prior_artifact.to_vec(), prior_evidence.to_vec())
    );
}

#[test]
fn v7_projects_gltf_clip_geometry_and_missing_inverse_binds() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let base = rest_bind_scale_rig_gltf();
    std::fs::write(dir.path().join("inputs/base.gltf"), &base).unwrap();
    let mut clip: Value = serde_json::from_slice(&base).unwrap();
    clip["skins"][0]
        .as_object_mut()
        .unwrap()
        .remove("inverseBindMatrices");
    let clip = serde_json::to_vec(&clip).unwrap();
    let expected_clip_sha256 = sha256_hex(&clip);
    std::fs::write(dir.path().join("inputs/walk.gltf"), &clip).unwrap();
    let recipe = fbx_recipe_v7("walk.gltf")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.gltf\"")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let clip_input = &evidence["rest_bind_scale"]["inputs"][1];
    assert_eq!(clip_input["application"], "skinless-clip-tracks");
    assert_eq!(clip_input["sha256"], expected_clip_sha256);
    assert_eq!(clip_input["bytes"], clip.len());
    assert_eq!(clip_input["source_projection"]["kind"], "raw-gltf");
    assert!(clip_input.get("resolved_source_skin_index").is_none());

    let source_base = animsmith_gltf::load_bytes(Path::new("base.gltf"), &base).unwrap();
    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(assembled.clips.len(), 1);
    assert_eq!(
        (
            assembled.assets.instances.len(),
            assembled.assets.source_skeleton.skins.len(),
            assembled.assets.materials.len(),
        ),
        (
            source_base.assets.instances.len(),
            source_base.assets.source_skeleton.skins.len(),
            source_base.assets.materials.len(),
        ),
        "projected clip geometry, skin, bind, and material domains must not reach the output"
    );
}

#[test]
fn v7_projects_skinless_gltf_clip_geometry() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();

    let mut clip =
        animsmith_fbx::load_bytes(Path::new("walk.fbx"), rigged_limb_triangle_fbx().as_bytes())
            .expect("analytic FBX fixture loads");
    clip.assets.source_skeleton.skins.clear();
    for instance in &mut clip.assets.instances {
        instance.skin_joints.clear();
        instance.skin_ibms.clear();
    }
    for bone in &mut clip.skeleton.bones {
        bone.inverse_bind = None;
    }
    animsmith_gltf::write::write(&clip, &dir.path().join("inputs/walk.glb")).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.glb")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        evidence["rest_bind_scale"]["inputs"][1]["application"],
        "skinless-clip-tracks"
    );
    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(
        (
            assembled.assets.instances.len(),
            assembled.assets.source_skeleton.skins.len(),
            assembled.clips.len(),
        ),
        (1, 1, 1),
        "only the base contributes geometry and skin while the clip contributes tracks"
    );
}

#[test]
fn v7_projects_non_triangle_clip_geometry_but_keeps_the_base_strict() {
    let valid = rest_bind_scale_rig_gltf();
    let mut unsupported: Value = serde_json::from_slice(&valid).unwrap();
    unsupported["meshes"][0]["primitives"][0]["mode"] = serde_json::json!(1);
    let unsupported = serde_json::to_vec(&unsupported).unwrap();

    let clip_dir = tempfile::tempdir().expect("clip temporary directory");
    std::fs::create_dir(clip_dir.path().join("inputs")).unwrap();
    std::fs::write(clip_dir.path().join("inputs/base.gltf"), &valid).unwrap();
    std::fs::write(clip_dir.path().join("inputs/walk.gltf"), &unsupported).unwrap();
    let recipe = fbx_recipe_v7("walk.gltf")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.gltf\"")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(clip_dir.path().join("recipe.toml"), &recipe).unwrap();
    let clip_output = run(clip_dir.path());
    assert!(
        clip_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&clip_output.stdout),
        String::from_utf8_lossy(&clip_output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&clip_output.stdout).unwrap();
    assert_eq!(
        evidence["rest_bind_scale"]["inputs"][1]["application"],
        "skinless-clip-tracks"
    );

    let base_dir = tempfile::tempdir().expect("base temporary directory");
    std::fs::create_dir(base_dir.path().join("inputs")).unwrap();
    std::fs::write(base_dir.path().join("inputs/base.gltf"), unsupported).unwrap();
    std::fs::write(base_dir.path().join("inputs/walk.gltf"), valid).unwrap();
    std::fs::write(base_dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(base_dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(base_dir.path().join("character.json"), prior_evidence).unwrap();

    let base_output = run(base_dir.path());
    assert_eq!(base_output.status.code(), Some(1));
    assert!(refusal_detail(&base_output).contains("preflight rejected input base.gltf"));
    assert_eq!(
        std::fs::read(base_dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(base_dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v7_keeps_missing_inverse_binds_strict_on_the_base_atomically() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let valid = rest_bind_scale_rig_gltf();
    let mut base: Value = serde_json::from_slice(&valid).unwrap();
    base["skins"][0]
        .as_object_mut()
        .unwrap()
        .remove("inverseBindMatrices");
    std::fs::write(
        dir.path().join("inputs/base.gltf"),
        serde_json::to_vec(&base).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/walk.gltf"), valid).unwrap();
    let recipe = fbx_recipe_v7("walk.gltf")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.gltf\"")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(
        refusal_detail(&output).contains("rest_bind_scale preflight rejected input base.gltf"),
        "{}",
        refusal_detail(&output)
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v7_refuses_skinless_tracks_outside_the_base_scale_domain_atomically() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let base_path = dir.path().join("inputs/base.glb");
    std::fs::write(&base_path, unaffected_bind_scale_rig_glb()).unwrap();
    write_skinless_outside_domain_clip(&base_path, &dir.path().join("inputs/walk.glb"));
    let recipe = fbx_recipe_v7("walk.glb")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.glb\"")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(refusal_detail(&output).contains("animation-target-outside-scale-domain"));
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v7_refuses_skinless_root_and_basis_ambiguity_atomically() {
    for (ordinal, mutation, expected) in [
        (0, "missing-root", "source-root-name-not-unique"),
        (1, "ambiguous-root", "source-root-name-not-unique"),
        (2, "topology", "named-topology"),
        (3, "rest", "named-rest-basis"),
        (4, "orientation", "named-orientation"),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(
            dir.path().join("inputs/base.fbx"),
            rigged_limb_triangle_fbx(),
        )
        .unwrap();
        let mut clip =
            animsmith_fbx::load_bytes(Path::new("walk.fbx"), skinless_animation_fbx().as_bytes())
                .expect("skinless fixture loads");
        let root = clip
            .skeleton
            .bones
            .iter()
            .position(|bone| bone.name == "root")
            .unwrap();
        let limb = clip
            .skeleton
            .bones
            .iter()
            .position(|bone| bone.name == "tri")
            .unwrap();
        match mutation {
            "missing-root" => clip.skeleton.bones[root].name = "other-root".into(),
            "ambiguous-root" => clip.skeleton.bones[limb].name = "root".into(),
            "topology" => clip.skeleton.bones[limb].parent = None,
            "rest" => {
                clip.skeleton.bones[root].rest.translation.x += 100.0;
                let source_node = clip
                    .assets
                    .source_skeleton
                    .nodes
                    .iter_mut()
                    .find(|node| node.bone == Some(root))
                    .unwrap();
                let SourceNodeLocalRest::Trs { translation, .. } = &mut source_node.local_rest
                else {
                    panic!("fixture root uses TRS rest")
                };
                translation.x += 100.0;
            }
            "orientation" => {
                let rotation = Quat::from_rotation_z(0.5);
                clip.skeleton.bones[root].rest.rotation = rotation;
                let source_node = clip
                    .assets
                    .source_skeleton
                    .nodes
                    .iter_mut()
                    .find(|node| node.bone == Some(root))
                    .unwrap();
                let SourceNodeLocalRest::Trs {
                    rotation: source_rotation,
                    ..
                } = &mut source_node.local_rest
                else {
                    panic!("fixture root uses TRS rest")
                };
                *source_rotation = rotation;
            }
            _ => unreachable!(),
        }
        animsmith_gltf::write::write(&clip, &dir.path().join("inputs/walk.glb")).unwrap();
        std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.glb")).unwrap();
        let prior_artifact = b"prior artifact";
        let prior_evidence = b"prior evidence";
        std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
        std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(1), "case {ordinal}");
        assert!(
            refusal_detail(&output).contains(expected),
            "case {ordinal}: {}",
            refusal_detail(&output)
        );
        assert_eq!(
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            prior_artifact
        );
        assert_eq!(
            std::fs::read(dir.path().join("character.json")).unwrap(),
            prior_evidence
        );
    }
}

#[test]
fn v7_keeps_refusing_a_skinless_base() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), skinless_animation_fbx()).unwrap();
    std::fs::write(
        dir.path().join("inputs/walk.fbx"),
        rigged_limb_triangle_fbx(),
    )
    .unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(
        refusal_detail(&output)
            .contains("root_node_name \"root\" fully governs 0 source skins; expected exactly one")
    );
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v7_composes_fbx_rest_bind_scale_with_unskinned_prop_removal() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = unskinned_prop_fbx();
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let retained = run(dir.path());
    assert_eq!(retained.status.code(), Some(1));
    let detail = refusal_detail(&retained);
    assert!(detail.contains("rest_bind_scale plan rejected input base.fbx"));
    assert!(detail.contains("carries unskinned geometry inside the affected closure"));
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );

    let wrong_removal =
        fbx_recipe_v7("walk.fbx").replacen("fps = 30.0", "fps = 30.0\nremove_nodes = [\"tri\"]", 1);
    std::fs::write(dir.path().join("recipe.toml"), wrong_removal).unwrap();
    let retained = run(dir.path());
    assert_eq!(retained.status.code(), Some(1));
    let detail = refusal_detail(&retained);
    assert!(detail.contains("rest_bind_scale plan rejected input base.fbx"));
    assert!(detail.contains("carries unskinned geometry inside the affected closure"));
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );

    let explicitly_retained = fbx_recipe_v7("walk.fbx").replacen(
        "fps = 30.0",
        "fps = 30.0\nmesh_instances = [\"prop\"]\nremove_nodes = [\"prop-parent\"]",
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), explicitly_retained).unwrap();
    let retained = run(dir.path());
    assert_eq!(retained.status.code(), Some(1));
    let refusal: Value = serde_json::from_slice(&retained.stdout).expect("typed refusal JSON");
    assert_eq!(refusal["rejection"]["stage"], "selection");
    assert_eq!(refusal["rejection"]["kind"], "asset-recipe-mismatch");
    let detail = refusal_detail(&retained);
    assert!(detail.contains("rest_bind_scale FBX base projection rejected input base.fbx"));
    assert!(detail.contains(
        "mesh_instances selection retains no skinned base mesh instance for rest_bind_scale"
    ));
    assert!(!detail.contains("staged GLB"));
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );

    let recipe = fbx_recipe_v7("walk.fbx").replacen(
        "fps = 30.0",
        "fps = 30.0\ncanonicalize_skin = true\nground_and_center = true\nremove_nodes = [\"prop-parent\"]",
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let removed = run(dir.path());
    assert!(
        removed.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&removed.stdout),
        String::from_utf8_lossy(&removed.stderr)
    );
    let artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence_bytes = std::fs::read(dir.path().join("character.json")).unwrap();
    assert_eq!(removed.stdout, evidence_bytes);
    let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(
        evidence["transforms"]["retained_mesh_instances"],
        serde_json::json!(["tri"])
    );
    assert_eq!(evidence["transforms"]["removed_mesh_instances"], 1);
    assert_eq!(evidence["transforms"]["canonicalized_skin"], true);
    assert_eq!(evidence["transforms"]["ground_and_center"], true);
    let raw_stage = normalized_fbx_stage_bytes(dir.path(), "raw-prop", source.as_bytes());
    for input in evidence["rest_bind_scale"]["inputs"].as_array().unwrap() {
        assert_eq!(input["sha256"], sha256_hex(source.as_bytes()));
        assert_eq!(input["source_projection"]["kind"], "normalized-baked-fbx");
        assert_ne!(
            input["source_projection"]["staged_source"]["sha256"],
            sha256_hex(&raw_stage)
        );
    }
    let removed_nodes = evidence["transforms"]["removed_nodes"].as_array().unwrap();
    assert_eq!(removed_nodes.len(), 2);
    assert!(
        removed_nodes
            .iter()
            .any(|node| { node["name"] == "prop-parent" && node["selected"] == true })
    );
    assert!(
        removed_nodes
            .iter()
            .any(|node| { node["name"] == "prop" && node["selected"] == false })
    );
    assert_eq!(
        evidence["rest_bind_scale"]["read_back_sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );
    assert_eq!(evidence["artifact"]["sha256"], sha256_hex(&artifact));
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.assets.instances.len(), 1);
    assert!(
        document
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.name != "prop")
    );
}

#[test]
fn v7_resolves_fbx_mesh_selection_before_private_holder_rename() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = renamed_skinned_holder_with_unselected_prop_fbx();
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), &source).unwrap();
    let raw = animsmith_fbx::load_bytes(Path::new("base.fbx"), source.as_bytes())
        .expect("analytic renamed-holder FBX loads");
    assert_eq!(
        raw.assets
            .instances
            .iter()
            .map(|instance| raw.skeleton.bone_name(instance.node))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["body-instance", "prop"])
    );
    assert_eq!(
        raw.assets
            .meshes
            .iter()
            .map(|mesh| mesh.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["prop", "tri"]),
        "the selected raw instance name deliberately differs from its mesh definition"
    );
    assert_eq!(
        raw.assets
            .materials
            .iter()
            .map(|material| material.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["body-material", "prop-material"])
    );
    let mut expected_stage = raw.clone();
    let expected_skeleton = &expected_stage.skeleton;
    expected_stage
        .assets
        .instances
        .retain(|instance| expected_skeleton.bone_name(instance.node) == "body-instance");
    assert_eq!(expected_stage.assets.instances[0].mesh, 0);
    expected_stage.assets.meshes.truncate(1);
    expected_stage.assets.materials.truncate(1);
    let expected_stage_path = dir.path().join("expected-selected-stage.glb");
    animsmith_gltf::write::write(&expected_stage, &expected_stage_path).unwrap();
    let expected_stage_bytes = std::fs::read(expected_stage_path).unwrap();
    let expected_stage_sha256 = sha256_hex(&expected_stage_bytes);
    let expected_stage_size = u64::try_from(expected_stage_bytes.len()).unwrap();

    let recipe = fbx_recipe_v7("walk.fbx").replacen(
        "fps = 30.0",
        "fps = 30.0\nmesh_instances = [\"body-instance\"]",
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact_bytes = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(
        evidence["transforms"]["retained_mesh_instances"],
        serde_json::json!(["body-instance"])
    );
    assert_eq!(evidence["transforms"]["removed_mesh_instances"], 1);
    assert_eq!(
        (
            evidence["rest_bind_scale"]["inputs"][0]["source_projection"]["staged_source"]
                ["sha256"]
                .as_str(),
            evidence["rest_bind_scale"]["inputs"][0]["source_projection"]["staged_source"]
                ["bytes"]
                .as_u64(),
        ),
        (
            Some(expected_stage_sha256.as_str()),
            Some(expected_stage_size),
        ),
        "the private scale source must be the independently selected raw-node projection"
    );
    assert_eq!(evidence["artifact"]["sha256"], sha256_hex(&artifact_bytes));
    assert_eq!(
        evidence["rest_bind_scale"]["read_back_sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );

    let assembled = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(
        (
            assembled.assets.instances.len(),
            assembled.assets.meshes.len(),
            assembled.assets.materials.len(),
        ),
        (1, 1, 1),
        "the unselected instance, mesh definition, and material must not reappear"
    );
    let instance = &assembled.assets.instances[0];
    assert_eq!(assembled.skeleton.bone_name(instance.node), "tri_skinned");
    assert_eq!(assembled.assets.meshes[instance.mesh].name, "tri");
    assert_eq!(assembled.assets.materials[0].name, "body-material");
    assert_eq!(
        assembled.assets.meshes[instance.mesh].primitives[0].material,
        Some(0)
    );
    assert!(!instance.skin_joints.is_empty());
    assert_eq!(instance.skin_joints.len(), instance.skin_ibms.len());
    assert!(
        instance
            .skin_joints
            .iter()
            .all(|joint| *joint < assembled.skeleton.bones.len())
    );
    assert!(instance.skin_ibms.iter().all(|ibm| ibm.is_finite()));
}

#[test]
fn v7_missing_raw_fbx_mesh_instance_name_refuses_atomically() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = renamed_skinned_holder_with_unselected_prop_fbx();
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), &source).unwrap();
    let recipe = fbx_recipe_v7("walk.fbx").replacen(
        "fps = 30.0",
        "fps = 30.0\nmesh_instances = [\"tri\"]",
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(
        refusal_detail(&output)
            .contains("mesh_instances entry \"tri\" matches no base mesh instance node")
    );
    assert_eq!(
        (
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            std::fs::read(dir.path().join("character.json")).unwrap(),
        ),
        (prior_artifact.to_vec(), prior_evidence.to_vec())
    );
}

#[test]
fn v7_ambiguous_raw_fbx_mesh_instance_name_refuses_atomically() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = renamed_skinned_holder_with_unselected_prop_fbx().replacen(
        "\tModel: 1003, \"Model::prop\", \"Mesh\"",
        "\tModel: 1003, \"Model::body-instance\", \"Mesh\"",
        1,
    );
    let raw = animsmith_fbx::load_bytes(Path::new("base.fbx"), source.as_bytes())
        .expect("analytic ambiguous-instance FBX loads");
    assert_eq!(
        raw.assets
            .instances
            .iter()
            .filter(|instance| raw.skeleton.bone_name(instance.node) == "body-instance")
            .map(|instance| instance.node)
            .collect::<BTreeSet<_>>()
            .len(),
        2,
        "the selected name belongs to two distinct raw instance nodes"
    );
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), &source).unwrap();
    let recipe = fbx_recipe_v7("walk.fbx").replacen(
        "fps = 30.0",
        "fps = 30.0\nmesh_instances = [\"body-instance\"]",
        1,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(
        refusal_detail(&output).contains(
            "mesh_instances entry \"body-instance\" matches 2 base mesh instance nodes; expected exactly one"
        )
    );
    assert_eq!(
        (
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            std::fs::read(dir.path().join("character.json")).unwrap(),
        ),
        (prior_artifact.to_vec(), prior_evidence.to_vec())
    );
}

#[test]
fn v7_admits_scale_invariant_fbx_conversion_fidelity_without_erasing_evidence() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = scale_invariant_fidelity_fbx();
    std::fs::write(dir.path().join("inputs/base.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), &source).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence_bytes = std::fs::read(dir.path().join("character.json")).unwrap();
    assert_eq!(output.stdout, evidence_bytes);
    let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(evidence["artifact"]["sha256"], sha256_hex(&artifact));
    assert_eq!(
        evidence["rest_bind_scale"]["read_back_sha256"],
        evidence["artifact"]["sha256"]
    );
    for input in evidence["rest_bind_scale"]["inputs"].as_array().unwrap() {
        let capability = &input["source_projection"]["capability"];
        assert_eq!(capability["unsupported_vertex_payload_mesh_count"], 1);
        assert_eq!(
            capability["domains"]["other_vertex_and_source_data"],
            "unsupported"
        );
        assert_eq!(capability["non_triangle_face_count"], 1);
        assert_eq!(capability["triangulated_face_count"], 1);
        assert_eq!(capability["omitted_non_polygon_face_count"], 0);
        assert_eq!(capability["truncated_influence_vertex_count"], 4);
        assert_eq!(capability["discarded_influence_count"], 4);
        assert_eq!(capability["rejected_influence_count"], 4);
        assert_eq!(capability["renormalized_influence_vertex_count"], 4);
        assert_eq!(capability["bone_convenience_bind_overwrite_count"], 0);
        assert_eq!(capability["missing_skin_influence_corner_count"], 0);
        assert_eq!(capability["pre_weld_vertex_count"], 6);
        assert_eq!(capability["post_weld_vertex_count"], 4);
    }
    animsmith_gltf::load_bytes(Path::new("character.glb"), &artifact)
        .expect("the exact published artifact reloads");
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );
    assert!(
        evidence["rest_bind_scale"]["proof"]["proof"]["residuals"]
            .as_object()
            .is_some_and(|residuals| residuals
                .values()
                .all(|value| value.get("evaluated").is_some())),
        "the shared rest/bind proof must evaluate every normalized before/after residual"
    );
    assert!(
        evidence["rest_bind_scale"]["residual_comparison_counts"]
            .as_object()
            .is_some_and(|counts| counts.values().filter_map(Value::as_u64).sum::<u64>() > 0),
        "the admitted path must exercise the normalized before/after proof"
    );

    let second = run(dir.path());
    assert!(second.status.success());
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        artifact
    );
    assert_eq!(second.stdout, evidence_bytes);
}

#[test]
fn v7_named_selector_applies_a_declared_factor_two_and_pins_the_proof() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let authored = factor_two_named_root_rig_glb();
    std::fs::write(dir.path().join("inputs/base.glb"), &authored).unwrap();
    std::fs::write(dir.path().join("inputs/walk.glb"), &authored).unwrap();
    let recipe = fbx_recipe_v7("walk.glb")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.glb\"")
        .replace("expected_factor = 0.01", "expected_factor = 2.0")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let scale = &evidence["rest_bind_scale"];
    assert_eq!(scale["expected_factor"], 2.0);
    assert_eq!(scale["proof"]["factors"]["declared"], 2.0);
    assert_eq!(scale["proof"]["factors"]["planned_observed"], 2.0);
    assert_eq!(scale["proof"]["factors"]["proved_observed"], 2.0);
    assert_eq!(scale["proof"]["factors"]["divergence"], 0.0);
    assert_eq!(scale["read_back_sha256"], sha256_hex(&artifact));
    assert_eq!(scale["proof"]["artifact"]["sha256"], sha256_hex(&artifact));
    assert_eq!(scale["proof"]["artifact"]["bytes"], artifact.len());
    assert_eq!(scale["proof"]["proof"]["read_back_digest_matches"], true);
    assert!(
        scale["proof"]["artifact"]["rewritten_json_pointers"]
            .as_array()
            .is_some_and(|pointers| !pointers.is_empty())
    );
    assert!(
        scale["residual_comparison_counts"]
            .as_object()
            .unwrap()
            .values()
            .filter_map(Value::as_u64)
            .sum::<u64>()
            > 0
    );

    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.skeleton.bones[0].rest.scale, Vec3::ONE);
    assert_eq!(document.skeleton.bones[1].rest.translation.y, 200.0);
    let translation = document.clips[0]
        .tracks
        .iter()
        .find(|track| track.property == Property::Translation)
        .unwrap();
    let TrackValues::Vec3s(values) = &translation.values else {
        panic!("translation values")
    };
    assert_eq!(values[0].y, 200.0);
    assert_eq!(values[1].y, 600.0);
}

#[test]
fn v7_uses_the_declared_exact_name_without_a_root_literal_special_case() {
    let renamed = RIGGED_TRIANGLE_FBX.replace("\"Model::root\"", "\"Model::scale-root\"");
    for (declared, succeeds) in [("scale-root", true), ("Scale-root", false)] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(dir.path().join("inputs/base.fbx"), &renamed).unwrap();
        std::fs::write(dir.path().join("inputs/walk.fbx"), &renamed).unwrap();
        let recipe = fbx_recipe_v7("walk.fbx").replace(
            "root_node_name = \"root\"",
            &format!("root_node_name = {declared:?}"),
        );
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
        let output = run(dir.path());
        if succeeds {
            assert!(output.status.success());
            let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                evidence["rest_bind_scale"]["declared_root_node_name"],
                "scale-root"
            );
            for input in evidence["rest_bind_scale"]["inputs"].as_array().unwrap() {
                assert_eq!(input["resolved_root_node_name"], "scale-root");
            }
        } else {
            assert_eq!(output.status.code(), Some(1));
            assert!(refusal_detail(&output).contains("resolves to 0 source nodes"));
            assert!(!dir.path().join("character.glb").exists());
            assert!(!dir.path().join("character.json").exists());
        }
    }
}

#[test]
fn v7_name_selector_resolves_a_scaled_ancestor_above_the_selected_skin() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let source = rest_bind_scale_rig_gltf();
    std::fs::write(dir.path().join("inputs/base.gltf"), &source).unwrap();
    std::fs::write(dir.path().join("inputs/walk.gltf"), &source).unwrap();
    let recipe = fbx_recipe_v7("walk.gltf")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.gltf\"")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence_bytes = std::fs::read(dir.path().join("character.json")).unwrap();
    assert_eq!(output.stdout, evidence_bytes);
    let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
    assert_eq!(evidence["artifact"]["sha256"], sha256_hex(&artifact));
    assert_eq!(evidence["artifact"]["bytes"], artifact.len());
    for (index, input) in evidence["rest_bind_scale"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(input["resolved_root_node_name"], "root");
        assert_eq!(input["resolved_source_root_node_index"], 0);
        assert_eq!(input["resolved_source_skin_index"], 0, "input {index}");
    }
    assert_eq!(
        evidence["rest_bind_scale"]["proof"]["proof"]["read_back_digest_matches"],
        true
    );
}

#[test]
fn v7_name_selector_refuses_missing_ambiguous_or_outside_roots_atomically() {
    let mut ambiguous_root: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    ambiguous_root["nodes"][1]["name"] = Value::String("root".into());
    let mut ambiguous_skin: Value = serde_json::from_slice(&rest_bind_scale_rig_gltf()).unwrap();
    let duplicate_skin = ambiguous_skin["skins"][0].clone();
    ambiguous_skin["skins"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_skin);
    for (ordinal, input_name, base_bytes, root_name, expected) in [
        (
            0,
            "base.fbx",
            RIGGED_TRIANGLE_FBX.as_bytes().to_vec(),
            "missing",
            "resolves to 0 source nodes; expected exactly one",
        ),
        (
            1,
            "base.gltf",
            rest_bind_scale_rig_gltf(),
            "holder",
            "fully governs 0 source skins; expected exactly one",
        ),
        (
            2,
            "base.gltf",
            serde_json::to_vec(&ambiguous_root).unwrap(),
            "root",
            "resolves to 2 source nodes; expected exactly one",
        ),
        (
            3,
            "base.gltf",
            serde_json::to_vec(&ambiguous_skin).unwrap(),
            "root",
            "fully governs 2 source skins; expected exactly one",
        ),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(dir.path().join("inputs").join(input_name), base_bytes).unwrap();
        std::fs::write(dir.path().join("inputs/walk.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
        let recipe = fbx_recipe_v7("walk.fbx")
            .replace(
                "base_input = \"base.fbx\"",
                &format!("base_input = {input_name:?}"),
            )
            .replace(
                "root_node_name = \"root\"",
                &format!("root_node_name = {root_name:?}"),
            );
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
        let prior_artifact = b"prior artifact";
        let prior_evidence = b"prior evidence";
        std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
        std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(1), "case {ordinal}");
        let detail = refusal_detail(&output);
        assert!(
            detail.contains(expected),
            "case {ordinal}: expected {expected:?} in {detail:?}"
        );
        assert_eq!(
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            prior_artifact,
            "case {ordinal}"
        );
        assert_eq!(
            std::fs::read(dir.path().join("character.json")).unwrap(),
            prior_evidence,
            "case {ordinal}"
        );
    }
}

#[test]
fn v7_resolves_every_clip_selector_and_refuses_clip_side_misses_atomically() {
    for (ordinal, mutation, expected) in [
        (0, "missing-root", "resolves to 0 source nodes"),
        (1, "ambiguous-root", "resolves to 2 source nodes"),
        (
            2,
            "different-skin",
            "assembly scale basis mismatch (source-name-selector)",
        ),
        (3, "ambiguous-skin", "fully governs 2 source skins"),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
        let clip_path = dir.path().join("inputs/walk.gltf");
        let mut clip = normalized_fbx_gltf_value(&clip_path);
        match mutation {
            "missing-root" => clip["nodes"][1]["name"] = Value::String("other".into()),
            "ambiguous-root" => clip["nodes"][2]["name"] = Value::String("root".into()),
            "different-skin" => clip["skins"][0]["joints"] = serde_json::json!([2]),
            "ambiguous-skin" => {
                let duplicate = clip["skins"][0].clone();
                clip["skins"].as_array_mut().unwrap().push(duplicate);
            }
            _ => unreachable!(),
        }
        std::fs::write(&clip_path, serde_json::to_vec(&clip).unwrap()).unwrap();
        std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.gltf")).unwrap();
        let prior_artifact = b"prior artifact";
        let prior_evidence = b"prior evidence";
        std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
        std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(1), "case {ordinal}");
        let detail = refusal_detail(&output);
        assert!(detail.contains(expected), "case {ordinal}: {detail}");
        assert_eq!(
            std::fs::read(dir.path().join("character.glb")).unwrap(),
            prior_artifact
        );
        assert_eq!(
            std::fs::read(dir.path().join("character.json")).unwrap(),
            prior_evidence
        );
    }
}

#[test]
fn v7_resolves_names_independently_across_mixed_fbx_and_gltf_inputs() {
    for (base, clip, expected_formats) in [
        ("base.fbx", "walk.glb", ["fbx", "glb"]),
        ("base.glb", "walk.fbx", ["glb", "fbx"]),
        ("base.fbx", "walk.gltf", ["fbx", "gltf"]),
        ("base.gltf", "walk.fbx", ["gltf", "fbx"]),
        ("base.glb", "walk.glb", ["glb", "glb"]),
        ("base.gltf", "walk.gltf", ["gltf", "gltf"]),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        for input in [base, clip] {
            let path = dir.path().join("inputs").join(input);
            if input.ends_with(".fbx") {
                std::fs::write(path, RIGGED_TRIANGLE_FBX).unwrap();
            } else {
                write_normalized_fbx_glb(&path);
            }
        }
        let recipe = fbx_recipe_v7(clip).replace(
            "base_input = \"base.fbx\"",
            &format!("base_input = {base:?}"),
        );
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

        let output = run(dir.path());
        assert!(
            output.status.success(),
            "{base} + {clip}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
        let inputs = evidence["rest_bind_scale"]["inputs"].as_array().unwrap();
        for (index, (input, expected_format)) in inputs.iter().zip(expected_formats).enumerate() {
            assert_eq!(input["input_format"], expected_format);
            assert_eq!(input["resolved_root_node_name"], "root");
            if index == 0 || expected_format != "fbx" {
                assert!(input["resolved_source_skin_index"].is_u64());
            } else {
                assert!(input.get("resolved_source_skin_index").is_none());
            }
            assert!(input["resolved_source_root_node_index"].is_u64());
        }
    }
}

#[test]
fn v7_resolves_a_valid_clip_root_and_skin_in_its_own_source_index_space() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.glb"),
        unaffected_bind_scale_rig_glb(),
    )
    .unwrap();
    write_reindexed_and_reskinned_glb(&dir.path().join("inputs/walk.glb"));
    let recipe = fbx_recipe_v7("walk.glb")
        .replace("base_input = \"base.fbx\"", "base_input = \"base.glb\"")
        .replace("root_node_name = \"root\"", "root_node_name = \"joint\"")
        .replace("take = \"take\"", "take = \"clip\"");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    let inputs = evidence["rest_bind_scale"]["inputs"].as_array().unwrap();
    assert_eq!(inputs[0]["resolved_source_skin_index"], 0);
    assert_eq!(inputs[1]["resolved_source_skin_index"], 1);
    assert_eq!(inputs[0]["resolved_source_root_node_index"], 1);
    assert_eq!(inputs[1]["resolved_source_root_node_index"], 0);
    assert_eq!(inputs[0]["resolved_root_node_name"], "joint");
    assert_eq!(inputs[1]["resolved_root_node_name"], "joint");
}

#[test]
fn v4_through_v7_keep_selector_forms_required_fields_and_identities_disjoint() {
    let padded_recipe: toml::Value = toml::from_str(
        &fbx_recipe_v7("walk.fbx")
            .replace("root_node_name = \"root\"", "root_node_name = \" root \""),
    )
    .unwrap();
    let schema: Value = serde_json::from_str(RECIPE_SCHEMA_V7).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(
        !validator.is_valid(&serde_json::to_value(padded_recipe).unwrap()),
        "v7 schema must reject boundary whitespace that the CLI refuses"
    );

    for (ordinal, recipe, expected) in [
        (
            0,
            fbx_recipe_v6("walk.fbx")
                .replace("source_skin_index = 0\n", "")
                .replace("source_root_node_index = 1", "root_node_name = \"root\""),
            Some("character-assembly-recipe v6 rest_bind_scale does not admit root_node_name"),
        ),
        (
            1,
            fbx_recipe_v7("walk.fbx").replace(
                "root_node_name = \"root\"",
                "root_node_name = \"root\"\nsource_skin_index = 0\nsource_root_node_index = 1",
            ),
            Some("character-assembly-recipe v7 rest_bind_scale does not admit source indices"),
        ),
        (
            2,
            fbx_recipe_v7("walk.fbx").replace(
                "root_node_name = \"root\"",
                "source_skin_index = 0\nsource_root_node_index = 1",
            ),
            Some("character-assembly-recipe v7 rest_bind_scale does not admit source indices"),
        ),
        (
            3,
            fbx_recipe_v7("walk.fbx").replace("expected_factor = 0.01\n", ""),
            Some("missing field `expected_factor`"),
        ),
        (
            4,
            fbx_recipe_v7("walk.fbx").replacen("schema_version = 7", "schema_version = 6", 1),
            Some("unsupported assembly recipe identity"),
        ),
        (
            5,
            fbx_recipe_v7("walk.fbx").replacen(
                "character-assembly-recipe:7",
                "character-assembly-recipe:6",
                1,
            ),
            Some("unsupported assembly recipe identity"),
        ),
        (
            6,
            fbx_recipe_v7("walk.fbx").replace("root_node_name = \"root\"\n", ""),
            Some("missing field `root_node_name`"),
        ),
        (
            7,
            fbx_recipe_v7("walk.fbx")
                .replace("root_node_name = \"root\"", "root_node_name = \" root \""),
            Some("must be non-empty and contain no leading or trailing whitespace"),
        ),
        (
            8,
            fbx_recipe_v6("walk.fbx")
                .replacen("schema_version = 6", "schema_version = 4", 1)
                .replacen(
                    "character-assembly-recipe:6",
                    "character-assembly-recipe:4",
                    1,
                )
                .replace("source_skin_index = 0\n", ""),
            Some("missing field `source_skin_index`"),
        ),
        (
            9,
            fbx_recipe_v6("walk.fbx")
                .replacen("schema_version = 6", "schema_version = 5", 1)
                .replacen(
                    "character-assembly-recipe:6",
                    "character-assembly-recipe:5",
                    1,
                )
                .replace("source_root_node_index = 1\n", ""),
            Some("missing field `source_root_node_index`"),
        ),
        (
            10,
            fbx_recipe_v6("walk.fbx").replace("expected_factor = 0.01\n", ""),
            Some("missing field `expected_factor`"),
        ),
        (
            11,
            fbx_recipe_v7("walk.fbx").replace(
                "root_node_name = \"root\"",
                "root_node_name = \"root\"\nsource_node_name = \"root\"",
            ),
            Some(
                "unknown field `source_node_name` in character-assembly-recipe v7 rest_bind_scale",
            ),
        ),
        (
            12,
            fbx_recipe_v6("walk.fbx").replace(
                "source_root_node_index = 1",
                "source_root_node_index = 1\nsource_node_name = \"root\"",
            ),
            Some(
                "unknown field `source_node_name` in character-assembly-recipe v6 rest_bind_scale",
            ),
        ),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
        std::fs::write(dir.path().join("inputs/walk.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(2), "case {ordinal}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(expected) = expected {
            assert!(stderr.contains(expected), "case {ordinal}: {stderr}");
        }
        assert!(
            !stderr.contains("AssemblyRestBindScaleRecipe") && !stderr.contains("untagged enum"),
            "case {ordinal} leaked a private serde diagnostic: {stderr}"
        );
        assert!(!dir.path().join("character.glb").exists());
        assert!(!dir.path().join("character.json").exists());
    }
}

#[test]
fn v6_and_v7_accept_scale_irrelevant_fbx_custom_properties() {
    for (label, recipe, custom_base, custom_clip) in [
        ("v6-base", fbx_recipe_v6("walk.fbx"), true, false),
        ("v6-clip", fbx_recipe_v6("walk.fbx"), false, true),
        ("v7-base", fbx_recipe_v7("walk.fbx"), true, false),
        ("v7-clip", fbx_recipe_v7("walk.fbx"), false, true),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(
            dir.path().join("inputs/base.fbx"),
            if custom_base {
                user_property_fbx()
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("inputs/walk.fbx"),
            if custom_clip {
                user_property_fbx()
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

        let output = run(dir.path());
        assert!(
            output.status.success(),
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_schema(
            &evidence,
            if label.starts_with("v6") {
                EVIDENCE_SCHEMA_V6
            } else {
                EVIDENCE_SCHEMA_V7
            },
        );
        assert_eq!(
            evidence["artifact"]["sha256"], evidence["rest_bind_scale"]["read_back_sha256"],
            "{label}: the exact staged artifact remains the proven result"
        );
        let inputs = evidence["rest_bind_scale"]["inputs"]
            .as_array()
            .expect("scale evidence inputs");
        assert_eq!(inputs.len(), 2, "{label}");
        assert_eq!(
            inputs[0]["source_projection"]["capability"]["user_defined_property_count"],
            usize::from(custom_base),
            "{label}: base projection belongs to the base input"
        );
        assert_eq!(
            inputs[1]["source_projection"]["capability"]["user_defined_property_count"],
            usize::from(custom_clip),
            "{label}: clip projection belongs to the clip input"
        );
        assert!(dir.path().join("character.glb").exists(), "{label}");
        assert!(dir.path().join("character.json").exists(), "{label}");
    }
}

#[test]
fn v7_accepts_external_fbx_texture_references_as_scale_irrelevant() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir_all(dir.path().join("inputs/nested")).unwrap();
    std::fs::write(
        dir.path().join("inputs/nested/base.fbx"),
        external_normal_texture_fbx(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("inputs/nested/walk.fbx"),
        RIGGED_TRIANGLE_FBX,
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/nested/normal.png"), TINY_PNG).unwrap();
    std::fs::write(dir.path().join("inputs/normal.png"), b"wrong-root decoy").unwrap();
    let recipe = fbx_recipe_v7("nested/walk.fbx").replace(
        "base_input = \"base.fbx\"",
        "base_input = \"nested/base.fbx\"",
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let source = animsmith_fbx::load(&dir.path().join("inputs/nested/base.fbx"))
        .expect("analytic external-texture FBX loads");
    assert_eq!(
        source.assets.materials[0]
            .normal_texture
            .as_ref()
            .expect("source carries captured texture")
            .texture
            .bytes,
        TINY_PNG
    );

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
    assert_eq!(
        evidence["artifact"]["sha256"],
        evidence["rest_bind_scale"]["read_back_sha256"]
    );
    assert_eq!(
        evidence["rest_bind_scale"]["inputs"][0]["source_projection"]["capability"]["external_resource_count"],
        2,
        "the immutable evidence remains honest about both texture/video declarations"
    );
    assert_eq!(
        evidence["rest_bind_scale"]["inputs"][1]["source_projection"]["capability"]["external_resource_count"],
        0,
        "the clean clip keeps its own source projection rather than inheriting the base count"
    );
    let artifact = animsmith_gltf::load(&dir.path().join("character.glb"))
        .expect("published artifact reloads");
    assert_eq!(
        artifact.assets.materials[0]
            .normal_texture
            .as_ref()
            .expect("published material retains its captured normal texture")
            .texture
            .bytes,
        TINY_PNG,
        "rest/bind staging must not turn an admitted external texture into silent data loss"
    );
}

#[test]
fn v7_refuses_a_destination_that_aliases_a_captured_fbx_dependency() {
    for (source_role, destination, locator, output, evidence) in [
        (
            "base",
            "evidence",
            "normal.png",
            "character.glb",
            "inputs/base/normal.png",
        ),
        (
            "base",
            "output",
            "artifact.glb",
            "inputs/base/artifact.glb",
            "character.json",
        ),
        (
            "clip",
            "evidence",
            "normal.png",
            "character.glb",
            "inputs/clip/normal.png",
        ),
        (
            "clip",
            "output",
            "artifact.glb",
            "inputs/clip/artifact.glb",
            "character.json",
        ),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir_all(dir.path().join("inputs/base")).unwrap();
        std::fs::create_dir_all(dir.path().join("inputs/clip")).unwrap();
        let external = external_normal_texture_fbx().replace("normal.png", locator);
        std::fs::write(
            dir.path().join("inputs/base/base.fbx"),
            if source_role == "base" {
                external.as_str()
            } else {
                RIGGED_TRIANGLE_FBX
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("inputs/clip/walk.fbx"),
            if source_role == "clip" {
                external.as_str()
            } else {
                RIGGED_TRIANGLE_FBX
            },
        )
        .unwrap();
        let dependency = dir.path().join("inputs").join(source_role).join(locator);
        std::fs::write(&dependency, TINY_PNG).unwrap();
        let recipe = fbx_recipe_v7("clip/walk.fbx").replace(
            "base_input = \"base.fbx\"",
            "base_input = \"base/base.fbx\"",
        );
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
        let (peer, prior_peer) = if destination == "evidence" {
            (dir.path().join(output), b"prior artifact".as_slice())
        } else {
            (dir.path().join(evidence), b"prior evidence".as_slice())
        };
        std::fs::write(&peer, prior_peer).unwrap();

        let result = run_to(dir.path(), "recipe.toml", output, evidence);
        assert_eq!(
            result.status.code(),
            Some(2),
            "{source_role}/{destination} stderr:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stdout.is_empty(), "{source_role}/{destination}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(&format!(
                "assemble external dependency {locator:?} and {destination} must be different paths"
            )),
            "{source_role}/{destination} stderr:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            std::fs::read(dependency).unwrap(),
            TINY_PNG,
            "{source_role}/{destination}: assembly must not replace the source dependency"
        );
        assert_eq!(
            std::fs::read(peer).unwrap(),
            prior_peer,
            "{source_role}/{destination}: refusal must preserve the non-aliased peer"
        );
    }
}

#[test]
fn v7_external_texture_does_not_mask_a_real_unmodeled_construct() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    std::fs::write(
        dir.path().join("inputs/walk.fbx"),
        external_normal_texture_with_unmodeled_pose_fbx(),
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/normal.png"), TINY_PNG).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        refusal_detail(&output),
        concat!(
            "rest_bind_scale FBX clip-track capability rejected input walk.fbx: ",
            "FBX rest/bind raw-source facts rejected: ",
            "raw_source.construct=unknown_element(fbx:unmodeled-elements; ",
            "count=1; incomplete_bind_poses=1)"
        )
    );
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v7_fbx_capability_refusal_names_the_exact_unsupported_source_fact() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(
        dir.path().join("inputs/base.fbx"),
        nonbearing_node_attributes_fbx(true),
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/walk.fbx"), RIGGED_TRIANGLE_FBX).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        refusal_detail(&output),
        concat!(
            "rest_bind_scale FBX capability rejected input base.fbx: ",
            "FBX rest/bind raw-source facts rejected: ",
            "raw_source.construct=unknown_element(fbx:unmodeled-elements; ",
            "count=1; incomplete_bind_poses=1)"
        )
    );
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v7_admits_exact_nonbearing_fbx_node_attributes_for_base_and_clip() {
    for source_role in ["base", "clip"] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(
            dir.path().join("inputs/base.fbx"),
            if source_role == "base" {
                nonbearing_node_attributes_fbx(false)
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("inputs/walk.fbx"),
            if source_role == "clip" {
                nonbearing_node_attributes_fbx(false)
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

        let output = run(dir.path());
        assert!(
            output.status.success(),
            "{source_role}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
        let inputs = evidence["rest_bind_scale"]["inputs"]
            .as_array()
            .expect("scale evidence inputs");
        assert_eq!(inputs.len(), 2, "{source_role}");
        assert_eq!(
            inputs[0]["source_projection"]["capability"]["unsupported_source_element_count"],
            usize::from(source_role == "base") * 4,
            "{source_role}: base evidence retains the raw aggregate"
        );
        assert_eq!(
            inputs[1]["source_projection"]["capability"]["unsupported_source_element_count"],
            usize::from(source_role == "clip") * 4,
            "{source_role}: clip evidence retains the raw aggregate"
        );
        assert!(dir.path().join("character.glb").exists(), "{source_role}");
        assert!(dir.path().join("character.json").exists(), "{source_role}");
    }
}

#[test]
fn v7_admits_display_layer_editor_metadata_for_base_and_clip() {
    for source_role in ["base", "clip"] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(
            dir.path().join("inputs/base.fbx"),
            if source_role == "base" {
                display_layer_fbx()
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("inputs/walk.fbx"),
            if source_role == "clip" {
                display_layer_fbx()
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

        let output = run(dir.path());
        assert!(
            output.status.success(),
            "{source_role}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
        let inputs = evidence["rest_bind_scale"]["inputs"]
            .as_array()
            .expect("scale evidence inputs");
        assert_eq!(inputs.len(), 2, "{source_role}");
        assert_eq!(
            inputs[0]["source_projection"]["capability"]["unsupported_source_element_count"],
            usize::from(source_role == "base"),
            "{source_role}: base evidence retains the raw display-layer declaration"
        );
        assert_eq!(
            inputs[1]["source_projection"]["capability"]["unsupported_source_element_count"],
            usize::from(source_role == "clip"),
            "{source_role}: clip evidence retains the raw display-layer declaration"
        );
        assert!(dir.path().join("character.glb").exists(), "{source_role}");
        assert!(dir.path().join("character.json").exists(), "{source_role}");
    }
}

#[test]
fn v7_admits_shader_bindings_and_reconciled_bind_pose_for_base_and_clip() {
    for source_role in ["base", "clip"] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(
            dir.path().join("inputs/base.fbx"),
            if source_role == "base" {
                bind_pose_with_shader_fbx(IDENTITY_FBX_MATRIX)
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("inputs/walk.fbx"),
            if source_role == "clip" {
                bind_pose_with_shader_fbx(IDENTITY_FBX_MATRIX)
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

        let output = run(dir.path());
        assert!(
            output.status.success(),
            "{source_role}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_schema(&evidence, EVIDENCE_SCHEMA_V7);
        let inputs = evidence["rest_bind_scale"]["inputs"]
            .as_array()
            .expect("scale evidence inputs");
        assert_eq!(inputs.len(), 2, "{source_role}");
        assert_eq!(
            inputs[0]["source_projection"]["capability"]["unsupported_source_element_count"],
            usize::from(source_role == "base") * 3,
            "{source_role}: base evidence retains pose/shader declarations"
        );
        assert_eq!(
            inputs[1]["source_projection"]["capability"]["unsupported_source_element_count"],
            usize::from(source_role == "clip") * 3,
            "{source_role}: clip evidence retains pose/shader declarations"
        );
        assert!(dir.path().join("character.glb").exists(), "{source_role}");
        assert!(dir.path().join("character.json").exists(), "{source_role}");
    }
}

#[test]
fn v7_refuses_mismatched_bind_pose_for_base_and_clip() {
    let mismatched = "1,0,0,0,0,1,0,0,0,0,1,0,100,0,0,1";
    for source_role in ["base", "clip"] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        std::fs::write(
            dir.path().join("inputs/base.fbx"),
            if source_role == "base" {
                bind_pose_with_shader_fbx(mismatched)
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("inputs/walk.fbx"),
            if source_role == "clip" {
                bind_pose_with_shader_fbx(mismatched)
            } else {
                RIGGED_TRIANGLE_FBX.to_owned()
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("recipe.toml"), fbx_recipe_v7("walk.fbx")).unwrap();

        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(1), "{source_role}");
        assert_eq!(
            refusal_detail(&output),
            format!(
                concat!(
                    "rest_bind_scale FBX {}capability rejected input {}.fbx: ",
                    "FBX rest/bind raw-source facts rejected: ",
                    "raw_source.construct=unknown_element(fbx:unmodeled-elements; ",
                    "count=1; mismatched_bind_poses=1)"
                ),
                if source_role == "base" {
                    ""
                } else {
                    "clip-track "
                },
                if source_role == "base" {
                    "base"
                } else {
                    "walk"
                }
            )
        );
        assert!(!dir.path().join("character.glb").exists(), "{source_role}");
        assert!(!dir.path().join("character.json").exists(), "{source_role}");
    }
}

#[test]
fn v6_accepts_mixed_fbx_and_glb_inputs_in_both_directions() {
    for (base, clip, expected_formats) in [
        ("base.fbx", "walk.glb", ["fbx", "glb"]),
        ("base.glb", "walk.fbx", ["glb", "fbx"]),
    ] {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        for input in [base, clip] {
            let path = dir.path().join("inputs").join(input);
            if input.ends_with(".fbx") {
                std::fs::write(path, RIGGED_TRIANGLE_FBX).unwrap();
            } else {
                write_normalized_fbx_glb(&path);
            }
        }
        let recipe = fbx_recipe_v6(clip).replace(
            "base_input = \"base.fbx\"",
            &format!("base_input = {base:?}"),
        );
        std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

        let output = run(dir.path());
        assert!(
            output.status.success(),
            "{base} + {clip}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_schema(&evidence, EVIDENCE_SCHEMA_V6);
        let inputs = evidence["rest_bind_scale"]["inputs"].as_array().unwrap();
        assert_eq!(inputs[0]["input_format"], expected_formats[0]);
        assert_eq!(inputs[1]["input_format"], expected_formats[1]);
        for input in inputs {
            assert_eq!(
                input["source_projection"]["kind"],
                if input["input_format"] == "fbx" {
                    "normalized-baked-fbx"
                } else {
                    "raw-gltf"
                }
            );
        }
        assert_eq!(
            evidence["rest_bind_scale"]["read_back_sha256"],
            evidence["artifact"]["sha256"]
        );
    }
}

#[test]
fn v6_records_gltf_inputs_as_raw_gltf_projections() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let bytes = rest_bind_scale_rig_gltf();
    std::fs::write(dir.path().join("inputs/base.gltf"), &bytes).unwrap();
    std::fs::write(dir.path().join("inputs/walk.gltf"), bytes).unwrap();
    let recipe = recipe("walk.gltf")
        .replace("base.glb", "base.gltf")
        .replacen("schema_version = 4", "schema_version = 6", 1)
        .replacen(
            "urn:animsmith:schema:character-assembly-recipe:4",
            "urn:animsmith:schema:character-assembly-recipe:6",
            1,
        );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V6);
    for input in evidence["rest_bind_scale"]["inputs"].as_array().unwrap() {
        assert_eq!(input["input_format"], "gltf");
        assert_eq!(input["source_projection"]["kind"], "raw-gltf");
    }
}

#[test]
fn v6_gltf_projection_adds_only_the_new_evidence_boundary() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let bytes = rest_bind_scale_rig_glb();
    std::fs::write(dir.path().join("inputs/base.glb"), &bytes).unwrap();
    std::fs::write(dir.path().join("inputs/walk.glb"), &bytes).unwrap();
    let v6 = recipe("walk.glb")
        .replacen("schema_version = 4", "schema_version = 6", 1)
        .replacen(
            "urn:animsmith:schema:character-assembly-recipe:4",
            "urn:animsmith:schema:character-assembly-recipe:6",
            1,
        );
    std::fs::write(dir.path().join("recipe.toml"), &v6).unwrap();
    let output = run(dir.path());
    assert!(output.status.success());
    let v6_artifact = std::fs::read(dir.path().join("character.glb")).unwrap();
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V6);
    for input in evidence["rest_bind_scale"]["inputs"].as_array().unwrap() {
        assert_eq!(input["input_format"], "glb");
        assert_eq!(input["source_projection"]["kind"], "raw-gltf");
        assert_eq!(
            input["source_projection"]["authored_curve_keys_preserved"],
            true
        );
        assert_eq!(
            input["source_projection"]["raw_source_spans_preserved"],
            true
        );
    }

    let v5 = v6
        .replacen("schema_version = 6", "schema_version = 5", 1)
        .replacen(
            "urn:animsmith:schema:character-assembly-recipe:6",
            "urn:animsmith:schema:character-assembly-recipe:5",
            1,
        );
    std::fs::write(dir.path().join("legacy.toml"), v5).unwrap();
    let legacy = run_to(dir.path(), "legacy.toml", "legacy.glb", "legacy.json");
    assert!(legacy.status.success());
    assert_eq!(
        std::fs::read(dir.path().join("legacy.glb")).unwrap(),
        v6_artifact,
        "v6 changes evidence/admission, not established glTF artifact semantics"
    );
    let legacy_evidence: Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_schema(&legacy_evidence, EVIDENCE_SCHEMA_V5);
    assert!(
        legacy_evidence["rest_bind_scale"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|input| input.get("input_format").is_none()
                && input.get("source_projection").is_none())
    );
}

#[test]
fn v4_has_no_default_rest_bind_operation() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let input = rest_bind_scale_rig_glb();
    std::fs::write(dir.path().join("inputs/base.glb"), &input).unwrap();
    std::fs::write(
        dir.path().join("inputs/clip.gltf"),
        rest_bind_scale_rig_gltf(),
    )
    .unwrap();
    std::fs::write(dir.path().join("inputs/clip-two.glb"), &input).unwrap();
    let recipe = format!(
        "{}\n[[clips]]\nname = \"run\"\ninput = \"clip-two.glb\"\ntake = \"clip\"\n",
        recipe("clip.gltf")
    )
    .replace(
        "[rest_bind_scale]\nsource_skin_index = 0\nsource_root_node_index = 0\nexpected_factor = 0.01\n\n",
        "",
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("character.json")).unwrap()).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA);
    assert_eq!(evidence["schema_version"], 4);
    assert!(evidence.get("rest_bind_scale").is_none());
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.skeleton.bones[0].rest.scale, Vec3::splat(0.01));
    assert_eq!(document.clips.len(), 2);
    assert!(document.clips.iter().all(|clip| clip.tracks.len() == 2));
}

#[test]
fn v5_composes_rest_bind_scale_with_canonical_grounding_and_node_removal() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
    write_cubic_asset(&dir.path().join("inputs/walk.glb"), 10.0);
    write_cubic_asset(&dir.path().join("inputs/run.glb"), 20.0);
    write_cubic_asset(&dir.path().join("inputs/stripped.glb"), 30.0);
    let recipe = format!(
        "{}\n[[clips]]\nname = \"run\"\ninput = \"run.glb\"\ntake = \"clip\"\n\n[[clips]]\nname = \"stripped\"\ninput = \"stripped.glb\"\ntake = \"clip\"\nstrip_bones = [\"joint\"]\n",
        recipe_v5("walk.glb")
    );
    let recipe_value: toml::Value = toml::from_str(&recipe).unwrap();
    assert_schema(
        &serde_json::to_value(recipe_value).unwrap(),
        RECIPE_SCHEMA_V5,
    );
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("character.json")).unwrap()).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V5);
    assert_eq!(evidence["schema_version"], 5);
    assert!(
        evidence["transforms"]["canonicalized_skin"]
            .as_bool()
            .unwrap()
    );
    assert!(
        evidence["transforms"]["ground_and_center"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(evidence["transforms"]["removed_nodes"][0]["name"], "attach");
    assert!(evidence["transforms"]["converted_bounds_min"][1].is_number());
    let scale = &evidence["rest_bind_scale"];
    assert_eq!(scale["source_skin_index"], 0);
    assert_eq!(scale["source_root_node_index"], 0);
    assert_eq!(scale["effective_source_skin_index"], 0);
    assert_eq!(scale["effective_source_root_node_index"], 1);
    assert_eq!(scale["expected_factor"], 0.01);
    assert_eq!(scale["inputs"].as_array().unwrap().len(), 4);
    assert_eq!(scale["inputs"][0]["role"], "base");
    assert_eq!(scale["inputs"][1]["role"], "clip:walk");
    assert_eq!(scale["inputs"][2]["role"], "clip:run");
    assert_eq!(scale["inputs"][3]["role"], "clip:stripped");
    assert!(
        scale["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|input| input["compatible"] == true)
    );
    assert_eq!(scale["read_back_sha256"], evidence["artifact"]["sha256"]);
    assert_eq!(
        scale["proof"]["artifact"]["sha256"],
        evidence["artifact"]["sha256"]
    );
    assert_eq!(scale["proof"]["proof"]["read_back_digest_matches"], true);
    assert_eq!(
        scale["residual_comparison_counts"],
        serde_json::json!({
            "bounds": 42,
            "cubic_interior": 2,
            "key_translation": 4,
            "mesh_position": 3,
            "rest_rotation": 2,
            "rest_translation": 2,
            "skin_matrix": 7,
            "track_value": 16,
            "trajectory": 12,
            "transform_only_affine": 0,
            "unaffected_inverse_bind": 0,
            "unit_scale": 2,
        })
    );
    assert_eq!(evidence["clips"].as_array().unwrap().len(), 3);
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert!(
        document
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.name != "attach")
    );
    assert!(
        document
            .skeleton
            .bones
            .iter()
            .all(|bone| bone.rest.scale == Vec3::ONE),
        "the composed rest hierarchy has unit local scale"
    );
    let worlds = rest_worlds(&document);
    let skinned_joints = document
        .assets
        .instances
        .iter()
        .flat_map(|instance| instance.skin_joints.iter().copied())
        .collect::<BTreeSet<_>>();
    for (index, (bone, world)) in document.skeleton.bones.iter().zip(&worlds).enumerate() {
        for axis in [world.x_axis, world.y_axis, world.z_axis] {
            assert!(
                (axis.truncate().length() - 1.0).abs() <= 1.0e-5,
                "{} has a unit rest-world axis",
                bone.name
            );
        }
        if skinned_joints.contains(&index) {
            let inverse_bind = bone
                .inverse_bind
                .expect("emitted skin joint has an inverse bind");
            assert!(
                (*world * inverse_bind).abs_diff_eq(Mat4::IDENTITY, 1.0e-4),
                "{} inverse bind matches the emitted rest world",
                bone.name
            );
        }
    }
    assert_eq!(document.clips.len(), 2);
    let walk = document
        .clips
        .iter()
        .find(|clip| clip.name == "walk")
        .unwrap();
    let track = walk
        .tracks
        .iter()
        .find(|track| track.property == Property::Translation)
        .expect("rebased walk translation");
    assert_eq!(track.interpolation, Interpolation::CubicSpline);
    let TrackValues::Vec3s(values) = &track.values else {
        panic!("translation must remain VEC3");
    };
    let expected = [
        Vec3::new(12.0, 1.0, -1.0),
        Vec3::new(10.0, 100.0, 2.0),
        Vec3::new(13.0, 2.0, -2.0),
        Vec3::new(14.0, 3.0, -3.0),
        Vec3::new(10.0, 200.0, 4.0),
        Vec3::new(15.0, 4.0, -4.0),
    ];
    assert_eq!(values.len(), expected.len());
    for (actual, expected) in values.iter().zip(expected) {
        assert!(actual.abs_diff_eq(expected * 0.01, 1.0e-6));
    }
    let run = document
        .clips
        .iter()
        .find(|clip| clip.name == "run")
        .expect("independently scaled second clip");
    let run_track = run
        .tracks
        .iter()
        .find(|track| track.property == Property::Translation)
        .expect("rebased run translation");
    let TrackValues::Vec3s(run_values) = &run_track.values else {
        panic!("run translation must remain VEC3");
    };
    for (actual, expected) in run_values.iter().zip([
        Vec3::new(22.0, 1.0, -1.0),
        Vec3::new(20.0, 100.0, 2.0),
        Vec3::new(23.0, 2.0, -2.0),
        Vec3::new(24.0, 3.0, -3.0),
        Vec3::new(20.0, 200.0, 4.0),
        Vec3::new(25.0, 4.0, -4.0),
    ]) {
        assert!(actual.abs_diff_eq(expected * 0.01, 1.0e-6));
    }
    assert_eq!(evidence["clips"][2]["stripped_tracks"], 2);
    assert!(
        document.clips.iter().all(|clip| clip.name != "stripped"),
        "a fully stripped clip is omitted only at publication, not from evidence"
    );
}

#[test]
fn v5_composed_selector_removal_refuses_before_replacing_a_prior_pair() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
    write_cubic_asset(&dir.path().join("inputs/walk.glb"), 10.0);
    let recipe =
        recipe_v5("walk.glb").replace("remove_nodes = [\"attach\"]", "remove_nodes = [\"joint\"]");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();
    let prior_artifact = b"prior artifact";
    let prior_evidence = b"prior evidence";
    std::fs::write(dir.path().join("character.glb"), prior_artifact).unwrap();
    std::fs::write(dir.path().join("character.json"), prior_evidence).unwrap();

    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    assert!(
        refusal_detail(&output).contains("selected node"),
        "the removed selected joint is rejected at a located node: {}",
        refusal_detail(&output)
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        prior_artifact
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.json")).unwrap(),
        prior_evidence
    );
}

#[test]
fn v5_composed_artifact_matches_the_canonical_assembly_then_scale_pipeline() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
    write_cubic_asset(&dir.path().join("inputs/walk.glb"), 10.0);
    let composed_recipe = recipe_v5("walk.glb");
    std::fs::write(dir.path().join("recipe.toml"), &composed_recipe).unwrap();
    let composed = run(dir.path());
    assert!(
        composed.status.success(),
        "{}",
        String::from_utf8_lossy(&composed.stderr)
    );
    let composed_bytes = std::fs::read(dir.path().join("character.glb")).unwrap();

    let stage_recipe = composed_recipe.replace(
        "[rest_bind_scale]\nsource_skin_index = 0\nsource_root_node_index = 0\nexpected_factor = 0.01\n\n",
        "",
    );
    std::fs::write(dir.path().join("stage.toml"), stage_recipe).unwrap();
    let stage = run_to(dir.path(), "stage.toml", "stage.glb", "stage.json");
    assert!(
        stage.status.success(),
        "{}",
        String::from_utf8_lossy(&stage.stderr)
    );
    let scaled = Command::new(env!("CARGO_BIN_EXE_animsmith"))
        .current_dir(dir.path())
        .args([
            "scale",
            "rest-bind",
            "stage.glb",
            "-o",
            "two-stage.glb",
            "--source-skin-index",
            "0",
            "--source-root-node-index",
            "1",
            "--expected-factor=0.01",
            "--evidence",
            "two-stage.json",
            "--format",
            "json",
        ])
        .output()
        .expect("runs standalone rest-bind scale");
    assert!(
        scaled.status.success(),
        "{}",
        String::from_utf8_lossy(&scaled.stderr)
    );
    assert_eq!(
        std::fs::read(dir.path().join("two-stage.glb")).unwrap(),
        composed_bytes,
        "v5 preserves the exact geometry placement and grounding of the established two-stage path"
    );
}

#[test]
fn v5_without_rest_bind_scale_retains_ordinary_assembly() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    let bytes = rest_bind_scale_rig_glb();
    std::fs::write(dir.path().join("inputs/base.glb"), &bytes).unwrap();
    std::fs::write(dir.path().join("inputs/walk.glb"), &bytes).unwrap();
    let recipe = recipe_v5("walk.glb").replace(
        "[rest_bind_scale]\nsource_skin_index = 0\nsource_root_node_index = 0\nexpected_factor = 0.01\n\n",
        "",
    );
    std::fs::write(dir.path().join("recipe.toml"), &recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("character.json")).unwrap()).unwrap();
    assert_schema(&evidence, EVIDENCE_SCHEMA_V5);
    assert_eq!(evidence["schema_version"], 5);
    assert!(evidence.get("rest_bind_scale").is_none());
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    assert_eq!(document.skeleton.bones[0].rest.scale, Vec3::ONE);
    assert_eq!(document.clips.len(), 1);
    assert_eq!(document.clips[0].tracks.len(), 2);

    let legacy_recipe = recipe
        .replace("schema_version = 5", "schema_version = 4")
        .replace(
            "urn:animsmith:schema:character-assembly-recipe:5",
            "urn:animsmith:schema:character-assembly-recipe:4",
        );
    std::fs::write(dir.path().join("legacy.toml"), legacy_recipe).unwrap();
    let legacy = run_to(dir.path(), "legacy.toml", "legacy.glb", "legacy.json");
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert_eq!(
        std::fs::read(dir.path().join("character.glb")).unwrap(),
        std::fs::read(dir.path().join("legacy.glb")).unwrap(),
        "v5 without rest_bind_scale preserves the v4 ordinary-assembly artifact"
    );
}

#[test]
fn v5_rest_bind_scale_keeps_a_surviving_attachment_at_unit_world_scale() {
    let dir = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    write_cubic_asset(&dir.path().join("inputs/base.glb"), 0.0);
    write_cubic_asset(&dir.path().join("inputs/walk.glb"), 10.0);
    let recipe = recipe_v5("walk.glb").replace("remove_nodes = [\"attach\"]\n", "");
    std::fs::write(dir.path().join("recipe.toml"), recipe).unwrap();

    let output = run(dir.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document = animsmith_gltf::load(&dir.path().join("character.glb")).unwrap();
    let attachment = document
        .skeleton
        .bones
        .iter()
        .position(|bone| bone.name == "attach")
        .expect("unselected attachment survives");
    let world = rest_worlds(&document)[attachment];
    for axis in [world.x_axis, world.y_axis, world.z_axis] {
        assert!(
            (axis.truncate().length() - 1.0).abs() <= 1.0e-5,
            "surviving attachment has unit world scale"
        );
    }
}

#[test]
fn v4_rest_bind_recipe_fields_factors_and_conflicts_fail_closed() {
    let base = recipe("clip.glb");
    let cases = [
        (
            base.replace("source_skin_index = 0\n", ""),
            Some("missing field `source_skin_index`"),
        ),
        (
            base.replace("source_root_node_index = 0\n", ""),
            Some("missing field `source_root_node_index`"),
        ),
        (
            base.replace("expected_factor = 0.01\n", ""),
            Some("missing field `expected_factor`"),
        ),
        (
            base.replace("expected_factor = 0.01", "expected_factor = 0.0"),
            Some("must be finite and greater than zero"),
        ),
        (
            base.replace("expected_factor = 0.01", "expected_factor = nan"),
            Some("must be finite and greater than zero"),
        ),
        (
            base.replacen("fps = 30.0", "fps = 30.0\ncanonicalize_skin = true", 1),
            Some("cannot be combined with canonicalize_skin, ground_and_center, or remove_nodes"),
        ),
        (
            base.replacen(
                "fps = 30.0",
                "fps = 30.0\ncanonicalize_skin = true\nground_and_center = true",
                1,
            ),
            Some("cannot be combined with canonicalize_skin, ground_and_center, or remove_nodes"),
        ),
        (
            base.replacen("fps = 30.0", "fps = 30.0\nremove_nodes = [\"joint\"]", 1),
            Some("cannot be combined with canonicalize_skin, ground_and_center, or remove_nodes"),
        ),
        (
            base.replace(
                "urn:animsmith:schema:character-assembly-recipe:4",
                "urn:animsmith:schema:character-assembly-recipe:3",
            ),
            Some("unsupported assembly recipe identity"),
        ),
    ];
    for (ordinal, (invalid, expected)) in cases.into_iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("inputs")).unwrap();
        let bytes = rest_bind_scale_rig_glb();
        std::fs::write(dir.path().join("inputs/base.glb"), &bytes).unwrap();
        std::fs::write(dir.path().join("inputs/clip.glb"), &bytes).unwrap();
        std::fs::write(dir.path().join("recipe.toml"), invalid).unwrap();
        let output = run(dir.path());
        assert_eq!(output.status.code(), Some(2), "case {ordinal}");
        if let Some(expected) = expected {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(expected),
                "case {ordinal}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(!dir.path().join("character.glb").exists());
        assert!(!dir.path().join("character.json").exists());
    }
}

fn assert_scale_refusal(recipe_text: &str, base: &[u8], clip: &[u8], expected: &str) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("inputs")).unwrap();
    std::fs::write(dir.path().join("inputs/base.glb"), base).unwrap();
    std::fs::write(dir.path().join("inputs/clip.gltf"), clip).unwrap();
    std::fs::write(dir.path().join("recipe.toml"), recipe_text).unwrap();
    let output = run(dir.path());
    assert_eq!(output.status.code(), Some(1));
    let detail = refusal_detail(&output);
    assert!(detail.contains(expected), "expected {expected:?}: {detail}");
    assert!(!dir.path().join("character.glb").exists());
    assert!(!dir.path().join("character.json").exists());
}

#[test]
fn v4_public_selector_factor_topology_and_real_helper_mismatches_fail_closed() {
    let base = rest_bind_scale_rig_glb();
    let valid_clip = rest_bind_scale_rig_gltf();
    assert_scale_refusal(
        &recipe("clip.gltf").replace("source_skin_index = 0", "source_skin_index = 9"),
        &base,
        &valid_clip,
        "source skin index 9 is not a skin",
    );

    let mut factor: Value = serde_json::from_slice(&valid_clip).unwrap();
    factor["nodes"][0]["scale"] = serde_json::json!([0.02, 0.02, 0.02]);
    assert_scale_refusal(
        &recipe("clip.gltf"),
        &base,
        &serde_json::to_vec(&factor).unwrap(),
        "expected factor",
    );

    let mut topology: Value = serde_json::from_slice(&valid_clip).unwrap();
    topology["nodes"][0]["children"] = serde_json::json!([]);
    topology["scenes"][0]["nodes"] = serde_json::json!([0, 1, 3]);
    assert_scale_refusal(
        &recipe("clip.gltf"),
        &base,
        &serde_json::to_vec(&topology).unwrap(),
        "joint_not_descendant_of_scaled_root",
    );

    let mut helper: Value = serde_json::from_slice(&valid_clip).unwrap();
    helper["nodes"][0]["children"] = serde_json::json!([4]);
    helper["nodes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
                       0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            "children": [1]
        }));
    assert_scale_refusal(
        &recipe("clip.gltf"),
        &base,
        &serde_json::to_vec(&helper).unwrap(),
        "named-topology",
    );
}
