//! コマンドの JSON の形のテスト。

use super::*;

/// バリアント名 (`op` の値)。バリアントを追加すると、ここと `json_samples` の更新をコンパイラが求める。
fn op_name(command: &Command) -> &'static str {
    match command {
        Command::AddParameter { .. } => "add_parameter",
        Command::RemoveParameter { .. } => "remove_parameter",
        Command::SetParameterRange { .. } => "set_parameter_range",
        Command::Batch { .. } => "batch",
        Command::RenameParameter { .. } => "rename_parameter",
        Command::SetParameterGroup { .. } => "set_parameter_group",
        Command::SetParameterMirror { .. } => "set_parameter_mirror",
        Command::AddPart { .. } => "add_part",
        Command::RemovePart { .. } => "remove_part",
        Command::RenamePart { .. } => "rename_part",
        Command::SetPartParent { .. } => "set_part_parent",
        Command::SetPartKeyforms { .. } => "set_part_keyforms",
        Command::AddDeformer { .. } => "add_deformer",
        Command::RemoveDeformer { .. } => "remove_deformer",
        Command::RenameDeformer { .. } => "rename_deformer",
        Command::SetDeformerParent { .. } => "set_deformer_parent",
        Command::SetDeformerPart { .. } => "set_deformer_part",
        Command::SetWarpKeyforms { .. } => "set_warp_keyforms",
        Command::SetRotationKeyforms { .. } => "set_rotation_keyforms",
        Command::AddMesh { .. } => "add_mesh",
        Command::RemoveMesh { .. } => "remove_mesh",
        Command::RenameMesh { .. } => "rename_mesh",
        Command::SetMeshProperties { .. } => "set_mesh_properties",
        Command::SetMeshParent { .. } => "set_mesh_parent",
        Command::SetMeshKeyforms { .. } => "set_mesh_keyforms",
        Command::SetMeshKeyform { .. } => "set_mesh_keyform",
        Command::ReorderMesh { .. } => "reorder_mesh",
        Command::AddTexture { .. } => "add_texture",
        Command::SetTextureImage { .. } => "set_texture_image",
        Command::RemoveTexture { .. } => "remove_texture",
        Command::SetCanvas { .. } => "set_canvas",
        Command::RegenerateMesh { .. } => "regenerate_mesh",
        Command::InferKeyforms { .. } => "infer_keyforms",
        Command::MirrorKeyforms { .. } => "mirror_keyforms",
        Command::MarkKeyformAuthored { .. } => "mark_keyform_authored",
        Command::MarkKeyformDerived { .. } => "mark_keyform_derived",
        Command::MoveMeshVertices { .. } => "move_mesh_vertices",
        Command::AddMeshVertex { .. } => "add_mesh_vertex",
        Command::RemoveMeshVertices { .. } => "remove_mesh_vertices",
        Command::RetriangulateMesh { .. } => "retriangulate_mesh",
        Command::AddKeyformKey { .. } => "add_keyform_key",
        Command::RemoveKeyformKey { .. } => "remove_keyform_key",
        Command::SetKeyformCell { .. } => "set_keyform_cell",
        Command::SetMeshDrawOrder { .. } => "set_mesh_draw_order",
        Command::CreateWarpDeformer { .. } => "create_warp_deformer",
        Command::CreateRotationDeformer { .. } => "create_rotation_deformer",
        Command::ReparentPreservingLook { .. } => "reparent_preserving_look",
        Command::SetKeyformInference { .. } => "set_keyform_inference",
        Command::SetMirrorPair { .. } => "set_mirror_pair",
        Command::SetDepth { .. } => "set_depth",
        Command::ApplyDepthPreset { .. } => "apply_depth_preset",
        Command::MoveRotationOrigin { .. } => "move_rotation_origin",
        Command::UnwrapDeformer { .. } => "unwrap_deformer",
        Command::AddMotion { .. } => "add_motion",
        Command::RemoveMotion { .. } => "remove_motion",
        Command::RenameMotion { .. } => "rename_motion",
        Command::SetMotionProperties { .. } => "set_motion_properties",
        Command::AddMotionCurve { .. } => "add_motion_curve",
        Command::RemoveMotionCurve { .. } => "remove_motion_curve",
        Command::SetMotionCurveFade { .. } => "set_motion_curve_fade",
        Command::SetMotionKey { .. } => "set_motion_key",
        Command::MoveMotionKey { .. } => "move_motion_key",
        Command::RemoveMotionKey { .. } => "remove_motion_key",
        Command::SetMotionSegment { .. } => "set_motion_segment",
        Command::SetMotionEvents { .. } => "set_motion_events",
        Command::AddExpression { .. } => "add_expression",
        Command::RemoveExpression { .. } => "remove_expression",
        Command::RenameExpression { .. } => "rename_expression",
        Command::SetExpressionProperties { .. } => "set_expression_properties",
        Command::SetExpressionParameters { .. } => "set_expression_parameters",
        Command::SetPhysicsWorld { .. } => "set_physics_world",
        Command::AddPhysicsRig { .. } => "add_physics_rig",
        Command::CreatePhysicsRig { .. } => "create_physics_rig",
        Command::RemovePhysicsRig { .. } => "remove_physics_rig",
        Command::RenamePhysicsRig { .. } => "rename_physics_rig",
        Command::SetPhysicsInputs { .. } => "set_physics_inputs",
        Command::SetPhysicsOutputs { .. } => "set_physics_outputs",
        Command::SetPhysicsParticles { .. } => "set_physics_particles",
        Command::SetPhysicsNormalization { .. } => "set_physics_normalization",
        Command::SetPhysicsRigColliders { .. } => "set_physics_rig_colliders",
        Command::SetPhysicsFrame { .. } => "set_physics_frame",
        Command::AddPhysicsCollider { .. } => "add_physics_collider",
        Command::RemovePhysicsCollider { .. } => "remove_physics_collider",
        Command::SetPhysicsCollider { .. } => "set_physics_collider",
        Command::SetKeyformInterpolation { .. } => "set_keyform_interpolation",
        Command::ReorderPhysicsRig { .. } => "reorder_physics_rig",
        Command::SetParameterInternal { .. } => "set_parameter_internal",
        Command::CreateDrawClone { .. } => "create_draw_clone",
        Command::SyncDrawClone { .. } => "sync_draw_clone",
        Command::ApplyPhysicsTemplate { .. } => "apply_physics_template",
        Command::SetAvatarProfile { .. } => "set_avatar_profile",
        Command::UpdateAvatarProfile { .. } => "update_avatar_profile",
        Command::SetHitAreas { .. } => "set_hit_areas",
        Command::SetNormalMap { .. } => "set_normal_map",
        Command::SetLighting { .. } => "set_lighting",
        Command::SetEffects { .. } => "set_effects",
        Command::AddSlot { .. } => "add_slot",
        Command::SetSlot { .. } => "set_slot",
        Command::RemoveSlot { .. } => "remove_slot",
        Command::SetModuleSettings { .. } => "set_module_settings",
        Command::SetPose { .. } => "set_pose",
        Command::CreateGlue { .. } => "create_glue",
        Command::SetGlue { .. } => "set_glue",
        Command::RemoveGlue { .. } => "remove_glue",
    }
}

