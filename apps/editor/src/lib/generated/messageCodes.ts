// このファイルは生成物です。手で編集せず `just gen` で作り直してください (docs/editor.md#型の生成)。
// バックエンドが画面に出すエラー・警告のコード → 引数の名前 (docs/editor.md#エラーと警告のコード)。

export const messageCodes: Record<string, readonly string[]> = {
  "ai.api_error": [
    "detail",
    "kind",
    "status"
  ],
  "ai.api_key_required": [],
  "ai.cancelled": [],
  "ai.claude_model_missing": [],
  "ai.credential_store": [
    "detail"
  ],
  "ai.empty_api_key": [],
  "ai.invalid_base_url": [],
  "ai.invalid_proposal": [
    "detail"
  ],
  "ai.invalid_request": [
    "detail"
  ],
  "ai.invalid_response": [
    "detail"
  ],
  "ai.invalid_time_limit": [
    "max",
    "min"
  ],
  "ai.missing_api_key": [
    "detail"
  ],
  "ai.mixed_destinations": [],
  "ai.model_missing": [],
  "ai.network": [
    "detail"
  ],
  "ai.plain_http_api_key": [],
  "ai.provider": [
    "detail"
  ],
  "ai.rate_limited": [
    "retry_after"
  ],
  "ai.refused": [
    "category"
  ],
  "ai.send_disabled": [],
  "ai.time_limit": [
    "minutes"
  ],
  "ai.truncated": [],
  "atlas.invalid_max_size": [],
  "atlas.missing_texture": [
    "name"
  ],
  "atlas.not_png": [
    "name"
  ],
  "atlas.texture_out_of_range": [
    "mesh",
    "texture"
  ],
  "atlas.texture_too_large": [
    "height",
    "max",
    "name",
    "width"
  ],
  "avatar.duplicate_hit_area_name": [
    "name"
  ],
  "avatar.internal_tracking_target": [
    "field",
    "id"
  ],
  "avatar.invalid_level": [
    "level",
    "max"
  ],
  "avatar.invalid_value": [
    "expected",
    "field",
    "value"
  ],
  "avatar.missing_metadata": [
    "field"
  ],
  "avatar.missing_parameter": [
    "id",
    "level"
  ],
  "avatar.missing_profile": [],
  "avatar.missing_tracking_target": [
    "field"
  ],
  "avatar.newer_profile_version": [
    "supported",
    "version"
  ],
  "avatar.parameter_range": [
    "default",
    "expected_default",
    "expected_max",
    "expected_min",
    "id",
    "max",
    "min"
  ],
  "avatar.too_few_extended_parameters": [
    "found",
    "required"
  ],
  "avatar.unknown_reference": [
    "field",
    "target",
    "target_kind"
  ],
  "avatar.unknown_tracking_input": [
    "field",
    "input",
    "kind"
  ],
  "avatar.unknown_value": [
    "field"
  ],
  "cell.duplicate_key": [
    "parameter",
    "value"
  ],
  "cell.keyform_count": [
    "actual",
    "expected"
  ],
  "cell.missing_axis": [
    "parameter"
  ],
  "cell.non_finite_key": [
    "parameter",
    "value"
  ],
  "cell.out_of_range": [
    "index",
    "len"
  ],
  "cell.shape_mismatch": [
    "actual",
    "expected"
  ],
  "cell.unknown_axis": [
    "parameter"
  ],
  "cell.unknown_key": [
    "parameter",
    "value"
  ],
  "compile.invalid_model": [
    "count",
    "details"
  ],
  "compile.unknown_parameter": [
    "parameter"
  ],
  "compile.unsupported": [
    "feature"
  ],
  "edit.automesh_failed": [
    "detail",
    "id"
  ],
  "edit.cannot_regenerate_mesh": [
    "id",
    "reason"
  ],
  "edit.deformer_edit_failed": [
    "reason"
  ],
  "edit.deformer_kind_mismatch": [
    "expected",
    "id"
  ],
  "edit.depth_length": [
    "actual",
    "expected",
    "target"
  ],
  "edit.depth_point_out_of_range": [
    "index",
    "points",
    "target"
  ],
  "edit.draw_clone_shape": [
    "id",
    "original"
  ],
  "edit.duplicate_id": [
    "id",
    "kind"
  ],
  "edit.has_children": [
    "children",
    "id",
    "kind"
  ],
  "edit.inference_failed": [
    "detail",
    "target"
  ],
  "edit.invalid_avatar_profile": [
    "reason"
  ],
  "edit.invalid_canvas": [
    "height",
    "width"
  ],
  "edit.invalid_cell": [
    "id",
    "kind"
  ],
  "edit.invalid_draw_order": [
    "draw_order",
    "id"
  ],
  "edit.invalid_expression": [
    "expression",
    "reason"
  ],
  "edit.invalid_glue": [
    "id",
    "reason"
  ],
  "edit.invalid_model": [
    "count",
    "details"
  ],
  "edit.invalid_motion": [
    "motion",
    "reason"
  ],
  "edit.invalid_name": [
    "kind",
    "name"
  ],
  "edit.invalid_normal_map": [
    "reason",
    "texture"
  ],
  "edit.invalid_physics": [
    "reason"
  ],
  "edit.invalid_range": [
    "default",
    "max",
    "min"
  ],
  "edit.invalid_slot": [
    "id",
    "reason"
  ],
  "edit.invalid_texture_name": [
    "name"
  ],
  "edit.keyform_kind_mismatch": [
    "actual",
    "expected",
    "id",
    "kind"
  ],
  "edit.mesh_geometry": [
    "id",
    "reason"
  ],
  "edit.mirror_pair_not_supported": [
    "target"
  ],
  "edit.module_shape": [
    "reason"
  ],
  "edit.non_finite_depth": [
    "target",
    "value"
  ],
  "edit.not_draw_clone": [
    "id"
  ],
  "edit.not_found": [
    "id",
    "kind"
  ],
  "edit.not_module": [],
  "edit.read_only_base": [
    "element"
  ],
  "edit.self_reference": [
    "field",
    "id",
    "kind"
  ],
  "edit.texture_not_png": [
    "name"
  ],
  "edit.texture_too_large": [
    "height",
    "max",
    "texture",
    "width"
  ],
  "edit.too_few_vertices": [
    "id",
    "remaining"
  ],
  "edit.unsupported_inference": [
    "target"
  ],
  "edit.unsupported_interpolation": [
    "interpolation",
    "target"
  ],
  "edit.vertex_out_of_range": [
    "id",
    "index",
    "vertices"
  ],
  "editor.create_folder": [
    "detail",
    "path"
  ],
  "editor.empty_path": [],
  "editor.expected_module_bytes": [],
  "editor.expected_png_bytes": [],
  "editor.external_change": [],
  "editor.internal": [
    "detail"
  ],
  "editor.invalid_header": [
    "detail",
    "header"
  ],
  "editor.invalid_percent_encoding": [
    "detail"
  ],
  "editor.io": [
    "detail",
    "path"
  ],
  "editor.lock_poisoned": [
    "what"
  ],
  "editor.missing_header": [
    "header"
  ],
  "editor.model_changed": [],
  "editor.not_a_module": [],
  "editor.not_json_file": [
    "path"
  ],
  "editor.not_module_file": [
    "path"
  ],
  "editor.not_png_data": [],
  "editor.not_png_file": [
    "path"
  ],
  "editor.not_saved": [],
  "editor.not_saved_use_save_as": [],
  "editor.relative_path": [
    "path"
  ],
  "editor.task_failed": [
    "detail"
  ],
  "editor.too_many_sample_folders": [
    "path"
  ],
  "export.invalid_max_size": [
    "max",
    "value"
  ],
  "export.invalid_padding": [
    "max",
    "value"
  ],
  "export.invalid_settings": [
    "detail"
  ],
  "format.chunk_limit_exceeded": [
    "chunk",
    "max"
  ],
  "format.corrupt": [
    "detail"
  ],
  "format.invalid_expression": [
    "count",
    "details",
    "name"
  ],
  "format.invalid_model": [
    "count",
    "details"
  ],
  "format.invalid_module": [
    "count",
    "details"
  ],
  "format.invalid_motion": [
    "count",
    "details",
    "name"
  ],
  "format.invalid_physics": [
    "count",
    "details"
  ],
  "format.not_ugoe_file": [],
  "format.too_large": [],
  "format.total_limit_exceeded": [
    "chunk",
    "max"
  ],
  "format.unknown_values": [
    "values"
  ],
  "format.unsupported_features": [
    "features"
  ],
  "format.unsupported_version": [
    "found",
    "supported"
  ],
  "inference.mirror_failed": [
    "detail",
    "source",
    "target"
  ],
  "lighting.empty_group": [
    "group"
  ],
  "lighting.generate_failed": [
    "detail"
  ],
  "lighting.image_not_found": [
    "name"
  ],
  "lighting.invalid_preview": [
    "reason"
  ],
  "lighting.invalid_settings": [
    "detail"
  ],
  "lighting.no_group_chosen": [],
  "lighting.normal_map_too_large": [
    "height",
    "max",
    "width"
  ],
  "lighting.normal_maps_model_only": [],
  "lighting.texture_not_found": [
    "texture"
  ],
  "lighting.texture_too_large": [
    "height",
    "max",
    "texture",
    "width"
  ],
  "lighting.unknown_group": [
    "group"
  ],
  "lighting.unreadable_normal_map": [],
  "lighting.unreadable_texture": [
    "texture"
  ],
  "mcp.bridge_start_failed": [
    "detail"
  ],
  "mcp.bundle_write_failed": [
    "detail"
  ],
  "mcp.invalid_port": [],
  "mcp.no_binary": [],
  "mcp.not_mcpb_file": [],
  "mcp.open_failed": [
    "detail",
    "path"
  ],
  "mcp.unsupported_platform": [],
  "module.collider_needs_base_physics": [
    "collider",
    "rig",
    "slot"
  ],
  "module.compose_failed": [
    "detail"
  ],
  "module.draw_order_outside_slot": [
    "mesh"
  ],
  "module.invalid_name": [
    "name"
  ],
  "module.read_only_base": [
    "element"
  ],
  "module.shape": [
    "reason"
  ],
  "module.unknown_slot": [
    "slot"
  ],
  "overflow.unknown_parameter": [
    "parameter",
    "target"
  ],
  "project.base_is_module": [
    "path"
  ],
  "project.base_not_found": [
    "path"
  ],
  "project.base_refused": [
    "path",
    "reason"
  ],
  "project.base_unreadable": [
    "path"
  ],
  "project.duplicate_name": [
    "kind",
    "name"
  ],
  "project.export_failed": [],
  "project.format": [
    "path"
  ],
  "project.invalid_module": [
    "count",
    "details"
  ],
  "project.invalid_name": [
    "kind",
    "name"
  ],
  "project.io": [
    "detail",
    "path"
  ],
  "project.json": [
    "detail",
    "path"
  ],
  "project.missing_module_link": [],
  "project.missing_texture": [
    "name"
  ],
  "project.not_a_module": [],
  "project.not_a_project": [
    "path"
  ],
  "project.not_empty": [
    "path"
  ],
  "project.not_png": [
    "path"
  ],
  "project.unsupported_base_texture": [
    "name"
  ],
  "project.unsupported_texture": [
    "name"
  ],
  "project.unsupported_version": [
    "found",
    "supported"
  ],
  "proposal.disappeared": [],
  "proposal.invalid_index": [
    "index"
  ],
  "proposal.model_changed": [],
  "proposal.no_preview": [],
  "proposal.no_psd": [],
  "proposal.not_classified": [],
  "proposal.not_found": [
    "id"
  ],
  "proposal.not_regenerable": [
    "id"
  ],
  "proposal.nothing_selected": [],
  "proposal.rejected": [],
  "proposal.stale": [],
  "psd.adjustment_layer": [
    "key"
  ],
  "psd.automesh_failed": [
    "detail",
    "layer"
  ],
  "psd.blend_mode_unsupported": [
    "blend_mode"
  ],
  "psd.canvas_mismatch": [
    "model_height",
    "model_width",
    "psd_height",
    "psd_width"
  ],
  "psd.clipping_base_missing": [],
  "psd.empty_layer": [],
  "psd.flattened_image": [],
  "psd.group_blend_mode_ignored": [
    "blend_mode"
  ],
  "psd.group_clipping_unsupported": [],
  "psd.invalid_mesh_options": [
    "reason"
  ],
  "psd.layer_effects_ignored": [],
  "psd.layer_mask_ignored": [],
  "psd.layer_too_complex": [],
  "psd.missing_pixels": [],
  "psd.read_failed": [
    "detail"
  ],
  "psd.read_file_failed": [
    "detail",
    "path"
  ],
  "psd.smart_object_without_pixels": [],
  "psd.transparent_layer": [],
  "psd.unsupported_compression": [
    "compression"
  ],
  "space.evaluate_failed": [
    "detail"
  ],
  "space.length_mismatch": [
    "guesses",
    "points"
  ],
  "space.no_space": [
    "id",
    "kind"
  ],
  "space.not_converged": [
    "index",
    "residual"
  ],
  "space.not_found": [
    "id",
    "kind"
  ],
  "space.singular": [
    "index"
  ]
};