const OP_COUNT: usize = 104;

fn json_samples() -> Vec<Command> {
    let quad = fixtures::single_quad();
    let DeformerKind::Warp(cloth) = rotation_chain_deformer("Cloth").kind else {
        panic!("Cloth is a warp deformer");
    };
    let DeformerKind::Rotation(arm) = rotation_chain_deformer("Arm").kind else {
        panic!("Arm is a rotation deformer");
    };
    let mesh_keyforms = quad.meshes[0].keyforms.clone();
    vec![
        Command::AddParameter {
            parameter: new_parameter("Y", -1.0, 0.0, 1.0),
        },
        Command::RemoveParameter { id: "X".into() },
        Command::SetParameterRange {
            id: "X".into(),
            min: -2.0,
            default: 0.5,
            max: 2.0,
        },
        Command::Batch {
            label: "batch".into(),
            commands: vec![Command::RemoveParameter { id: "X".into() }],
        },
        Command::RenameParameter {
            id: "X".into(),
            name: "Horizontal".into(),
        },
        Command::SetParameterGroup {
            id: "X".into(),
            group: Some("Face".into()),
        },
        Command::SetParameterMirror {
            id: "X".into(),
            mirror: None,
        },
        Command::AddPart {
            part: new_part("Head", Some("Root")),
        },
        Command::RemovePart { id: "Head".into() },
        Command::RenamePart {
            id: "Root".into(),
            name: "Body".into(),
        },
        Command::SetPartParent {
            id: "Head".into(),
            parent: Some("Root".into()),
        },
        Command::SetPartKeyforms {
            id: "Root".into(),
            keyforms: quad.parts[0].keyforms.clone(),
        },
        Command::AddDeformer {
            deformer: rotation_chain_deformer("Hand"),
        },
        Command::RemoveDeformer { id: "Hand".into() },
        Command::RenameDeformer {
            id: "Hand".into(),
            name: "Wrist".into(),
        },
        Command::SetDeformerParent {
            id: "Hand".into(),
            parent: None,
        },
        Command::SetDeformerPart {
            id: "Hand".into(),
            part: "Root".into(),
        },
        Command::SetWarpKeyforms {
            id: "Cloth".into(),
            keyforms: cloth.keyforms,
        },
        Command::SetRotationKeyforms {
            id: "Arm".into(),
            keyforms: arm.keyforms,
        },
        Command::AddMesh {
            mesh: new_mesh("Quad2"),
        },
        Command::RemoveMesh { id: "Quad".into() },
        Command::RenameMesh {
            id: "Quad".into(),
            name: "Square".into(),
        },
        Command::SetMeshProperties {
            id: "Quad".into(),
            part: None,
            texture: None,
            texture_name: Some("white".into()),
            blend: Some(BlendMode::Additive),
            culling: None,
            masks: Some(vec![MaskRef::inverted("Other")]),
            mask_mode: None,
            mask_only: Some(false),
            lit: None,
        },
        Command::SetMeshParent {
            id: "Quad".into(),
            parent: Some("Warp".into()),
        },
        Command::SetMeshKeyform {
            id: "Quad".into(),
            cell: cell(&[("X", 1.0)]),
            keyform: mesh_keyforms.keyforms[2].clone(),
        },
        Command::SetMeshKeyforms {
            id: "Quad".into(),
            keyforms: mesh_keyforms,
        },
        Command::ReorderMesh {
            id: "Quad".into(),
            before: Some("Other".into()),
        },
        Command::AddTexture {
            name: "hair".into(),
            png: crate::texture::solid_png(1, 1, [0; 4]),
        },
        Command::SetTextureImage {
            name: "white".into(),
            png: crate::texture::solid_png(1, 1, [255; 4]),
        },
        Command::RemoveTexture {
            name: "white".into(),
        },
        Command::SetCanvas {
            width: 512.0,
            height: 256.0,
            origin: ugoe_core::Vec2::new(256.0, 200.0),
        },
        Command::RegenerateMesh {
            id: "Quad".into(),
            options: crate::AutoMeshOptions {
                edge_spacing: 12.0,
                ..crate::AutoMeshOptions::default()
            },
        },
        Command::InferKeyforms {
            target: crate::KeyformTarget::Deformer("Face".into()),
            method: crate::InferMethod::DepthRotation,
            overwrite_derived: true,
        },
        Command::MirrorKeyforms {
            source: crate::KeyformTarget::Mesh("EyeL".into()),
            target: crate::KeyformTarget::Mesh("EyeR".into()),
            axis: Some(0.5),
            tolerance: Some(0.01),
            overwrite_authored: true,
        },
        Command::MarkKeyformAuthored {
            target: crate::KeyformTarget::Part("Root".into()),
            cells: vec![cell(&[("X", 1.0)])],
        },
        Command::MarkKeyformDerived {
            target: crate::KeyformTarget::Glue("Glue".into()),
            cells: vec![KeyformCell::new()],
        },
        Command::MoveMeshVertices {
            id: "Quad".into(),
            moves: vec![crate::VertexMove {
                index: 2,
                delta: ugoe_core::Vec2::new(1.0, -2.0),
            }],
        },
        Command::AddMeshVertex {
            id: "Quad".into(),
            position: ugoe_core::Vec2::new(100.0, 120.0),
        },
        Command::RemoveMeshVertices {
            id: "Quad".into(),
            indices: vec![1, 3],
        },
        Command::RetriangulateMesh { id: "Quad".into() },
        Command::AddKeyformKey {
            target: crate::KeyformTarget::Mesh("Quad".into()),
            parameter: "X".into(),
            key: 0.5,
        },
        Command::RemoveKeyformKey {
            target: crate::KeyformTarget::Deformer("Arm".into()),
            parameter: "Angle".into(),
            key: 30.0,
        },
        Command::SetKeyformCell {
            target: crate::KeyformTarget::Glue("Seam".into()),
            cell: cell(&[]),
            keyform: crate::Keyform::Glue(ugoe_model::mesh::GlueKeyform { intensity: 0.5 }),
        },
        Command::SetMeshDrawOrder {
            id: "Quad".into(),
            draw_order: 250.0,
        },
        Command::CreateWarpDeformer {
            id: None,
            name: "Warp".into(),
            children: vec![ChildRef::Mesh("Quad".into())],
            parts: Vec::new(),
            rows: 2,
            cols: 2,
            parent: None,
        },
        Command::CreateRotationDeformer {
            id: Some("Rot".into()),
            name: "Rot".into(),
            children: vec![ChildRef::Deformer("Arm".into())],
            parts: Vec::new(),
            origin: Some(ugoe_core::Vec2::new(1.0, 2.0)),
            parent: Some("Body".into()),
        },
        Command::ReparentPreservingLook {
            element: ChildRef::Deformer("Arm".into()),
            new_parent: Some("Body".into()),
        },
        Command::SetKeyformInference {
            target: crate::KeyformTarget::Mesh("Quad".into()),
            inference: Some(ugoe_model::KeyformInference::Additive),
        },
        Command::SetMirrorPair {
            target: crate::KeyformTarget::Mesh("Quad".into()),
            mirror: Some("Quad".into()),
            axis: Some(0.0),
        },
        Command::SetDepth {
            target: ChildRef::Mesh("Quad".into()),
            depth: Some(vec![0.0, 1.0, 2.0, 3.0]),
        },
        Command::ApplyDepthPreset {
            target: ChildRef::Deformer("Cloth".into()),
            preset: crate::DepthPreset::Dome { peak: 12.0 },
        },
        Command::MoveRotationOrigin {
            id: "Arm".into(),
            delta: ugoe_core::Vec2::new(4.0, -2.0),
        },
        Command::UnwrapDeformer { id: "Arm".into() },
        Command::SetKeyformInterpolation {
            target: crate::KeyformTarget::Mesh("Quad".into()),
            blend_shape: None,
            interpolation: ugoe_model::KeyformInterpolation::Smooth,
        },
        Command::SetKeyformInterpolation {
            target: crate::KeyformTarget::Mesh("Mouth".into()),
            blend_shape: Some("Smile".into()),
            interpolation: ugoe_model::KeyformInterpolation::Linear,
        },
    ]
    .into_iter()
    .chain(crate::codegen::motion_sample_commands())
    .chain(crate::codegen::physics_samples())
    .chain(crate::codegen::draw_clone_samples())
    .chain(crate::codegen::glue_samples())
    .chain(crate::codegen::avatar_samples())
    .chain(crate::codegen::lighting_samples())
    .chain(crate::codegen::slot_samples())
    .collect()
}

#[test]
fn every_variant_roundtrips_json() {
    let samples = json_samples();
    let mut ops: Vec<_> = samples.iter().map(op_name).collect();
    ops.sort_unstable();
    ops.dedup();
    assert_eq!(ops.len(), OP_COUNT, "json_samples must cover every variant");
    for command in samples {
        let json = serde_json::to_value(&command).unwrap();
        assert_eq!(json["op"], op_name(&command));
        let back: Command = serde_json::from_value(json).unwrap();
        assert_eq!(back, command);
    }
}

#[test]
fn json_uses_ids_for_keyform_cells() {
    let cmd: Command = serde_json::from_str(
        r#"{"op":"set_mesh_keyform","id":"Quad","cell":{"X":1},
            "keyform":{"positions":[[0,0],[1,0],[1,1],[0,1]],"opacity":1,"draw_order":500}}"#,
    )
    .unwrap();
    let Command::SetMeshKeyform { cell: c, .. } = &cmd else {
        panic!("unexpected command {cmd:?}");
    };
    assert_eq!(c, &cell(&[("X", 1.0)]));
}
