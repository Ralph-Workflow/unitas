use super::compat_catalog::owning_feature_tier;
use super::{CompatibilityState, Tier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OwnedCapabilityPart {
    pub behavior: &'static str,
    pub location: &'static str,
    pub evidence: &'static [&'static str],
    pub missing: Option<&'static str>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OwnedFeatureCapability {
    pub feature_id: &'static str,
    pub owning_spec: &'static str,
    pub capability: &'static str,
    pub parts: &'static [OwnedCapabilityPart],
}

impl OwnedFeatureCapability {
    pub fn state(&self) -> CompatibilityState {
        if self.parts.is_empty() {
            return CompatibilityState::Unsupported;
        }
        let demonstrated = self
            .parts
            .iter()
            .filter(|part| !part.evidence.is_empty())
            .count();
        if demonstrated == self.parts.len() {
            CompatibilityState::Supported
        } else if demonstrated == 0 {
            CompatibilityState::Unsupported
        } else {
            CompatibilityState::PartiallySupported
        }
    }

    pub fn missing_parts(&self) -> Vec<&'static OwnedCapabilityPart> {
        self.parts
            .iter()
            .filter(|part| part.evidence.is_empty())
            .collect()
    }

    pub fn gap(&self) -> Option<String> {
        let missing = self.missing_parts();
        if missing.is_empty() {
            return None;
        }
        let missing_text: Vec<String> = missing
            .iter()
            .map(|part| {
                format!(
                    "{} ({})",
                    part.missing.unwrap_or(part.behavior),
                    part.location
                )
            })
            .collect();
        match self.state() {
            CompatibilityState::PartiallySupported => {
                let provided: Vec<String> = self
                    .parts
                    .iter()
                    .filter(|part| !part.evidence.is_empty())
                    .map(|part| format!("{} ({})", part.behavior, part.location))
                    .collect();
                Some(format!(
                    "Partially supported: {}; missing: {}",
                    provided.join("; "),
                    missing_text.join("; ")
                ))
            }
            _ => Some(format!("Unsupported: {}", missing_text.join("; "))),
        }
    }
}

pub const OWNED_FEATURE_CAPABILITIES: &[OwnedFeatureCapability] = &[
    OwnedFeatureCapability {
        feature_id: "PRJ-001",
        owning_spec: "project-and-assets.md",
        capability: "Open and recognize existing Unity projects",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the project root is recognized and its asset inventory, GUID index, and duplicate GUID report are built",
                location: "crates/unitas-asset/src/project.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/project_tests.rs::open_report_enumerates_assets_packages_settings",
                    "crates/unitas-asset/tests/asset_import/project_tests.rs::duplicate_guids_are_reported_not_resolved",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the declared Unity version is read and an earlier release is reported as migrated with an upgrade preview",
                location: "crates/unitas-asset/src/project.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/project_tests.rs::project_open_2021_3_reports_migration_without_writing",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-002",
        owning_spec: "project-and-assets.md",
        capability: "Browse, search, label, preview, move, and delete assets",
        parts: &[
            OwnedCapabilityPart {
                behavior: "assets are browsed and filtered by name, class, label, folder, and import state",
                location: "crates/unitas-asset/src/database.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::search_combines_filters_and_distinguishes_sub_assets",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "assets are renamed, moved, deleted, and restored",
                location: "crates/unitas-asset/src/database.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::move_preserves_guid_and_reference",
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::delete_exposes_missing_reference",
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::restore_re_resolves_reference",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a preview, favorites, the recent list, duplication, and undo are provided",
                location: "crates/unitas-asset/src/database.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::database_preview_favorites_recents_duplicate_and_undo_redo_round_trip",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the editor ABI exposes the asset search, preview, move, delete, restore, duplicate, favorites, and recents tools",
                location: "crates/unitas-editor-abi/src/asset_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::assets_search_lists_project_assets_and_filters_by_label",
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::assets_preview_reports_class_size_and_state",
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::assets_move_delete_restore_and_duplicate_keep_the_project_consistent",
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::assets_favorites_and_recents_track_the_open_project",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-003",
        owning_spec: "serialization-and-prefabs.md",
        capability: "Preserve GUID, local identifier, and sub-asset references",
        parts: &[
            OwnedCapabilityPart {
                behavior: "asset GUIDs are parsed from meta files and indexed",
                location: "crates/unitas-asset/src/meta.rs",
                evidence: &[
                    "crates/unitas-asset/tests/meta_tests.rs::meta_guid_is_parsed_from_file",
                    "crates/unitas-asset/tests/meta_tests.rs::meta_index_maps_guid_to_asset_path",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "local file identifiers are preserved on load",
                location: "crates/unitas-asset/src/scene.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/project_tests.rs::prj003_ser004_missing_asset_and_sub_asset_keep_original_identity_with_similarly_named_siblings_and_duplicate_local_file_ids_are_rejected",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a sub-asset keeps its identity apart from its main asset",
                location: "crates/unitas-asset/src/database.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::search_combines_filters_and_distinguishes_sub_assets",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-004",
        owning_spec: "project-and-assets.md",
        capability: "Import textures, models, animation, and audio",
        parts: &[
            OwnedCapabilityPart {
                behavior: "textures are decoded and imported, including the PSD and Radiance HDR decoders",
                location: "crates/unitas-asset/src/texture/import.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/texture_tests.rs::dispatches_decoding_by_file_name",
                    "crates/unitas-asset/tests/texture_decoders_tests.rs::u4_psd_and_hdr_dispatch_through_the_shared_texture_decoder",
                    "crates/unitas-asset/tests/texture_decoders_tests.rs::u4_psd_and_hdr_are_recognized_as_texture_assets",
                    "crates/unitas-asset/tests/texture_decoders_tests.rs::u4_psd_rle_composite_decodes_to_the_known_pixels",
                    "crates/unitas-asset/tests/texture_decoders_tests.rs::u4_hdr_rle_scanlines_decode_to_the_known_float_pixels",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "models are imported",
                location: "crates/unitas-asset/src/model/mod.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/model_tests.rs::glb_import_reads_positions_and_indices",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "audio assets are imported",
                location: "crates/unitas-asset/src/audio_import.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/audio_import_tests.rs::decodes_aifc_with_none_compression",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an animation clip is decoded and imported",
                location: "crates/unitas-asset/src/anim_import.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/texture_tests.rs::imports_unity_yaml_anim_clip_curves_settings_and_events",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a platform texture format is encoded",
                location: "crates/unitas-asset/src/texture/import.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/texture_tests.rs::encodes_and_decodes_bcn_etc2_and_astc_compressed_texture_blocks_for_platform_targets",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-005",
        owning_spec: "project-and-assets.md",
        capability: "Inspect dependencies and reverse dependencies",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the dependency graph lists direct dependencies, transitive reverse dependencies, and cycles",
                location: "crates/unitas-asset/src/dependencies.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/dependencies_tests.rs::dependencies_direct_and_transitive_reverse",
                    "crates/unitas-asset/tests/asset_import/dependencies_tests.rs::prj005_outcome7_dependency_cycles_are_displayed_without_failure",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a dependency is classified by kind",
                location: "crates/unitas-asset/src/dependencies.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/dependencies_tests.rs::dependency_graph_classifies_direct_script_package_resources_bundle_and_editor_dependencies",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the editor ABI reports the dependencies, the reverse dependencies, and the cycles of an asset",
                location: "crates/unitas-editor-abi/src/asset_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::assets_dependencies_report_direct_and_reverse_dependencies",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-006",
        owning_spec: "project-and-assets.md",
        capability: "Resolve package manifests, registries, and project extensions",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the manifest and lock file are parsed and dependencies resolve with conflicts and unresolved causes named",
                location: "crates/unitas-asset/src/packages.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/packages_tests.rs::conflicting_transitive_requests_report_full_chains",
                    "crates/unitas-asset/tests/asset_import/packages_tests.rs::offline_open_reproduces_the_lock_and_names_missing_packages",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "package content is fetched from a registry, git host, or tarball and a dependency is added, removed, upgraded, or downgraded",
                location: "crates/unitas-asset/src/packages.rs",
                evidence: &[],
                missing: Some(
                    "no registry, git, or tarball client fetches package content and no dependency is added, removed, upgraded, or downgraded in manifest.json or packages-lock.json",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-007",
        owning_spec: "project-and-assets.md",
        capability: "Represent project settings and target-specific effects",
        parts: &[
            OwnedCapabilityPart {
                behavior: "time, dynamics, input, quality, player, and layer collision settings are parsed and honored, including per-target overrides",
                location: "crates/unitas-asset/src/project_settings.rs",
                evidence: &[
                    "crates/unitas-asset/tests/project_settings.rs::parses_time_manager_asset",
                    "crates/unitas-asset/tests/project_settings.rs::parses_dynamics_manager_asset",
                    "crates/unitas-asset/tests/project_settings.rs::parses_input_manager_asset_and_skips_joystick_axes",
                    "crates/unitas-asset/tests/project_settings.rs::parses_player_settings_from_prefab_fixture_and_inline_streams",
                    "crates/unitas-asset/tests/project_settings.rs::parses_quality_settings_asset_and_reports_per_target_effective_value",
                    "crates/unitas-asset/tests/project_settings.rs::prj007_outcome10_per_target_scripting_defines_collision_matrix_and_fixed_timestep_effective_target_resolution",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a changed setting is written back to its ProjectSettings file in Unity-readable form",
                location: "crates/unitas-asset/src/scene_save.rs",
                evidence: &[
                    "crates/unitas-asset/tests/project_settings.rs::project_settings_unchanged_save_byte_identity_and_edit_save_reload_round_trip",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "every setting family stored under ProjectSettings is represented",
                location: "crates/unitas-asset/src/project_settings.rs",
                evidence: &[
                    "crates/unitas-asset/tests/project_settings.rs::parses_all_project_settings_families_and_round_trips_edits",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the editor ABI inspects the setting families and writes one setting value back into its ProjectSettings file",
                location: "crates/unitas-editor-abi/src/asset_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::settings_inspect_reports_the_project_settings_families",
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::settings_update_writes_the_value_back_into_the_project_settings_file",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRJ-008",
        owning_spec: "project-and-assets.md",
        capability: "Project trust and safe opening of untrusted projects",
        parts: &[
            OwnedCapabilityPart {
                behavior: "per-user trust records decide whether project code may execute",
                location: "crates/unitas-asset/src/trust.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/trust_tests.rs::trust_record_is_written_outside_the_project",
                    "crates/unitas-asset/tests/asset_import/trust_tests.rs::untrusted_project_is_restricted",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the code vectors a project supplies are scanned and reported before anything executes",
                location: "crates/unitas-asset/src/trust.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/trust_tests.rs::untrusted_open_blocks_initialize_on_load_source_generator_and_git_fetch_while_assets_stay_inspectable",
                    "crates/unitas-asset/tests/asset_import/trust_tests.rs::trusted_open_with_fetch_consent_moves_vectors_to_permitted",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "SER-001",
        owning_spec: "serialization-and-prefabs.md",
        capability: "Read, preserve, edit, and save supported serialized values",
        parts: &[
            OwnedCapabilityPart {
                behavior: "text-serialized values are parsed into documents and preserved",
                location: "crates/unitas-asset/src/unity_yaml.rs",
                evidence: &[
                    "crates/unitas-asset/tests/unity_yaml_tests.rs::yaml_scene_fixture_is_parsed",
                    "crates/unitas-asset/tests/asset_import/scene_save_tests.rs::emit_stream_round_trips_the_parsed_stream",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an edit is written back and the asset is saved in Unity-readable form",
                location: "crates/unitas-asset/src/scene_save.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/scene_save_tests.rs::editing_one_field_changes_only_that_field",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "SER-002",
        owning_spec: "serialization-and-prefabs.md",
        capability: "Author and load scenes, including additive scene sets",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a scene is assembled into a scene graph with its components",
                location: "crates/unitas-asset/src/scene.rs",
                evidence: &[
                    "crates/unitas-asset/tests/scene_tests.rs::scene_game_objects_become_named_entities",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a scene set is saved",
                location: "crates/unitas-asset/src/scene_save.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/scene_save_tests.rs::the_scene_set_save_rejects_a_scene_outside_the_set",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a scene set is loaded additively",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_additive_load_keeps_both_scenes_alive",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "SER-003",
        owning_spec: "serialization-and-prefabs.md",
        capability: "Author prefab sources, instances, nesting, variants, and overrides",
        parts: &[
            OwnedCapabilityPart {
                behavior: "prefab sources, instances, and property modifications are read and edited",
                location: "crates/unitas-asset/src/prefab.rs",
                evidence: &[
                    "crates/unitas-asset/tests/prefab_load_overrides_tests.rs::prefab_instance_property_override_applies_at_load",
                    "crates/unitas-asset/tests/asset_import/prefab_variant_tests.rs::apply_override_targets_intermediate_variant_or_base_and_cleans_lower_redundant_overrides",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "variants, nested prefabs, and overrides are resolved",
                location: "crates/unitas-asset/src/prefab_variant.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/prefab_variant_tests.rs::multi_level_variant_inheritance_resolves_closer_level_precedence_and_provenance",
                    "crates/unitas-asset/tests/asset_import/prefab_variant_tests.rs::unchanged_prefab_save_byte_identity_and_nested_prefab_plus_broken_base_disk_round_trip",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "SER-004",
        owning_spec: "serialization-and-prefabs.md",
        capability: "Preserve missing, unknown, cyclic, and cross-scene references accurately",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a missing game object, parent, child, or prefab source is reported and is not replaced by similarly named content",
                location: "crates/unitas-asset/src/scene.rs",
                evidence: &[
                    "crates/unitas-asset/tests/scene_tests.rs::scene_missing_game_object_is_an_error",
                    "crates/unitas-asset/tests/asset_import/project_tests.rs::prj003_ser004_missing_asset_and_sub_asset_keep_original_identity_with_similarly_named_siblings_and_duplicate_local_file_ids_are_rejected",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a dependency cycle is detected and terminates safely",
                location: "crates/unitas-asset/src/dependencies.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/dependencies_tests.rs::prj005_outcome7_dependency_cycles_are_displayed_without_failure",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "SER-005",
        owning_spec: "serialization-and-prefabs.md",
        capability: "Text, binary, and mixed serialization forms and generated caches",
        parts: &[
            OwnedCapabilityPart {
                behavior: "text and binary serialized forms are read and their form is detected",
                location: "crates/unitas-asset/src/binary_ser.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/binary_ser_tests.rs::mixed_sniffing_reads_text_yaml_and_binary_v22",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the generated import cache is written and reused",
                location: "crates/unitas-asset/src/import_cache.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/import_cache_tests.rs::import_cache_persists_across_sessions_and_reimports_nothing_when_the_sources_are_unchanged",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PHY-001",
        owning_spec: "physics-navigation-terrain.md",
        capability: "3D bodies, colliders, joints, queries, and events",
        parts: &[
            OwnedCapabilityPart {
                behavior: "3D rigid bodies and colliders are simulated with forces and impulses",
                location: "crates/unitas-physics/src/world.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/force_mode_tests.rs::force_mode_force_applies_f_over_m_times_dt_during_step",
                    "crates/unitas-physics/tests/simulation/force_mode_tests.rs::force_mode_impulse_applies_f_over_m_at_call_time",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "all four of Unity's collision-detection modes are simulated, where only Discrete and Continuous existed before this phase",
                location: "crates/unitas-physics/src/body.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/ccd_mode_tests.rs::discrete_collision_detection_tunnels_through_a_thin_wall",
                    "crates/unitas-physics/tests/physics/ccd_mode_tests.rs::continuous_collision_detection_sweeps_static_geometry_only",
                    "crates/unitas-physics/tests/physics/ccd_mode_tests.rs::continuous_dynamic_collision_detection_sweeps_moving_geometry",
                    "crates/unitas-physics/tests/physics/ccd_mode_tests.rs::continuous_speculative_collision_detection_uses_speculative_contacts",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a physic material's friction and bounce combine modes are read from the imported asset",
                location: "crates/unitas-asset/src/scene.rs",
                evidence: &[
                    "crates/unitas-asset/tests/project_settings.rs::parses_the_sample_bumper_physic_material",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Unity's four friction combine rules and four bounce combine rules are simulated, where every pair was averaged before this phase",
                location: "crates/unitas-physics/src/world.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::average_friction_combine_averages_the_frictions",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::minimum_friction_combine_takes_the_lower_friction",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::multiply_friction_combine_multiplies_the_frictions",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::maximum_friction_combine_takes_the_higher_friction",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::average_bounce_combine_averages_the_restitutions",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::minimum_bounce_combine_takes_the_lower_restitution",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::multiply_bounce_combine_multiplies_the_restitutions",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::maximum_bounce_combine_takes_the_higher_restitution",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::max_bounce_combine_keeps_the_authored_restitution_above_one",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "spatial queries run and collision and trigger events are delivered",
                location: "crates/unitas-physics/src/world.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/query_tests.rs::ray_hits_a_sphere_on_its_surface",
                    "crates/unitas-physics/tests/physics/collision_tests.rs::trigger_and_non_trigger_pair_emits_trigger_enter_stay_exit_and_zero_collision_events",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a collision callback carries the contact points, the impulse, and the relative velocity",
                location: "crates/unitas-physics/src/world.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::the_collision_payload_reports_contacts_and_impulse",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::the_collision_payload_reports_a_resting_contact",
                    "crates/unitas-physics/tests/physics/unity_semantics_tests.rs::the_collision_payload_is_reachable_in_either_order",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a 3D rigid joint constrains two bodies",
                location: "crates/unitas-physics/src/joint.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/joint_3d_tests.rs::fixed_joint_3d_locks_relative_position_and_orientation",
                    "crates/unitas-physics/tests/simulation/joint_3d_tests.rs::hinge_joint_3d_enforces_axis_motor_spring_and_angle_limits",
                    "crates/unitas-physics/tests/simulation/joint_3d_tests.rs::spring_joint_3d_pulls_bodies_toward_distance_band_with_damping",
                    "crates/unitas-physics/tests/simulation/joint_3d_tests.rs::character_joint_3d_anchors_bodies_and_clamps_twist_and_swing_cone",
                    "crates/unitas-physics/tests/simulation/joint_3d_tests.rs::configurable_joint_3d_enforces_locked_limited_free_axes_and_position_drives",
                    "crates/unitas-physics/tests/simulation/joint_3d_tests.rs::joint_3d_breaks_when_force_or_torque_exceeds_threshold_and_emits_event",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a mesh collider is simulated as the mesh it names",
                location: "crates/unitas-physics/src/world.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import_tests.rs::mesh_collider_resolves_its_named_mesh_and_cooks_collision_geometry",
                    "crates/unitas-physics/tests/simulation_tests.rs::a_mesh_collider_collides_with_the_mesh_and_not_with_its_bounding_box",
                    "crates/unitas-physics/tests/simulation_tests.rs::a_non_convex_mesh_collider_on_a_dynamic_body_is_refused",
                    "crates/unitas-runtime/tests/sample_project.rs::the_sample_mesh_collider_produces_a_mesh_shaped_collider",
                    "bridge/Unitas.Bridge.Tests/CollisionCounterpartTests.cs::MeshColliderIsMaterializedFromTheScene",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a GameObject carries more than one collider",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import_tests.rs::a_game_object_keeps_every_one_of_its_colliders",
                    "crates/unitas-physics/tests/simulation_tests.rs::a_body_carries_every_collider_of_its_game_object",
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::a_game_object_with_two_colliders_contributes_two_shapes",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PHY-002",
        owning_spec: "physics-navigation-terrain.md",
        capability: "2D bodies, colliders, effectors, joints, queries, and events",
        parts: &[
            OwnedCapabilityPart {
                behavior: "2D bodies, colliders, and effectors are simulated",
                location: "crates/unitas-physics/src/two_d/world.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/two_d_tests.rs::fixed_timestep_accumulator_sleeping_and_simulated_flag",
                    "crates/unitas-physics/tests/simulation/two_d_effector_tests.rs::platform_effector_respects_collider_mask_and_surface_arc",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a 2D joint constrains two bodies and can break",
                location: "crates/unitas-physics/src/two_d/body.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/two_d_tests.rs::phy_002_story_5_constant_force_2d_and_2d_joints_constrain_and_break",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "2D queries run and collision and trigger messages are delivered",
                location: "crates/unitas-physics/src/two_d/world.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/two_d_tests.rs::trigger_events_fire_enter_stay_exit_only_when_rigidbody_present",
                    "crates/unitas-physics/tests/simulation/two_d_tests.rs::shapes_box_circle_edge_polygon_and_spatial_queries_respect_contact_filter",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "2D physics is reachable from a Unity scene and from the C# surface",
                location: "crates/unitas-runtime/src/physics.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/physics2d_collider_surface_tests.rs::physics2d_composite_outline_merges_two_abutting_box_colliders_and_slides_smoothly_across_seam",
                    "bridge/Unitas.Bridge.Tests/Physics2DTests.cs::TestSceneParsesExtendedCollidersAndMaterials",
                    "bridge/Unitas.Bridge.Tests/Physics2DMessageTests.cs::TestWicksLanternEndToEndPlatformerScenario",
                ],
                missing: Some(
                    "the 2D solver is a hand-written impulse solver rather than a maintained engine, and 2D joints have no C# entry point",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PHY-003",
        owning_spec: "physics-navigation-terrain.md",
        capability: "Character movement and grounding",
        parts: &[OwnedCapabilityPart {
            behavior: "the character controller honors slope limit, step offset, skin width, and grounding",
            location: "crates/unitas-physics/src/character.rs",
            evidence: &[
                "crates/unitas-physics/tests/physics/character_tests.rs::character_lands_on_ground_and_reports_below_and_grounded",
                "crates/unitas-physics/tests/physics/character_tests.rs::excessive_slope_steeper_than_slope_limit_acts_as_wall",
                "crates/unitas-physics/tests/physics/character_tests.rs::step_taller_than_step_offset_blocks_forward_motion_and_sets_sides",
                "crates/unitas-physics/tests/physics/character_tests.rs::narrow_step_too_thin_to_support_tread_is_rejected",
            ],
            missing: None,
        }],
    },
    OwnedFeatureCapability {
        feature_id: "PHY-004",
        owning_spec: "physics-navigation-terrain.md",
        capability: "Wheels, articulations, cloth, and simulation control",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a wheel collider solves suspension and slip",
                location: "crates/unitas-physics/src/wheel.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/wheel_tests.rs::wheel_reports_ground_hit_with_normal_point_and_compression",
                    "crates/unitas-physics/tests/physics/wheel_tests.rs::suspension_compression_stays_within_the_configured_travel",
                    "crates/unitas-physics/tests/physics/wheel_tests.rs::steer_angle_rotates_the_wheel_forward_direction",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an articulation body is simulated",
                location: "crates/unitas-physics/src/articulation.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/articulation_tests.rs::joint_drive_never_leaves_the_authored_limits",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "cloth is simulated",
                location: "crates/unitas-physics/src/cloth.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/cloth_tests.rs::cloth_grid_keeps_its_spacing_while_falling",
                    "crates/unitas-physics/tests/simulation/cloth_tests.rs::cloth_collides_with_assigned_capsules_only",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the simulation advances under the script's control and reports the contacts of a step",
                location: "crates/unitas-physics/src/simulate.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/sim_control_tests.rs::a_script_driven_scene_moves_only_when_simulate_is_called",
                    "crates/unitas-physics/tests/simulation/sim_control_tests.rs::auto_simulation_selects_the_fixed_update_and_script_modes",
                    "crates/unitas-physics/tests/simulation/sim_control_tests.rs::a_simulate_step_reports_the_contacts_of_that_step",
                    "crates/unitas-physics/tests/simulation/sim_control_tests.rs::phy_004_story_8_script_simulation_mode_advances_only_via_simulate_and_local_physics_mode_scene_simulates_independently",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a wheel collider is reachable from a Unity scene and from the C# surface",
                location: "crates/unitas-runtime/src/physics.rs",
                evidence: &[],
                missing: Some(
                    "no runtime or C# entry point exists for a wheel collider, so a wheel can only be driven from Rust",
                ),
            },
            OwnedCapabilityPart {
                behavior: "an articulation body is reachable from a Unity scene and from the C# surface",
                location: "crates/unitas-runtime/src/physics.rs",
                evidence: &[],
                missing: Some(
                    "no runtime or C# entry point exists for an articulation body, so an articulation can only be driven from Rust",
                ),
            },
            OwnedCapabilityPart {
                behavior: "cloth is reachable from a Unity scene and from the C# surface",
                location: "crates/unitas-runtime/src/physics.rs",
                evidence: &[],
                missing: Some(
                    "no runtime or C# entry point exists for cloth, so cloth can only be driven from Rust",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "NAV-001",
        owning_spec: "physics-navigation-terrain.md",
        capability: "Walkable worlds, navigation data building, paths, agents, obstacles, and links",
        parts: &[
            OwnedCapabilityPart {
                behavior: "navigation data is built and is marked stale when it no longer matches its source",
                location: "crates/unitas-physics/src/navigation/mesh.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/navigation_tests.rs::navigation_navmesh_builds_one_polygon_per_walkable_triangle",
                    "crates/unitas-physics/tests/physics/navigation_tests.rs::nav_001_story_10_surface_edit_time_build_and_runtime_rebuild_after_geometry_change",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "agents, obstacles, and path queries resolve a path and name an unreachable reason",
                location: "crates/unitas-physics/src/navigation/agent.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/navigation_tests.rs::navigation_agent_walks_towards_the_destination_at_its_speed",
                    "crates/unitas-physics/tests/physics/navigation_tests.rs::navigation_path_query_reports_partial_when_the_goal_cannot_be_reached",
                    "crates/unitas-physics/tests/physics/navigation_tests.rs::nav_001_story_9_carving_obstacle_detours_agent_and_reports_distinct_path_statuses",
                    "crates/unitas-physics/tests/physics/navigation_tests.rs::navigation_off_mesh_link_manual_and_auto_traversal",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "ENV-001",
        owning_spec: "physics-navigation-terrain.md",
        capability: "Terrain, vegetation, large worlds, and environment behavior",
        parts: &[
            OwnedCapabilityPart {
                behavior: "terrain data, vegetation prototypes, and grass wind are authored and evaluated",
                location: "crates/unitas-physics/src/terrain/mod.rs",
                evidence: &[
                    "crates/unitas-physics/tests/physics/terrain_tests.rs::terrain_data_reports_resolution_size_and_per_position_height",
                    "crates/unitas-physics/tests/physics/terrain_tests.rs::vegetation_follows_the_terrain_height",
                    "crates/unitas-physics/tests/physics/terrain_tests.rs::env_001_story_12_wind_zone_moves_tree_creator_trees_and_grass_while_excluding_speedtree",
                    "crates/unitas-physics/tests/physics/terrain_tests.rs::large_world_terrain_is_accepted_and_reports_affected_systems",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "SpeedTree wind animation moves trunks, branches, and leaves",
                location: "crates/unitas-physics/src/terrain/mod.rs",
                evidence: &[],
                missing: Some(
                    "SpeedTree wind animation is excluded, so a SpeedTree prototype keeps its geometry, materials, and LODs but reports wind_excluded",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "ANI-001",
        owning_spec: "rendering-and-animation.md",
        capability: "Clips, curves, events, controllers, states, and blend trees",
        parts: &[
            OwnedCapabilityPart {
                behavior: "clips, curves, and animation events are sampled",
                location: "crates/unitas-animation/src/clip.rs",
                evidence: &[
                    "crates/unitas-animation/tests/animation/clip_tests.rs::zero_duration_clip_evaluates_at_zero_and_fires_event_once_on_entry",
                    "crates/unitas-animation/tests/animation/clip_tests.rs::same_frame_events_preserve_declared_order_even_at_identical_timestamps",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "state machines and blend trees drive parameters",
                location: "crates/unitas-animation/src/controller.rs",
                evidence: &[
                    "crates/unitas-animation/tests/animation/controller_tests.rs::float_parameter_greater_and_less_conditions_drive_transition",
                    "crates/unitas-animation/tests/animation/controller_tests.rs::ani_001_outcome_28_2d_and_direct_blend_trees_interruption_sources_and_callback_order",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "ANI-002",
        owning_spec: "rendering-and-animation.md",
        capability: "Avatars, humanoid retargeting, inverse kinematics, and playable graphs",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a humanoid avatar is retargeted",
                location: "crates/unitas-animation/src/humanoid.rs",
                evidence: &[
                    "crates/unitas-animation/tests/animation/humanoid_tests.rs::pose_to_muscles_retarget_and_muscles_to_pose_roundtrip_across_different_avatars",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "inverse kinematics is solved",
                location: "crates/unitas-runtime/src/animation.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/animation_ik_tests.rs::two_bone_ik_reaches_reachable_target_within_tolerance_and_preserves_bone_lengths",
                    "crates/unitas-runtime/tests/animation_ik_tests.rs::look_at_ik_rotates_forward_vector_toward_target_according_to_weights_and_clamp",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a playable graph evaluates its nodes and its script playables",
                location: "crates/unitas-animation/src/playable.rs",
                evidence: &[
                    "crates/unitas-animation/tests/animation/playable_tests.rs::clip_playable_reports_time_speed_duration_and_done_state",
                    "crates/unitas-animation/tests/animation/playable_tests.rs::ani_002_outcome_32_script_playable_callbacks_fire_in_the_order_the_specification_states",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "CIN-001",
        owning_spec: "rendering-and-animation.md",
        capability: "Camera behavior and time-based cinematic sequencing",
        parts: &[
            OwnedCapabilityPart {
                behavior: "virtual cameras blend and the highest priority active camera is live",
                location: "crates/unitas-runtime/src/cinemachine.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/cinemachine_tests.rs::cinemachine_custom_blend_is_used_for_a_matching_camera_pair",
                    "crates/unitas-runtime/tests/runtime/cinemachine_tests.rs::cinemachine_highest_priority_active_camera_is_live",
                    "crates/unitas-runtime/tests/runtime/cinemachine_tests.rs::cinemachine_presented_pose_blends_between_following_cameras",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a virtual camera follows a target and aims at a look-at target",
                location: "crates/unitas-runtime/src/cinemachine.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/cinemachine_tests.rs::cinemachine_virtual_camera_follows_target_and_aims_at_look_at_target",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a timeline drives bound animation and cameras",
                location: "crates/unitas-runtime/src/timeline.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/timeline_tests.rs::timeline_animation_track_gates_the_bound_animator",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-003",
        owning_spec: "rendering-and-animation.md",
        capability: "Lighting, shadows, sky, reflection and light probes, post-processing, and quality behavior",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a real-time directional shadow map is rendered from the project's quality level (enablement and hard or 3x3 PCF soft filtering from shadows, map size from Unity 2022.3's Built-in-RP shadowResolution multiplier rule NextPowerOfTwo(pixel_size * multiplier * 3.8), and camera-centered orthographic extent from shadowDistance, with per-platform default quality selection from m_PerPlatformDefaultQuality falling back to m_CurrentQuality)",
                location: "crates/unitas-render/src/gpu.rs",
                evidence: &[
                    "crates/unitas-render/src/gpu.rs::directional_shadow_darkens_occluded_surface_and_tracks_map_size",
                    "crates/unitas-render/src/shadow.rs::shadow_map_size_matches_unity_multiplier_table",
                    "crates/unitas-runtime/src/presenter.rs::quality_shadow_settings_mapping_covers_shadows_resolutions_and_distances",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a post chain of a bright pass, a separable blur, and an ACES-fit tone map runs as render-graph passes on the offscreen, capture, and surface paths, and is enabled per quality level",
                location: "crates/unitas-render/src/post.rs",
                evidence: &[
                    "crates/unitas-render/src/post.rs::bright_pass_keeps_pixels_above_the_threshold_and_drops_the_rest",
                    "crates/unitas-render/src/post.rs::aces_fit_maps_zero_to_zero_and_one_to_one",
                    "crates/unitas-render/src/post.rs::aces_fit_stays_monotonic",
                    "crates/unitas-render/src/post.rs::the_blur_weights_sum_to_one",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "cascade-split directional shadows, per-renderer ShadowCastingMode, URP 14 shadow-atlas resolution source and tent filter ladder, and skybox, ambient, and light probes are rendered",
                location: "crates/unitas-render/src/gpu.rs",
                evidence: &[],
                missing: Some(
                    "cascade splitting for shadowCascades 2/4, per-renderer ShadowCastingMode, URP 14's UniversalRenderPipelineAsset shadow-atlas resolution source and cascade model, URP's 5x5 and 7x7 tent filter ladder, and skybox, ambient, and light probes are not implemented",
                ),
            },
            OwnedCapabilityPart {
                behavior: "a staged or packaged unitas player carries the project's ProjectSettings directory next to the staged Assets directory, so the runtime resolves the Standalone quality level and renders the directional shadow; a project with no quality settings keeps the documented shadows-off fallback, only the Standalone target group is wired end to end today, and the WebGL group is unreachable because no web player crate exists",
                location: "crates/unitas-build/src/bundle.rs",
                evidence: &[
                    "crates/unitas-build/tests/build/project_settings_bundle.rs::bundled_player_carries_project_quality_settings",
                    "crates/unitas-build/tests/build/project_settings_bundle.rs::packaged_player_carries_project_quality_settings",
                    "crates/unitas-build/tests/build/project_settings_bundle.rs::bundle_without_project_settings_keeps_the_shadow_fallback",
                ],
                missing: Some(
                    "the WebGL target group stays unreachable until a web player crate exists",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the shadow map's texel count follows a quality level's max_texels cap down to a 64-texel floor, and the orthographic light volume is fitted as a cube of half-extent shadowDistance around the camera position",
                location: "crates/unitas-render/src/shadow.rs",
                evidence: &[],
                missing: Some(
                    "the shadow map never falls below 64 texels even when the quality level's max_texels cap is lower, and the orthographic light volume keeps an aspect of 1.0, so a scene wider than the camera aspect can clip sideways out of the shadow volume",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-004",
        owning_spec: "rendering-and-animation.md",
        capability: "Particles, trails, and visual effects",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a particle system simulates emission, lifetime, and over-lifetime curves",
                location: "crates/unitas-physics/src/particles.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/particles_tests.rs::rate_emission_produces_the_expected_particle_count",
                    "crates/unitas-physics/tests/simulation/particles_tests.rs::the_property_curves_interpolate_at_the_midpoint_of_the_lifetime",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a trail keeps a point history and fades",
                location: "crates/unitas-physics/src/trails.rs",
                evidence: &[
                    "crates/unitas-physics/tests/simulation/trails_tests.rs::a_trail_fades_width_and_color_from_the_head_to_the_tail",
                    "crates/unitas-physics/tests/simulation/trails_tests.rs::a_point_older_than_the_trail_time_expires",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a visual effect graph is simulated and rendered",
                location: "crates/unitas-physics/src/particles.rs",
                evidence: &[],
                missing: Some(
                    "Visual Effect Graph content is not simulated or rendered, so only ParticleSystem and TrailRenderer behavior is implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "EDT-007",
        owning_spec: "editor-and-authoring.md",
        capability: "External-change, source-control, and semantic merge workflows",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a semantic merge reports conflicting properties, overrides, and unresolved references",
                location: "crates/unitas-asset/src/merge.rs",
                evidence: &[
                    "crates/unitas-asset/tests/merge_tests.rs::merge_reports_property_conflict_with_base_and_both_values",
                    "crates/unitas-asset/tests/merge_tests.rs::semantic_diff_reports_prefab_override_changes",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an external change is recorded and a conflict with unsaved work is named",
                location: "crates/unitas-asset/src/merge.rs",
                evidence: &[
                    "crates/unitas-asset/tests/merge_tests.rs::edt_007_outcome_7_workspace_guard_accounts_for_unsaved_human_agent_and_conflicting_files_before_close_or_reload",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a source-control client is provided",
                location: "crates/unitas-asset/src/merge.rs",
                evidence: &[],
                missing: Some("no version-control client is provided"),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "CMP-002",
        owning_spec: "compatibility-contract.md",
        capability: "Supported package and version matrix",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the supported package matrix records each package's version line, tier, state, and owning feature",
                location: "crates/unitas-core/src/compat.rs",
                evidence: &[
                    "crates/unitas-core/tests/core/compat_tests.rs::cmp_002_story_6_supported_package_matrix_reports_urp_and_input_system_supported_and_unlisted_or_preview_unknown",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the state of an owned feature is derived from its demonstrated and missing capabilities",
                location: "crates/unitas-core/src/compat.rs",
                evidence: &[
                    "crates/unitas-core/tests/core/compat_tests.rs::cmp_002_every_owned_feature_reports_a_real_state_from_assess_project",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the editor ABI reports the package matrix, the project findings, and the state of every owned feature",
                location: "crates/unitas-editor-abi/src/compat_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::compat_package_matrix_reports_the_supported_packages",
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::compat_report_lists_findings_and_owned_feature_states",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "CMP-004",
        owning_spec: "compatibility-contract.md",
        capability: "Product versioning, release readiness, and interface stability",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the editor ABI reports the declared Unity version, the effective behavior version, and the migration state of a project",
                location: "crates/unitas-editor-abi/src/compat_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::compat_version_info_reports_the_declared_and_effective_versions",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the release readiness and the interface stability of a product are checked",
                location: "crates/unitas-editor-abi/src/compat_tools.rs",
                evidence: &[],
                missing: Some(
                    "the editor ABI reports the version information of a project and checks no release readiness and no interface stability",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "DIA-001",
        owning_spec: "diagnostics-profiling-playtesting.md",
        capability: "Logs, exceptions, diagnostics, and source navigation",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the editor ABI lists the trust, package, and import diagnostics of a project with severity, category, identifier, and message",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::diagnostics_list_reports_the_trust_blocked_script",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a finding navigates to the responsible content and opens its script location in the external editor",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[],
                missing: Some(
                    "the editor ABI reports diagnostics but navigates to no content and opens no script location",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRF-001",
        owning_spec: "diagnostics-profiling-playtesting.md",
        capability: "CPU, frame, rendering, memory, physics, audio, and target profiling",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the editor ABI captures the timing markers and the counters of a project inspection pass",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::profiler_capture_reports_timing_markers_and_counters",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a running player reports its frame, rendering, memory, physics, and audio profiling",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[],
                missing: Some(
                    "the editor ABI captures a project inspection pass and profiles no running player frame",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "ACC-001",
        owning_spec: "accessibility-localization-persistence.md",
        capability: "Accessible editor and game experiences",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the editor ABI reports the fonts, text assets, sorting layers, and input axes of a project",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[
                    "crates/unitas-editor-abi/tests/editor_tools_tests.rs::accessibility_inspect_reports_fonts_text_assets_and_input_axes",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the accessibility roles, labels, focus order, and contrast of the scene content are inspected",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[],
                missing: Some(
                    "the editor ABI reports the accessibility assets of a project and inspects no scene role, label, focus order, or contrast",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "SAV-001",
        owning_spec: "accessibility-localization-persistence.md",
        capability: "Player settings, save data, corruption, and migration",
        parts: &[
            OwnedCapabilityPart {
                behavior: "PlayerPrefs resolves to Unity's per-target preferences location, lands in a directory disjoint from save data, and stays inside the armed data root when persistence is armed",
                location: "bridge/UnityEngine/PlayerPrefs.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/PlayerPrefsPersistenceTests.cs::TestPlayerPrefsResolvesPerTargetPreferencesLocation",
                    "bridge/Unitas.Bridge.Tests/PlayerPrefsPersistenceTests.cs::TestPreferencesLandUnderArmedOverrideAndStayDisjointFromSaveData",
                    "bridge/Unitas.Bridge.Tests/PlayerPrefsPersistenceTests.cs::TestDeleteAllAndPersistentDataPathSaveFilesRemainMutuallyIsolated",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a save write is write-then-replace: an interrupted write and a quota refusal each leave the prior valid save byte-identical and raise distinct exceptions to the caller",
                location: "bridge/UnityEngine/SaveMigration.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory7InterruptedSaveFileWriteKeepsThePriorValidSaveByteIdentical",
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory7QuotaExhaustedSaveFileWriteKeepsThePriorValidSaveAndLeavesNoStagingFile",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "loading reports distinct outcomes (Absent, Corrupt, IncompatibleNewerVersion, MigrationFailed) and a migration reports its input version, resulting version, and applied steps",
                location: "bridge/UnityEngine/SaveMigration.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory8SaveMigrationReportsInputVersionResultingVersionAndDistinctOutcomes",
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory8AbsentSaveFileReportsTheAbsentOutcome",
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory8MalformedSaveFileReportsTheCorruptOutcome",
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory8MigratedSaveFileReportsInputVersionResultingVersionAndAppliedSteps",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a binary payload round-trips through the same write-then-replace save path",
                location: "bridge/UnityEngine/SaveMigration.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory7BinarySaveFilePayloadRoundTripsByteIdentical",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the roll-a-ball sample restores its records, medals, and unlocked-arena set from PlayerPrefs on the next launch and writes and restores a ghost file under Application.persistentDataPath",
                location: "samples/roll-a-ball/Assets/Scripts/SavedState.cs",
                evidence: &[
                    "samples/roll-a-ball/Assets/Scripts/SavedState.cs::RestoreAndSave",
                    "samples/roll-a-ball/Scenarios/persist_state_read.json::persist_state_read",
                    "samples/roll-a-ball/Scenarios/persist_state_write.json::persist_state_write",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the sample's ghost file records the ball's real per-fixed-step run and the restored route matches that recorded run within 0.01 m",
                location: "samples/roll-a-ball/Assets/Scripts/SavedState.cs",
                evidence: &[
                    "samples/roll-a-ball/Assets/Scripts/ScoreManager.cs::FixedUpdate",
                    "samples/roll-a-ball/Assets/Scripts/SavedState.cs::FinishRun",
                    "samples/roll-a-ball/Scenarios/persist_state_write.json::persist_state_write",
                    "samples/roll-a-ball/Scenarios/persist_state_read.json::persist_state_read",
                    "samples/roll-a-ball/Scenarios/ghost_replay.scenario.json::ghost_replay",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the engine states each target's PlayerPrefs durability point, persistent-data write durability point, crash loss window, and write-then-replace guarantees",
                location: "bridge/UnityEngine/SaveMigration.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory11ProductKnowledgeStatesPerTargetDurabilityAndCrashLossWindow",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a playtest scenario that terminates the player during a save file write under Application.persistentDataPath, and one that exhausts storage quota, finds the prior valid save intact on every target including the browser, and the failed write is reported to the calling code",
                location: "bridge/UnityEngine/SaveMigration.cs",
                evidence: &[],
                missing: Some(
                    "the interrupted write and the quota refusal are simulated in-process through SaveMigration.SimulatedInterruptionAfterBytes and SaveMigration.SimulatedQuotaBytes instead of by a playtest scenario that terminates the player, and the write-then-replace guarantee is not proven on the browser target",
                ),
            },
            OwnedCapabilityPart {
                behavior: "save data round-trips through JsonUtility, Newtonsoft Json, XmlSerializer, and BinaryFormatter on each scripting backend and every target including the browser, and a type removed by managed code stripping produces a diagnostic naming the type",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[],
                missing: Some(
                    "the native scripting backend provides no JsonUtility, Newtonsoft Json, XmlSerializer, or BinaryFormatter, the C# bridge round-trips a save envelope through JsonUtility, Newtonsoft Json, and XmlSerializer on the desktop mono backend only, the bridge reports BinaryFormatter as partially supported and unavailable on the browser target but round-trips no payload through it, the browser target round-trip is not demonstrated, and the stripped-type diagnostic that names the type and member is produced by the bridge test harness and not by the shipped save path",
                ),
            },
            OwnedCapabilityPart {
                behavior: "product knowledge available through automation states the per-target durability guarantees and each target's storage quota",
                location: "crates/unitas-editor-abi/src/compat_tools.rs",
                evidence: &[],
                missing: Some(
                    "the per-target durability contract is not available through automation, no editor command exposes SaveMigration.GetTargetDurabilityContract, the contract states no storage quota for a target, the browser target's asynchronous flush is not proven, and crash survival is asserted as product knowledge and simulated in-process rather than demonstrated per target",
                ),
            },
            OwnedCapabilityPart {
                behavior: "on the browser target PlayerPrefs stores more than 1 MB up to the browser's quota, and a save file closed under Application.persistentDataPath is present after the page reloads",
                location: "bridge/UnityEngine/PlayerPrefs.cs",
                evidence: &[],
                missing: Some(
                    "the browser target has no IndexedDB-backed storage: the C# bridge's WebGL preferences fall back to the host's local application data folder and a closed persistent-data file is never copied into the browser's persistent storage, so neither the 1 MB limit nor survival across a page reload is demonstrated",
                ),
            },
            OwnedCapabilityPart {
                behavior: "PlayerPrefs on the Windows target is stored in the registry-backed per-user store Unity uses for that target, company name, and product name",
                location: "bridge/UnityEngine/Application.cs",
                evidence: &[],
                missing: Some(
                    "the Windows target's preferences are a file under %USERPROFILE%\\AppData\\LocalLow\\<company>\\<product>\\prefs instead of Unity's registry-backed per-user store",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the roll-a-ball sample's settings all persist through PlayerPrefs and take effect immediately",
                location: "samples/roll-a-ball/Assets/Scripts/SavedState.cs",
                evidence: &[],
                missing: Some(
                    "the sample persists its records, medals, unlocked-arena set and the quality setting through PlayerPrefs, but the stored quality value is only read back and logged and never applied to the render quality, and the sample defines a reduced-motion preference that the game honours (the speed trail stops while it is on) but exposes through no settings panel, and still defines no master, music, or effects volume, fullscreen mode, resolution, vertical sync, or camera shake setting to persist",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "BLD-005",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Content builds and post-release content updates",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a content bundle is staged and a content catalog is built",
                location: "crates/unitas-build/src/bundle.rs",
                evidence: &[
                    "crates/unitas-build/tests/build/content_catalog.rs::content_update_diffing_against_v1_catalog_emits_only_changed_bundles_and_updated_catalog",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a content update produces the changed bundles and a new catalog",
                location: "crates/unitas-build/src/bundle.rs",
                evidence: &[
                    "crates/unitas-build/tests/build/content_catalog.rs::bld_005_story_25_addressables_profile_groups_dependencies_and_shipped_player_content_update_without_rebuild",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a Unity-built AssetBundle or Addressables catalog is loaded in a unitas player",
                location: "crates/unitas-build/src/bundle.rs",
                evidence: &[],
                missing: Some(
                    "a Unity-built AssetBundle or Addressables catalog is not loaded, so a player loads only content unitas builds from project sources",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-001",
        owning_spec: "scripting-runtime.md",
        capability: "GameObject, Component, Transform, identity, and lifetime",
        parts: &[
            OwnedCapabilityPart {
                behavior: "native scene-graph entities keep distinct live identities, parent-child transform composition, and deferred frame-boundary destruction without reissuing destroyed ids",
                location: "crates/unitas-core/src/scene_graph.rs",
                evidence: &[
                    "crates/unitas-core/tests/core/scene_graph.rs::attach_and_detach_updates_the_hierarchy",
                    "crates/unitas-core/tests/core/scene_graph.rs::world_transform_composes_through_the_whole_chain",
                    "crates/unitas-core/tests/destruction_tests.rs::marking_destroyed_destroys_the_children_with_their_parent",
                    "crates/unitas-core/tests/destruction_tests.rs::a_destroyed_identity_is_never_reissued",
                    "crates/unitas-runtime/tests/destroy_lifecycle_tests.rs::a_marked_entity_leaves_the_scene_graph",
                    "crates/unitas-runtime/tests/runtime/runtime_object_tests.rs::runtime_object_creates_entity_in_active_scene_graph",
                    "crates/unitas-runtime/tests/runtime/runtime_object_tests.rs::runtime_object_destroy_removes_entity_from_graph_scripts_and_physics_world",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the managed Transform hierarchy mirror converges onto the native transform hierarchy and Object.Instantiate copies native child hierarchies and serialized field values into clones",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[],
                missing: Some(
                    "the managed Transform keeps a separate hierarchy mirror rather than converging wholesale onto the native transform hierarchy, and Object.Instantiate does not copy native child hierarchies and serialized field values into native clones",
                ),
            },
            OwnedCapabilityPart {
                behavior: "a destroyed managed object compares equal to null through the UnityEngine.Object operators and converts to false, while the managed reference and null-conditional access still observe the destroyed object and a live object compares non-null",
                location: "bridge/UnityEngine/Object.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/RuntimeObjectTests.cs::TestRuntimeObjectDestroyFakeNull",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a ScriptableObject created with CreateInstance is held by the native world under its own identity and lifetime, receives the Awake, OnEnable, and OnValidate callbacks on creation and the OnDisable and OnDestroy callbacks on destruction, and compares equal to null once destroyed",
                location: "bridge/UnityEngine/ScriptableObject.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/ScriptableObjectTests.cs::TestScriptableObjectCreateInstanceLifecycle",
                    "bridge/Unitas.Bridge.Tests/ScriptableObjectTests.cs::TestScriptableObjectDestroyFakeNullAndDoubleDestroy",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a ScriptableObject is stored in a project .asset file, is offered for creation by the editor's CreateAssetMenu attribute, is cloned by Object.Instantiate, and is loaded from a Resources folder or the Preloaded Assets player setting",
                location: "bridge/UnityEngine/ScriptableObject.cs",
                evidence: &[],
                missing: Some(
                    "a ScriptableObject is not serialized to a .asset file, carries no CreateAssetMenu attribute, is not cloned by Object.Instantiate, and is not loaded from a Resources folder or the Preloaded Assets player setting, so a ScriptableObject exists only as a runtime instance",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-002",
        owning_spec: "scripting-runtime.md",
        capability: "C# scripting profile and UnityEngine-compatible APIs",
        parts: &[
            OwnedCapabilityPart {
                behavior: "project scripts compile against the UnityEngine-compatible API assembly with the C# 9 language profile and the Unity 2022.3 symbols of the selected target",
                location: "bridge/Unitas.Scripting/ScriptCompilation.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory7TargetDefinesEditorAndPlayers",
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestCliCompileEntryPoint",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the generated glue dispatches Unity's messages to project scripts without reflection",
                location: "bridge/Unitas.Scripting/ScriptAnalyzerAndGlueGenerator.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestGeneratedGlueAndZeroReflectionMessageDispatch",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the Mathf functions the scripting profile exposes behave as Unity",
                location: "bridge/UnityEngine/Mathf.cs",
                evidence: &["bridge/Unitas.Bridge.Tests/CoreApiTests.cs::TestMathfSurface"],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the native interop members of the scripting profile are available on the targets that provide them and raise Unity's exception on the targets that do not",
                location: "bridge/UnityEngine/UnityEngine.csproj",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no AndroidJavaObject, AndroidJavaClass, AndroidJavaProxy, AndroidJNI, AndroidJNIHelper, or UnitySendMessage member, so a project that calls one does not compile and no build report lists the use as Platform-limited naming the calling member",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the API compatibility level the project's Player settings choose is applied to the compilation of each target",
                location: "bridge/Unitas.Scripting/ScriptCompilation.cs",
                evidence: &[],
                missing: Some(
                    "the compiler always references the host's netstandard.dll and System.* assemblies and nothing reads the API compatibility level from ProjectSettings, so a project that selects the .NET Framework level compiles against .NET Standard 2.1 instead",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the Visual Scripting package's script graphs and state graphs run in editor, headless, and player sessions",
                location: "bridge/UnityEngine/UnityEngine.csproj",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no Visual Scripting type, so no script machine, state machine, flow graph, or node library runs and the com.unity.visualscripting package stays unsupported",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-003",
        owning_spec: "scripting-runtime.md",
        capability: "MonoBehaviour lifecycle, messages, attributes, and activation behavior",
        parts: &[
            OwnedCapabilityPart {
                behavior: "MonoBehaviour scripts receive Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate, OnDisable, and OnDestroy callbacks and physics collision and trigger messages in frame order",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_lifecycle_order_over_two_ticks_and_shutdown",
                    "crates/unitas-runtime/tests/runtime/physics_tests.rs::runtime_dispatches_ball_crate_trigger_phases",
                    "crates/unitas-runtime/tests/physics_event_order_tests.rs::the_physics_message_order_is_the_same_at_every_frame_rate",
                    "crates/unitas-runtime/tests/runtime/roll_a_ball_tests.rs::roll_a_ball_scenario_collects_each_pickup_exactly_once_at_every_frame_rate",
                    "crates/unitas-runtime/tests/runtime/runtime_object_tests.rs::runtime_object_attaches_script_and_runs_lifecycle_in_order",
                    "crates/unitas-runtime/tests/runtime/runtime_object_tests.rs::runtime_object_creates_trigger_body_and_fires_trigger_event_with_new_entity_id",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Script Execution Order settings and [DefaultExecutionOrder], [RequireComponent], [DisallowMultipleComponent], and [ExecuteAlways] attributes are enforced by the native runtime",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[],
                missing: Some(
                    "the native runtime does not sort script dispatch by a ProjectSettings script execution order table or [DefaultExecutionOrder] and does not enforce [RequireComponent], [DisallowMultipleComponent], or [ExecuteAlways] edit-mode execution",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-004",
        owning_spec: "scripting-runtime.md",
        capability: "Coroutines, asynchronous operations, threading, and time",
        parts: &[
            OwnedCapabilityPart {
                behavior: "Time scale, fixed timestep accumulator, maximumDeltaTime catch-up clamping, and scaled versus unscaled frame advancement run deterministically",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_fixed_step_accumulator_accumulates_and_dispatches",
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_time_scale_halves_the_simulated_time_of_a_falling_body",
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_zero_time_scale_freezes_the_world_while_update_still_runs",
                    "crates/unitas-runtime/tests/frame_loop_tests.rs::frame_loop_fixed_step_catch_up_is_clamped_by_maximum_delta_time",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "The native runtime runs a Unity 2022.3 PlayerLoop phase tree, a custom PlayerLoopSystem registered at a named position runs every frame at that position, removing a built-in system removes its work, and the tree round-trips through the ABI; Time.captureFramerate advances game time by the inverse rate regardless of the wall clock",
                location: "crates/unitas-runtime/src/player_loop.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/player_loop_tests.rs::the_default_player_loop_lists_the_seven_unity_phases_in_order",
                    "crates/unitas-runtime/tests/runtime/player_loop_tests.rs::a_custom_system_runs_at_its_registered_position_every_frame",
                    "crates/unitas-runtime/tests/runtime/player_loop_tests.rs::removing_a_built_in_system_removes_its_work",
                    "crates/unitas-runtime/tests/runtime/player_loop_tests.rs::a_player_loop_tree_round_trips_through_the_abi",
                    "bridge/Unitas.Bridge.Tests/TimeDiagnosticsTests.cs::CaptureFramerateAdvancesGameTimeByInverseRateRegardlessOfWallClock",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "main-thread API guards and cross-origin isolated browser threading are enforced by the native runtime",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[],
                missing: Some(
                    "main-thread API thread guards and cross-origin isolated browser worker threading are not implemented in the native runtime",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-005",
        owning_spec: "scripting-runtime.md",
        capability: "Runtime scene management and persistent objects",
        parts: &[
            OwnedCapabilityPart {
                behavior: "Single and Additive runtime scene loading, active scene selection, scene unloading with script and body teardown, and DontDestroyOnLoad persistence across single loads are executed",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_single_load_replaces_the_loaded_scene_set",
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_additive_load_keeps_both_scenes_alive",
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_unload_scene_disables_then_destroys_its_scripts",
                    "crates/unitas-runtime/tests/runtime/runtime_tests.rs::runtime_dont_destroy_on_load_survives_a_single_load_replacement",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "LoadSceneAsync with allowSceneActivation holding at 0.9 progress, build-list scene index resolution, and MoveGameObjectToScene are provided",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[],
                missing: Some(
                    "asynchronous scene loading with allowSceneActivation holding at 0.9 progress, build scene list index resolution, and MoveGameObjectToScene are not implemented in the native runtime",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-006",
        owning_spec: "scripting-runtime.md",
        capability: "Resource loading, residency, release, and content catalogs",
        parts: &[
            OwnedCapabilityPart {
                behavior: "Resources.Load, Resources.LoadAll, and Resources.LoadAsync find assets in any Resources folder by their path without the extension, a missing path returns null, and Resources.UnloadAsset and Resources.UnloadUnusedAssets release them",
                location: "bridge/UnityEngine/Resources.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/ResourcesSceneTests.cs::TestResourcesLoadTextAssetPrefabInstantiateAndMissingPaths",
                    "bridge/Unitas.Bridge.Tests/ResourcesSceneTests.cs::TestResourcesUnloadUnusedAssetsAndUnloadAssetValidation",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the resident assets, their paths, and the references held on them are inspectable, and the references decide when an asset is released",
                location: "bridge/UnityEngine/Resources.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/ResourcesSceneTests.cs::TestResourcesUnloadUnusedAssetsReferenceTrackingAndResidencyVisibility",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the resident assets, their reference counts, and the origin of each load are visible in the editor and through automation",
                location: "crates/unitas-editor-abi/src/asset_tools.rs",
                evidence: &[],
                missing: Some(
                    "the residency counters live in the managed bridge and the editor ABI exposes no tool that reports them, so the residency of a loaded asset is not visible through automation",
                ),
            },
            OwnedCapabilityPart {
                behavior: "an AssetBundle loads from a file, a memory block, a stream, or a web request, exposes its assets and scenes, and unloads with Unity's two modes",
                location: "bridge/UnityEngine/UnityEngine.csproj",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no AssetBundle type, so no bundle is loaded at runtime, a bundle unitas built is never read by a player, and the dependency, duplicate-load, and Unload(false) rules of the specification are not implemented",
                ),
            },
            OwnedCapabilityPart {
                behavior: "Addressables content loads by address key, label, and AssetReference, and a handle carries its status, progress, result, exception, and reference count",
                location: "bridge/UnityEngine/UnityEngine.csproj",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no Addressables type, so no addressable catalog, key, label, handle, or reference count exists and content cannot be loaded by address",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-007",
        owning_spec: "scripting-runtime.md",
        capability: "Script compilation, assemblies, defines, and plugins",
        parts: &[
            OwnedCapabilityPart {
                behavior: "scripts outside an assembly definition compile into Unity's predefined assemblies with Unity's folder rules, and assembly definition and assembly reference files define named assemblies with their references, platform exclusion, define constraints, version defines, and the scripts they claim",
                location: "bridge/Unitas.Scripting/AssemblyDiscovery.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory6AssembliesAsmrefDefinesAndVersionDefines",
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory6ReferenceCycleDiagnostic",
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory6OverrideReferencesHonorsPrecompiledReferences",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the compilation defines the Unity 2022.3 symbols of the selected target, including the editor, platform, development, and scripting backend symbols, and a csc.rsp contributes its defines",
                location: "bridge/Unitas.Scripting/DefinesAndResponseParser.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory7TargetDefinesEditorAndPlayers",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a failed edit leaves the last valid assemblies in effect and only the affected assemblies and their dependents recompile, a plugin excluded for the running architecture fails its DllImport with a diagnostic naming the library, target, architecture, and calling member, and a player build refuses a runtime script that references UnityEditor unguarded",
                location: "bridge/Unitas.Scripting/ScriptCompilation.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory1And4FailedEditPreservesLastValidAndIncrementalRecompile",
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory10IncompatibleNativePluginDllImportDiagnostic",
                    "bridge/Unitas.Bridge.Tests/CompilationTests.cs::TestStory29PlayerBuildRejectsUnguardedUnityEditorReference",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Roslyn analyzers and source generators run against the assemblies Unity's analyzer scoping rules apply them to, and their diagnostics and generated types join the compilation",
                location: "bridge/Unitas.Scripting/ScriptCompilation.cs",
                evidence: &[],
                missing: Some(
                    "the compiler loads no analyzer and no source generator, so an analyzer's warning and a generator's added type never appear in the compilation report and a trusted project's generated code is never compiled",
                ),
            },
            OwnedCapabilityPart {
                behavior: "every compiler option a csc.rsp carries takes Unity's effect, and an option Unity 2022.3 rejects is reported naming the option and the file",
                location: "bridge/Unitas.Scripting/DefinesAndResponseParser.cs",
                evidence: &[],
                missing: Some(
                    "the response file parser honors defines, warning suppression, warnings as errors, the unmanaged-memory option, and additional references and reports every other option as an unsupported compiler option, so the remaining Unity 2022.3 response file options never take Unity's effect",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-008",
        owning_spec: "scripting-runtime.md",
        capability: "Serialized events, messages, and delayed invocation",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a UnityEvent and its generic arities keep persistent listeners with their target, method, argument mode, and call state, invoke the enabled persistent listeners in serialized order before the dynamic ones, and report a persistent listener whose target or method no longer exists",
                location: "bridge/UnityEngine/UnityEvent.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestUnityEventPersistentBeforeDynamicOrdering",
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestUnityEventGenericPersistentAndDynamicListeners",
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestUnityEventMissingOrRenamedPersistentMethodLogsWarningAndContinues",
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestUnityEventCallStateAndRemoveAllListenersPreservesPersistent",
                    "bridge/Unitas.Bridge.Tests/UnityEventArityTests.cs::TestFourArityUnityEventDynamicAndPersistentListeners",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "SendMessage, BroadcastMessage, and SendMessageUpwards reach the components Unity's rules include, and a message with no receiver logs Unity's error naming the method and the target",
                location: "bridge/UnityEngine/GameObject.Messaging.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestSendMessageAndBroadcastMessageAndSendMessageUpwards",
                    "bridge/Unitas.Bridge.Tests/UnityEventArityTests.cs::TestSendMessageRequireReceiverVsDontRequireReceiver",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Invoke and InvokeRepeating run in scaled time, CancelInvoke cancels the pending invocations, IsInvoking reports them, and a destroyed owner stops the invocation for good",
                location: "bridge/UnityEngine/MonoBehaviour.Invoke.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestInvokeAndInvokeRepeatingScaledTimeAndPause",
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestInvokeStopsPermanentlyWhenOwnerOrGameObjectDestroyed",
                    "bridge/Unitas.Bridge.Tests/EventsTests.cs::TestCancelInvokeAndIsInvokingQueries",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "disabling the behaviour or deactivating its object cancels the pending invocations as Unity does",
                location: "bridge/UnityEngine/InvokeScheduler.cs",
                evidence: &[],
                missing: Some(
                    "the invoke scheduler cancels a pending invocation only when its owner is destroyed and never reads Behaviour.enabled or GameObject.activeInHierarchy, so a disabled behaviour and a deactivated object keep firing",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-009",
        owning_spec: "scripting-runtime.md",
        capability: "Web requests, file locations, JSON utilities, and randomness",
        parts: &[
            OwnedCapabilityPart {
                behavior: "project companyName, productName, and version are parsed from ProjectSettings and exposed through the runtime ABI for target file-location resolution",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/abi_scene_tests.rs::abi_project_identity_externs_write_loaded_identity_into_path_buffer",
                    "crates/unitas-runtime/tests/runtime/abi_scene_tests.rs::abi_project_identity_externs_return_negative_when_settings_missing_or_handle_invalid",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Application.persistentDataPath resolves per target to Unity's location for the project's company and product names",
                location: "bridge/UnityEngine/Application.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/SaveMigrationTests.cs::TestStory26PersistentDataPathResolvesPerTargetToUnityCompanyProductLocation",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "UnityWebRequest HTTP transport, CertificateHandler validation, and target-specific socket and browser ClientWebSocket diagnostics are provided by the runtime",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[],
                missing: Some(
                    "UnityWebRequest network transport, CertificateHandler validation, and target-specific raw socket or browser ClientWebSocket runtime diagnostics are not implemented",
                ),
            },
            OwnedCapabilityPart {
                behavior: "in editor and headless sessions the file locations resolve to the locations Unity uses on that host, so data the project wrote while running under Unity is found",
                location: "bridge/UnityEngine/Application.cs",
                evidence: &[],
                missing: Some(
                    "the editor targets are shown to resolve to the same persistent-data location as the player targets, but the headless session's file locations are not shown to match the locations Unity uses on that host, and data written while running under Unity is not demonstrated to be found",
                ),
            },
            OwnedCapabilityPart {
                behavior: "a file location that is not writable or not directly accessible on a target is reported with the target and the path",
                location: "bridge/UnityEngine/Application.cs",
                evidence: &[],
                missing: Some(
                    "a file location that is not writable or not directly accessible on a target is not reported with the target and the path",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-010",
        owning_spec: "scripting-runtime.md",
        capability: "Editor-time scripts and content hooks",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a method marked [MenuItem] is discovered with its validation function, priority, and shortcut, appears in the editor command palette and as an automation tool, and is disabled with its reason when the validation function returns false",
                location: "bridge/UnityEngine/UnityEditor/MenuItem.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/EditorMenuTests.cs::TestStory27MenuItemAttributePropertiesAndShortcutParsing",
                    "bridge/Unitas.Bridge.Tests/EditorMenuTests.cs::TestStory27CommandPaletteAndAutomationToolDiscoveryAndValidation",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a modal dialog raised by an editor script never blocks, returns the answer supplied for it or the default button, and is recorded as a structured event",
                location: "bridge/UnityEngine/UnityEditor/EditorUtility.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/EditorMenuTests.cs::TestStory44DisplayDialogNonBlockingDefaultAndOverrideInAgentSession",
                    "bridge/Unitas.Bridge.Tests/EditorMenuTests.cs::TestStory44DisplayDialogComplexFromMenuCommandInAgentSession",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an editor method a project assembly contributes is discovered from the compiled assembly and registered as a tool with the execute-project-code scope",
                location: "editor/Unitas.Editor/Registry/ExecuteMethodTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/ExecuteMethodToolsTests.cs::DiscoversEditorCallableMethodsAndEmitsGlueRegistration",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the content hooks run in a trusted project: AssetPostprocessor and ScriptedImporter during import, OnValidate and Reset after an edit, InitializeOnLoad after each domain reload, and the EditorApplication callbacks",
                location: "editor/Unitas.Editor/Registry/ExecuteMethodTools.cs",
                evidence: &[],
                missing: Some(
                    "the editor invokes a project editor method only on demand through editor.execute_method and defines no AssetPostprocessor, ScriptedImporter, OnValidate, Reset, InitializeOnLoad, or EditorApplication callback, so no content hook runs during import or after a domain reload",
                ),
            },
            OwnedCapabilityPart {
                behavior: "a method marked [ContextMenu] or [ContextMenuItem] appears as a named command beside the menu commands",
                location: "bridge/UnityEngine/UnityEditor/MenuItem.cs",
                evidence: &[],
                missing: Some(
                    "the editor reads only the MenuItem attribute, so a context menu method and a context menu item never appear in the command palette or as an automation tool",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the public UnityEditor API of Unity 2022.3 LTS is provided with Unity's meaning, including AssetDatabase, SerializedObject, EditorPrefs, Selection, PrefabUtility, CompilationPipeline, EditorSceneManager, and the drawing and gizmo APIs",
                location: "bridge/UnityEngine/UnityEditor/EditorUtility.cs",
                evidence: &[],
                missing: Some(
                    "the UnityEditor assembly of the bridge defines only EditorUtility and MenuItem, so no AssetDatabase, SerializedObject, EditorPrefs, SessionState, EditorApplication, Selection, EditorGUIUtility, DragAndDrop, GenericMenu, TypeCache, EditorSceneManager, PrefabUtility, CompilationPipeline, custom editor drawing, or BuildPipeline member exists and an editor script that calls one does not compile",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "RUN-011",
        owning_spec: "scripting-runtime.md",
        capability: "Jobs, Burst-compiled code, and native collections",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a job runs exactly once, a dependent job starts only after its dependency completes, a parallel-for job covers its range exactly once, and a handle completes, reports its state, and combines its dependencies",
                location: "crates/unitas-jobs/src/lib.rs",
                evidence: &[
                    "crates/unitas-jobs/tests/job_system_tests.rs::every_scheduled_job_runs_exactly_once",
                    "crates/unitas-jobs/tests/job_system_tests.rs::dependent_job_observes_its_dependency_completed",
                    "crates/unitas-jobs/tests/job_system_tests.rs::parallel_for_covers_the_range_exactly_once",
                    "crates/unitas-jobs/tests/job_system_tests.rs::complete_waits_for_the_job_and_is_complete_reports_the_state",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a managed job schedules through the job API, waits for its dependency, and completes only after a combined dependency completes",
                location: "bridge/UnityEngine/Jobs/Job.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/JobSystemTests.cs::ParallelForMatchesSequentialResult",
                    "bridge/Unitas.Bridge.Tests/JobSystemTests.cs::DependentJobWaitsForItsDependency",
                    "bridge/Unitas.Bridge.Tests/JobSystemTests.cs::CombinedHandleCompletesAfterBoth",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a native container allocates with the Temp and Persistent allocators, refuses a second disposal, rejects an access while a job holds it, and reports an undisposed allocation as a leak",
                location: "bridge/UnityEngine/Collections/NativeArray.cs",
                evidence: &[
                    "bridge/Unitas.Bridge.Tests/JobSystemTests.cs::SafetyRejectsAccessWhileTheJobRuns",
                    "bridge/Unitas.Bridge.Tests/JobSystemTests.cs::DoubleDisposeThrows",
                    "bridge/Unitas.Bridge.Tests/JobSystemTests.cs::UndisposedArrayIsReported",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "code marked [BurstCompile] produces results equivalent to its managed execution within the floating-point mode the attribute declares",
                location: "bridge/Unitas.Scripting/ScriptCompilation.cs",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no BurstCompile attribute and the compiler performs no Burst compilation, so code marked [BurstCompile] does not compile and no Burst-compiled result is produced",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the Unity.Mathematics types and functions have the same semantics in managed and Burst-compiled code",
                location: "bridge/UnityEngine/UnityEngine.csproj",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no Unity.Mathematics type, so a project that uses Unity.Mathematics does not compile",
                ),
            },
            OwnedCapabilityPart {
                behavior: "the Entities package's worlds, entities, systems, baking, and subscenes behave as the package defines",
                location: "bridge/UnityEngine/UnityEngine.csproj",
                evidence: &[],
                missing: Some(
                    "the UnityEngine assembly defines no Unity.Entities type, so no world, entity, system, baker, or subscene exists and the com.unity.entities package stays unsupported",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "EDT-004",
        owning_spec: "editor-and-authoring.md",
        capability: "Enter, pause, frame-step, and exit play mode, and restore the edit-time state",
        parts: &[
            OwnedCapabilityPart {
                behavior: "play mode starts an isolated play session, pauses it, advances a frame, injects input, and exits it, and the edit-time scene state is captured on entry and restored on exit",
                location: "editor/Unitas.Editor/Play/PlayTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/AuthoringToolsTests.cs::PlayModeRestoresTheEditTimeSceneState",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a runtime value observed during play mode is applied to the edit-time scene and its destination is named",
                location: "editor/Unitas.Editor/Registry/SceneTools.cs",
                evidence: &[],
                missing: Some(
                    "play.set_runtime_property applies the change to the open scene only while the play session runs, keeps it out of the undo history, and the editor discards it when the session ends, so no command applies a runtime value to the edit-time scene or names its destination",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "EDT-005",
        owning_spec: "editor-and-authoring.md",
        capability: "Undo and redo of a person's and an agent session's edits",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a scene edit is undone and redone exactly, an empty history returns a structured error, and the mutating call is recorded in a change set",
                location: "editor/Unitas.Editor/Registry/SceneTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/SceneToolsTests.cs::SetPropertyUndoAndRedoRoundTrip",
                    "editor/Unitas.Editor.Tests/Registry/SceneToolsTests.cs::UndoWithAnEmptyHistoryReturnsAStructuredError",
                    "editor/Unitas.Editor.Tests/Registry/SceneToolsTests.cs::MutatingCallRecordsAChangeSet",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an agent session's undo applies only to that session's own operations and names the conflicting operation when another actor changed the same content",
                location: "editor/Unitas.Editor/Core/UndoHistory.cs",
                evidence: &[],
                missing: Some(
                    "the undo history is one stack per open document whose entries record only the forward and the inverse operation, so an undo applies to another actor's operation and no conflict naming that operation is returned",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "EDT-006",
        owning_spec: "editor-and-authoring.md",
        capability: "Named editor commands with their availability, their disabled reason, and their MCP parity",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the command palette lists every registered tool and every project menu command with its identifier, its display name, its description, its required scope, its availability, and its disabled reason, and a command runs by its identifier",
                location: "editor/Unitas.Editor/Registry/CommandPaletteTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/AuthoringToolsTests.cs::TheCommandPaletteListsTheRegisteredCommands",
                    "editor/Unitas.Editor.Tests/Registry/AuthoringToolsTests.cs::TheCommandPaletteRunsACommand",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a project-contributed editor method is discovered from the compiled project assembly and registered as a tool with the execute-project-code scope",
                location: "editor/Unitas.Editor/Registry/ExecuteMethodTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/ExecuteMethodToolsTests.cs::DiscoversEditorCallableMethodsAndEmitsGlueRegistration",
                    "editor/Unitas.Editor.Tests/Registry/ExecuteMethodToolsTests.cs::ToolIsRegisteredWithExecuteProjectCodeScopeAndMethodSchema",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a shortcut is assigned per user and a conflicting assignment names the other command, and every drag-and-drop outcome is available as a command",
                location: "editor/Unitas.Editor/Registry/CommandPaletteTools.cs",
                evidence: &[],
                missing: Some(
                    "the palette reads the shortcut of a menu command and searches only names and descriptions, so no command assigns a shortcut or reports a shortcut conflict, and no drag-and-drop outcome is available as a command",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "EDT-008",
        owning_spec: "editor-and-authoring.md",
        capability: "Agent change review: change sets under a review policy, and accepting, rejecting, and reverting them",
        parts: &[
            OwnedCapabilityPart {
                behavior: "an agent mutation is held under the project review policy, a person approves it and the transition is audited, and a rejection restores the files the change set changed",
                location: "editor/Unitas.Editor/Core/ChangeSetStore.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/ChangeReviewTests.cs::AgentMutationIsHeldUnderReviewPolicy",
                    "editor/Unitas.Editor.Tests/ChangeReviewTests.cs::PersonApproveAppliesHeldMutationAndAuditsTransitions",
                    "editor/Unitas.Editor.Tests/ChangeReviewTests.cs::RejectRestoresTheFilesTheChangeSetChanged",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "each change-set item records the agent session, the identity, the time, the request that produced it, and its affected identities, and the change set lists the risk flags of its items",
                location: "editor/Unitas.Editor/Core/ChangeSet.cs",
                evidence: &[],
                missing: Some(
                    "a change set records its id, its author, its creation time, its status, and its operations, and an operation records only the tool and its arguments, so no item names the request that produced it and no risk flag marks a change to tests, to scenario expectations, to tolerances, to project settings, to package manifests, to editor-time scripts, or to native plugins",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "AUT-001",
        owning_spec: "automation-and-mcp.md",
        capability: "Feature parity between the editor and automation, over any MCP client and without network access",
        parts: &[
            OwnedCapabilityPart {
                behavior: "every registered editor command is exposed as an MCP tool with the scope it requires, over the stdio transport and the HTTP transport",
                location: "editor/Unitas.Editor/Registry/ToolRegistry.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/AuthoringToolsTests.cs::RegistersAuthoringToolsWithLeastPrivilegeScopes",
                    "editor/Unitas.Editor.Tests/Registry/SceneToolsTests.cs::RegistersTheSceneToolsWithLeastPrivilegeScopes",
                    "editor/Unitas.Editor.Tests/Registry/SceneToolsTests.cs::TheLiveHostRegistersTheSceneTools",
                    "editor/Unitas.Editor.Tests/Transport/StdioTransportTests.cs::StdioInitializeToolsAndResourcesRoundTrip",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a project-contributed editor method is discovered from the compiled project assembly and registered as a tool with its own scope",
                location: "editor/Unitas.Editor/Registry/ExecuteMethodTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/ExecuteMethodToolsTests.cs::DiscoversEditorCallableMethodsAndEmitsGlueRegistration",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a binary asset is created or imported by a command",
                location: "crates/unitas-editor-abi/src/asset_tools.rs",
                evidence: &[],
                missing: Some(
                    "the editor ABI exposes assets.search, assets.preview, assets.move, assets.delete, assets.restore, assets.duplicate, assets.favorites, assets.recents, and assets.dependencies, and project.write_file writes text only, so no command creates or imports a binary asset and the agent authoring benchmark records that absence as an AUT-001 parity finding",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "AUT-002",
        owning_spec: "automation-and-mcp.md",
        capability: "Capability discovery, structured inspection, and structured editing over MCP",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the outcome of a mutating request is queryable by its operation identifier and a running operation reports its stage",
                location: "editor/Unitas.Editor/Registry/OperationTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/OperationToolsTests.cs::GetReturnsOneOperationAndRefusesAnUnknownId",
                    "editor/Unitas.Editor.Tests/Registry/OperationToolsTests.cs::ListReturnsTheHostOperationsInStartOrder",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the project's tests and its declared scenarios run through MCP and report their outcome",
                location: "editor/Unitas.Editor/Play/ScenarioTools.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/AuthoringToolsTests.cs::TestRunReturnsAPassFailSummary",
                    "editor/Unitas.Editor.Tests/Play/ScenarioToolsTests.cs::RegisteredToolRunsCollectAllPickupsWithAllCheckpointsPassed",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the score, the multiplier, and the timer of the running sample are readable as structured state",
                location: "crates/unitas-build/src/control.rs",
                evidence: &[],
                missing: Some(
                    "the worker's play-state payload carries score, total, won, and script_host only, and the editor's play.state payload reports the session, the state, the frames, delta_seconds, random_seed, and crashed, so the sample's multiplier and timer are not readable through the editor's MCP surface although the sample specification requires an agent to read score, multiplier, and timer state",
                ),
            },
            OwnedCapabilityPart {
                behavior: "discovery reports the version of each tool and resource schema and the release in which a deprecation ends",
                location: "editor/Unitas.Editor/Registry/ToolRegistry.cs",
                evidence: &[],
                missing: Some(
                    "a registered tool carries its name, its description, its input schema, and its required scope, so discovery reports no schema version and no deprecation for a tool or a resource",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "AUT-007",
        owning_spec: "automation-and-mcp.md",
        capability: "Permission scopes, person grants, and the confirmation boundary",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a request without the scope a tool requires is refused with the missing scope named and changes nothing, and a session cannot widen its own scopes",
                location: "editor/Unitas.Editor/Registry/McpHost.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/AuthoringToolsTests.cs::WriteRequiresTheEditScopes",
                    "editor/Unitas.Editor.Tests/Registry/SceneToolsTests.cs::MutatingCallWithoutTheEditScopeIsRefused",
                    "editor/Unitas.Editor.Tests/Registry/ExecuteMethodToolsTests.cs::RefusesCallerWithoutExecuteProjectCodeScope",
                    "editor/Unitas.Editor.Tests/Transport/HttpTransportTests.cs::HttpSelfWideningAndUnknownPrimitiveErrorsAreMappedAndAudited",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "an irreversible or externally visible request returns a confirmation-required outcome and proceeds only after a person confirms it or under a standing grant",
                location: "editor/Unitas.Editor/Core/Errors.cs",
                evidence: &[],
                missing: Some(
                    "the editor defines no confirmation-required outcome and checks no standing grant before an irreversible action, so a deletion outside the undo history, a deployment, a signing, or an external-network request proceeds without a person confirming it",
                ),
            },
            OwnedCapabilityPart {
                behavior: "every scope discovery lists is the scope a registered tool requires, so a session that holds that scope can exercise it",
                location: "editor/Unitas.Editor/Core/Scope.cs",
                evidence: &[],
                missing: Some(
                    "the import-external-files, change-packages-and-settings, build, sign, deploy, and external-network scopes are declared, but no registered tool requires them, so a session that holds them exercises nothing",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "AUT-008",
        owning_spec: "automation-and-mcp.md",
        capability: "Audit log and provenance of an agent session's requests",
        parts: &[
            OwnedCapabilityPart {
                behavior: "a tool call, a refusal, and a grant are recorded with the time, the session identity, the target, the scope, the outcome, and the affected identities",
                location: "editor/Unitas.Editor/Core/Audit.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Registry/ExecuteMethodToolsTests.cs::RecordsAnAuditEntryForTheInvocation",
                    "editor/Unitas.Editor.Tests/ChangeReviewTests.cs::PersonApproveAppliesHeldMutationAndAuditsTransitions",
                    "editor/Unitas.Editor.Tests/Transport/HttpTransportTests.cs::ConcurrentClientsKeepTheirOwnAuditIdentity",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "the provenance of an accepted agent change is queryable: which agent session made each change, under which request, and who accepted it",
                location: "editor/Unitas.Editor/Core/Audit.cs",
                evidence: &[],
                missing: Some(
                    "the audit record carries the session, the identity, the target, the scope, the outcome, and the affected identities, and a change set carries the tool and its arguments, so no record names the request that produced an agent change",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-001",
        owning_spec: "rendering-and-animation.md",
        capability: "Cameras, viewports, displays, and render targets",
        parts: &[
            OwnedCapabilityPart {
                behavior: "perspective and orthographic camera projection matrices and view transforms render offscreen and to window surfaces",
                location: "crates/unitas-render/src/camera.rs",
                evidence: &[
                    "crates/unitas-render/tests/render/render_tests.rs::perspective_projection_matrix_matches_unity_formulas",
                    "crates/unitas-render/tests/render/render_tests.rs::orthographic_projection_matrix_matches_formulas",
                    "crates/unitas-render/tests/render/render_tests.rs::perspective_camera_renders_cube_center_and_background",
                    "crates/unitas-render/tests/render/render_tests.rs::orthographic_camera_renders_cube_center_and_background",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "scene cameras materialize with field of view, clip planes, orthographic flag, and background color, and WorldToScreenPoint, ViewportToWorldPoint, and ScreenPointToRay agree with rendered geometry and object ID capture",
                location: "crates/unitas-runtime/src/runtime.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/render_tests.rs::render_frame_produces_bmp_for_scene_with_camera_and_renderer",
                    "crates/unitas-runtime/tests/runtime/render_tests.rs::world_to_screen_point_agrees_with_rendered_geometry",
                    "bridge/Unitas.Bridge.Tests/SceneCameraTests.cs::SceneCameraIsMaterialized",
                    "bridge/Unitas.Bridge.Tests/RenderTests.cs::TestEndToEndRender",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "multi-camera depth and stack ordering, culling masks, viewport rectangles, clear flags, RenderTexture targets, multi-display routing, camera callbacks, and depth textures are rendered",
                location: "crates/unitas-render/src/camera.rs",
                evidence: &[],
                missing: Some(
                    "multi-camera depth and stack ordering, culling masks, viewport rectangles, clear flags, RenderTexture assets and Camera.targetTexture, multi-display routing (Display.displays), camera callbacks (OnPreCull, OnPreRender, OnPostRender, OnRenderImage), and _CameraDepthTexture are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-002",
        owning_spec: "rendering-and-animation.md",
        capability: "Meshes, skinning, blend shapes, materials, textures, and shaders",
        parts: &[
            OwnedCapabilityPart {
                behavior: "cube, UV sphere, and buffer-backed triangle meshes with normals, UVs, and tangents are constructed and rendered",
                location: "crates/unitas-render/src/mesh.rs",
                evidence: &[
                    "crates/unitas-render/tests/render/render_tests.rs::cube_mesh_has_expected_counts_bounds_and_unit_normals",
                    "crates/unitas-render/tests/render/render_tests.rs::uv_sphere_mesh_has_exact_vertex_and_index_counts_and_unit_normals",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Unity YAML .mat material assets parse shader GUIDs, colors, and main texture GUIDs and render through offscreen and translucent material passes",
                location: "crates/unitas-asset/src/material.rs",
                evidence: &[
                    "crates/unitas-asset/tests/material_tests.rs::material_fixture_parses_shader_color_and_main_texture",
                    "crates/unitas-render/tests/render/integration_tests.rs::material_to_pixel_end_to_end_pipeline",
                    "crates/unitas-render/tests/translucency.rs::translucent_quad_blends_over_opaque_background",
                    "crates/unitas-render/tests/translucency.rs::translucent_quads_are_drawn_back_to_front",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "skinned mesh bone deformation and blend shapes, ShaderLab/HLSL compilation and keyword variants, MaterialPropertyBlock, runtime Texture2D pixel mutation and GPU readback, and non-triangle mesh topologies are supported",
                location: "crates/unitas-render/src/model.rs",
                evidence: &[],
                missing: Some(
                    "skinned mesh bone deformation and blend shapes, ShaderLab/HLSL compilation and keyword variants, MaterialPropertyBlock, runtime Texture2D pixel mutation and readback (SetPixels, Apply, ReadPixels, AsyncGPUReadback), and non-triangle mesh topologies are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-005",
        owning_spec: "rendering-and-animation.md",
        capability: "Sprites, atlases, sorting, masks, and tilemaps",
        parts: &[
            OwnedCapabilityPart {
                behavior: "Sprite Atlas v1 and v2 assets and sprite sub-assets in texture metadata are recognized during project indexing, and object-reference animation curves bind SpriteRenderer.m_Sprite across clip evaluation and binary wire round-trips",
                location: "crates/unitas-asset/src/database.rs",
                evidence: &[
                    "crates/unitas-asset/tests/asset_import/database_tests.rs::prj002_prj004_outcome17_json_and_bytes_import_as_text_asset_and_avatar_mask_audio_mixer_and_sprite_atlas_v2_are_identified_by_family",
                    "crates/unitas-animation/tests/animation/clip_tests.rs::clip_object_reference_curve_steps_like_a_float_curve_across_the_loop_boundary",
                    "crates/unitas-animation/tests/animation/clip_tests.rs::clip_object_reference_curve_is_sampled_for_every_binding_in_the_clip",
                    "crates/unitas-animation/tests/animation/wire_tests.rs::wire_v2_round_trips_an_object_reference_curve",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "SpriteAtlas packing and runtime atlas binding, SortingGroup, SpriteMask, 2D lights, Grid and Tilemap/TilemapRenderer, PixelPerfectCamera, and 2D SpriteSkin deformation are rendered",
                location: "crates/unitas-render/src/ui_pass.rs",
                evidence: &[],
                missing: Some(
                    "SpriteAtlas packing and runtime atlas binding, SortingGroup and SpriteMask, 2D lights, Grid and Tilemap/TilemapRenderer, PixelPerfectCamera, and 2D SpriteSkin deformation are not implemented",
                ),
            },
            OwnedCapabilityPart {
                behavior: "SpriteRenderer Sliced and Tiled draw modes render nine-slice and tiled geometry built from the sprite's imported border, and the Simple draw mode is unchanged",
                location: "crates/unitas-render/src/sprite.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime_sprite_tests.rs::scene_sprite_sliced_draw_mode_draws_the_nine_slice_parts",
                    "crates/unitas-runtime/tests/runtime_sprite_tests.rs::sliced_sprite_draws_the_nine_slice_regions_into_the_frame",
                    "crates/unitas-runtime/tests/runtime_sprite_tests.rs::script_sprite_tiled_draw_mode_repeats_the_sprite_rect",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-006",
        owning_spec: "rendering-and-animation.md",
        capability: "Render pipelines, shader graphs, and scripted rendering",
        parts: &[
            OwnedCapabilityPart {
                behavior: "the default render graph orders shadow, opaque, transparent, UI overlay, and object-ID capture passes, and Shader Graph compatibility assessment classifies Built-in, URP, HDRP, and unrepresentable nodes",
                location: "crates/unitas-render/src/render_graph.rs",
                evidence: &[
                    "crates/unitas-render/src/render_graph.rs::offscreen_pass_order_with_and_without_shadows",
                    "crates/unitas-render/src/render_graph.rs::surface_pass_order_with_and_without_shadows",
                    "crates/unitas-render/src/render_graph.rs::capture_pass_order_with_and_without_shadows",
                    "crates/unitas-core/tests/core/compat_tests.rs::cmp_002_story_22_shader_graph_unrepresentable_node_reports_graph_and_node_and_hdrp_target_is_unsupported",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "URP 14.x and HDRP 14.x pipeline asset import and translation, custom RenderPipelineAsset and ScriptableRenderContext execution, Shader Graph asset import and graph editing, CommandBuffer and ComputeShader dispatch, decals, projectors, and stereo XR rendering are executed",
                location: "crates/unitas-render/src/render_graph.rs",
                evidence: &[],
                missing: Some(
                    "URP 14.x and HDRP 14.x pipeline asset import and translation, custom RenderPipelineAsset and ScriptableRenderContext execution, Shader Graph asset import and visual or MCP graph editing, CommandBuffer and ComputeShader dispatch, decals, projectors, and stereo XR rendering are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "REN-007",
        owning_spec: "rendering-and-animation.md",
        capability: "Frame presentation, frame rate, and display output",
        parts: &[
            OwnedCapabilityPart {
                behavior: "windowed wgpu presentation renders scene geometry, directional shadows, and canvas UI overlays to a window surface and reports sustained frame-rate metrics while headless sessions render offscreen",
                location: "crates/unitas-runtime/src/window.rs",
                evidence: &[
                    "crates/unitas-runtime/src/presenter.rs::windowed_present_shows_the_canvas_the_capture_path_composes",
                    "crates/unitas-runtime/src/window.rs::bound_scene_renders_non_flat_frame_on_gpu",
                    "crates/unitas-runtime/src/window.rs::sustained_frames_per_second_divides_presented_frames_by_elapsed_time",
                    "crates/unitas-runtime/src/window.rs::sustained_fps_report_line_uses_the_device_lane_greppable_format",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Application.targetFrameRate and QualitySettings.vSyncCount runtime pacing, Screen.SetResolution and FullScreenMode switching, multi-display output, and HDR display output are enforced",
                location: "crates/unitas-runtime/src/window.rs",
                evidence: &[],
                missing: Some(
                    "Application.targetFrameRate and QualitySettings.vSyncCount runtime frame pacing enforcement, Screen.SetResolution and FullScreenMode switching, multi-display output (Display.displays), and HDR display output are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "UI-001",
        owning_spec: "ui-audio-input.md",
        capability: "Canvas, layout, controls, events, focus, and navigation",
        parts: &[
            OwnedCapabilityPart {
                behavior: "Canvas, CanvasScaler, and RectTransform anchor and pivot layout resolve screen-space overlay rectangles and composite UI quads, text, and object IDs onto rendered frames",
                location: "crates/unitas-runtime/src/canvas.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/canvas_tests.rs::canvas_anchor_math_matches_unity_layout",
                    "crates/unitas-runtime/tests/canvas_tests.rs::canvas_scale_factor_follows_the_unity_match_rule",
                    "crates/unitas-runtime/tests/canvas_tests.rs::canvas_draw_items_follow_the_scaler_and_the_anchors",
                    "crates/unitas-runtime/tests/canvas_tests.rs::canvas_fixture_composites_ui_pixels_and_object_ids",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "buttons, labels, pointer hit-testing, keyboard and gamepad focus navigation, and Button.onClick dispatch work across the runtime and C# bridge",
                location: "crates/unitas-runtime/src/ui_components.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/pointer_ui.rs::pointer_click_activates_a_button_headless",
                    "crates/unitas-runtime/tests/pointer_ui.rs::keyboard_focus_still_cycles_and_enter_submits",
                    "crates/unitas-runtime/tests/ui_components.rs::a_pointer_click_on_a_button_is_dispatched",
                    "crates/unitas-runtime/tests/ui_components.rs::the_ui_abi_sets_the_text_and_reports_the_click",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "a uGUI Image in Sliced mode draws the nine quads of the sprite's imported border and in Tiled mode repeats the sprite, while Simple still draws one stretched quad",
                location: "crates/unitas-runtime/src/canvas.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/ui_sliced_tests.rs::sliced_image_with_an_eight_pixel_border_draws_nine_quads_with_the_border_uvs",
                    "crates/unitas-runtime/tests/ui_sliced_tests.rs::simple_image_stays_one_stretched_quad_and_tiled_repeats_the_sprite",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "screen-space camera and world-space canvases, layout groups, masks, CanvasGroup, ScrollRect, sliders, toggles, dropdowns, input fields, raycasters, and UI Toolkit runtime documents are supported",
                location: "crates/unitas-runtime/src/canvas.rs",
                evidence: &[],
                missing: Some(
                    "screen-space camera and world-space canvases, layout groups (HorizontalLayoutGroup, VerticalLayoutGroup, GridLayoutGroup, ContentSizeFitter, AspectRatioFitter), Mask and RectMask2D, CanvasGroup, ScrollRect, sliders, toggles, dropdowns, input fields, GraphicRaycaster and PhysicsRaycaster, and UI Toolkit runtime (UIDocument, UXML, USS) are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "UI-002",
        owning_spec: "ui-audio-input.md",
        capability: "Immediate-mode runtime GUI",
        parts: &[OwnedCapabilityPart {
            behavior: "MonoBehaviour.OnGUI layout and repaint events, Event.current, GUI and GUILayout controls, GUISkin and GUIStyle, and GUI.depth ordering are dispatched and rendered",
            location: "crates/unitas-runtime/src/ui.rs",
            evidence: &[],
            missing: Some(
                "MonoBehaviour.OnGUI dispatch, Event.current, GUI and GUILayout controls, GUISkin and GUIStyle rendering, and GUI.depth ordering are not implemented",
            ),
        }],
    },
    OwnedFeatureCapability {
        feature_id: "TXT-001",
        owning_spec: "ui-audio-input.md",
        capability: "Fonts, Unicode text, fallback, and rich presentation",
        parts: &[
            OwnedCapabilityPart {
                behavior: "TrueType and OpenType fonts import with TrueTypeFontImporter meta settings, glyph metrics, kerning, and packed font atlases, reporting diagnostics on malformed fonts or unsupported containers",
                location: "crates/unitas-asset/src/font/mod.rs",
                evidence: &[
                    "crates/unitas-asset/tests/font_import_tests.rs::imports_the_shipped_font_with_the_known_metrics",
                    "crates/unitas-asset/tests/font_import_tests.rs::imports_the_kerning_the_font_carries",
                    "crates/unitas-asset/tests/font_import_tests.rs::honors_the_import_settings_of_the_meta_file",
                    "crates/unitas-asset/tests/font_import_tests.rs::reports_a_named_diagnostic_for_a_malformed_font",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "runtime text layout shapes glyphs into a GlyphAtlas and renders alpha-blended text overlays",
                location: "crates/unitas-render/src/glyphs.rs",
                evidence: &[
                    "crates/unitas-render/src/glyphs.rs::shaping_layout_and_atlas_are_deterministic",
                    "crates/unitas-render/src/glyphs.rs::draw_text_alpha_blends_over_background",
                    "crates/unitas-runtime/src/ui.rs::label_text_layout_produces_glyph_quads_at_multiple_resolutions",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "TextMeshPro assets, SDF shaders, rich-text tags, missing-glyph fallback diagnostics naming the fallback chain and locale, complex script and bidirectional text layout, and IME text entry are supported",
                location: "crates/unitas-render/src/glyphs.rs",
                evidence: &[],
                missing: Some(
                    "TextMeshPro (TextMeshProUGUI, TMP_FontAsset, SDF shaders, rich-text tags, textInfo), explicit missing-glyph diagnostics naming the fallback chain and locale, Arabic/Indic/CJK complex script and UAX #9 bidirectional text layout mode selection, and IME/clipboard/on-screen keyboard input fields are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "INP-001",
        owning_spec: "ui-audio-input.md",
        capability: "Legacy input behavior",
        parts: &[
            OwnedCapabilityPart {
                behavior: "frame-exact keyboard and mouse button transitions (GetKey, GetKeyDown, GetKeyUp, GetMouseButton*), pointer position and delta, scroll delta, and named axes (GetAxis, GetAxisRaw, GetButton*) with sensitivity, gravity, dead zone, snap, inversion, and InputManager.asset discovery work across Rust and the C# UnityEngine.Input bridge",
                location: "crates/unitas-runtime/src/input.rs",
                evidence: &[
                    "crates/unitas-runtime/tests/runtime/input_tests.rs::input_key_and_mouse_transitions_hold_for_one_frame_and_ignore_redundant_events",
                    "crates/unitas-runtime/tests/runtime/input_tests.rs::input_key_axis_ramps_with_sensitivity_decays_with_gravity_and_respects_snap_invert_and_deadzone",
                    "crates/unitas-runtime/tests/runtime/input_tests.rs::runtime_discovers_input_manager_asset_on_load_scene_and_resets_input_on_reload",
                    "bridge/Unitas.Bridge.Tests/InputTests.cs::TestFrameExactKeyTransitionsAndStringOverloads",
                    "bridge/Unitas.Bridge.Tests/InputTests.cs::TestAxisRampSnapGravityInversionAndProjectSettingsDiscovery",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "multi-touch Input.touches, Input.inputString text entry, Cursor.lockState and visibility, mobile device sensors, and InvalidOperationException enforcement when active input handling is Input System only are provided",
                location: "crates/unitas-runtime/src/input.rs",
                evidence: &[],
                missing: Some(
                    "multi-touch (Input.touches), Input.inputString text entry, Cursor.lockState and visibility, accelerometer/gyroscope/compass/location sensors, and throwing InvalidOperationException when active input handling is set to Input System only are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "INP-002",
        owning_spec: "ui-audio-input.md",
        capability: "Action-based input, devices, players, and rebinding",
        parts: &[
            OwnedCapabilityPart {
                behavior: "gamepad connection, disconnection, stick dead zones, button transitions, and force-feedback rumble handlers drive runtime input state",
                location: "crates/unitas-runtime/src/input.rs",
                evidence: &[
                    "crates/unitas-runtime/src/input.rs::gamepad_stick_and_buttons_drive_the_named_axes_and_buttons",
                    "crates/unitas-runtime/src/input.rs::gamepad_stick_dead_zone_zeroes_small_deflections",
                    "crates/unitas-runtime/src/input.rs::gamepad_hot_plug_leaves_the_keyboard_state_untouched",
                    "crates/unitas-runtime/src/input.rs::rumble_without_a_device_is_a_silent_no_op",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "Input System 1.x .inputactions asset import, InputAction phases, composites, interactions, processors, PlayerInput and PlayerInputManager, Keyboard.current and Gamepad.current C# APIs, and interactive rebinding are provided",
                location: "crates/unitas-runtime/src/input.rs",
                evidence: &[],
                missing: Some(
                    "Input System 1.x .inputactions asset import, InputAction phases (Waiting, Started, Performed, Canceled), composites, interactions, processors, PlayerInput and PlayerInputManager, Keyboard.current/Gamepad.current C# APIs, and interactive rebinding (PerformInteractiveRebinding, SaveBindingOverridesAsJson) are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "AUD-001",
        owning_spec: "ui-audio-input.md",
        capability: "Clips, sources, listeners, spatial audio, filters, and mixing",
        parts: &[
            OwnedCapabilityPart {
                behavior: "WAV, AIFF, Ogg Vorbis, and MP3 decoding, streamed Ogg music playback, AudioSource playback states and SilenceReason diagnostics, 3D distance attenuation, stereo panning, Doppler pitch shift, sample-accurate play_scheduled, and Sfx/Music/Menu mixer group buses and device handoff are executed",
                location: "crates/unitas-audio/src/lib.rs",
                evidence: &[
                    "crates/unitas-audio/tests/audio_decode_tests.rs::decoded_clip_mixes_through_audio_world",
                    "crates/unitas-audio/src/lib.rs::group_gains_apply_only_to_their_own_bus",
                    "crates/unitas-audio/src/lib.rs::doppler_raises_pitch_for_an_approaching_source_and_lowers_it_for_a_receding_one",
                    "crates/unitas-audio/src/lib.rs::sources_scheduled_at_adjacent_samples_render_without_gap_or_overlap",
                    "crates/unitas-audio/src/output.rs::handoff_drains_the_exact_frames_that_were_pushed",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "AudioMixer asset import, snapshots, exposed parameters, built-in mixer DSP effects, AudioReverbZone, ambisonic decoding, GetOutputData/GetSpectrumData, and Microphone capture are supported",
                location: "crates/unitas-audio/src/lib.rs",
                evidence: &[],
                missing: Some(
                    "AudioMixer asset import, snapshots (TransitionTo), exposed parameters, and built-in mixer DSP effects, AudioDistortionFilter, AudioReverbFilter, AudioChorusFilter, OnAudioFilterRead, AudioReverbZone, ambisonic decoding, GetOutputData/GetSpectrumData, and Microphone capture are not implemented",
                ),
            },
            OwnedCapabilityPart {
                behavior: "AudioLowPassFilter, AudioHighPassFilter, and AudioEchoFilter on an AudioSource process the source's audio with a biquad low-pass, a biquad high-pass, and a feedback-delay echo on the real mix path",
                location: "crates/unitas-audio/src/fx.rs",
                evidence: &[
                    "crates/unitas-audio/tests/audio_fx_tests.rs::low_pass_filter_attenuates_8khz_by_30db_and_passes_200hz",
                    "crates/unitas-audio/tests/audio_fx_tests.rs::high_pass_filter_attenuates_200hz_by_30db_and_passes_8khz",
                    "crates/unitas-audio/tests/audio_fx_tests.rs::echo_filter_repeats_after_the_delay_and_decays",
                    "crates/unitas-runtime/tests/audio_filter_abi_tests.rs::runtime_abi_audio_filters_round_trip",
                ],
                missing: None,
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "MED-001",
        owning_spec: "ui-audio-input.md",
        capability: "Video playback and synchronized media state",
        parts: &[OwnedCapabilityPart {
            behavior: "VideoPlayer and VideoClip import, container and codec decoding (H.264, H.265, VP8, VP9, AV1), camera plane/render texture/material override render modes, audio track routing, DSP/game time synchronization, and HTTP/HLS/DASH streaming are executed",
            location: "crates/unitas-asset/src/database.rs",
            evidence: &[],
            missing: Some(
                "VideoPlayer and VideoClip import, container and codec decoding (H.264, H.265, VP8, VP9, AV1), camera plane/render texture/material override render modes, audio track routing, DSP/game time synchronization, and HTTP/HLS/DASH streaming are not implemented",
            ),
        }],
    },
    OwnedFeatureCapability {
        feature_id: "BLD-001",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Configure, validate, create, and report player builds",
        parts: &[
            OwnedCapabilityPart {
                behavior: "BuildConfiguration validates scenes, product identity, target, and development diagnostics settings, stages deterministic content-hashed bundles and desktop packages (.app on macOS, executable + <Product>_Data on Windows and Linux), writes build-report.json with size contributors, compatibility findings, licensing, startup measurement, and target availability, and exposes build.player over the editor ABI",
                location: "crates/unitas-build/src/player.rs",
                evidence: &[
                    "crates/unitas-build/tests/build/build_and_launch.rs::end_to_end_build_report_determinism_and_headless_player_launch",
                    "crates/unitas-build/tests/build/build_and_launch.rs::failed_build_validation_and_missing_source_scene_produce_failed_build_report",
                    "crates/unitas-build/src/package.rs::desktop_packaging_stages_three_targets_with_build_reports",
                    "crates/unitas-editor-abi/src/build_tools.rs::build_player_stages_a_bundle_and_writes_the_licensing_record",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "incremental build cache reuse after cancelled or failed builds, Android Gradle/APK/AAB and iOS Xcode project generation, platform distribution signing and notarization status reporting, and WebGL/WebAssembly browser bundle generation are provided",
                location: "crates/unitas-build/src/package.rs",
                evidence: &[],
                missing: Some(
                    "incremental build cache reuse after cancelled or failed builds, Android Gradle/APK/AAB and iOS Xcode project generation, platform distribution signing and notarization status reporting, and WebGL/WebAssembly browser bundle generation are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "BLD-004",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Deployment, device connection, and development-player channels",
        parts: &[OwnedCapabilityPart {
            behavior: "connected device discovery, deploying/installing/launching/removing built players on Android, iOS, XR, handheld, or console devices, and pairing-credential-authenticated remote profiler, debugger, and log-stream connections to development players are provided",
            location: "crates/unitas-build/src/player.rs",
            evidence: &[],
            missing: Some(
                "connected device discovery, deploying/installing/launching/removing built players on Android, iOS, XR, handheld, or console devices, and pairing-credential-authenticated remote profiler, debugger, and log-stream connections to development players are not implemented",
            ),
        }],
    },
    OwnedFeatureCapability {
        feature_id: "PLT-001",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Desktop, mobile, browser, handheld, and console behavior",
        parts: &[
            OwnedCapabilityPart {
                behavior: "desktop player bundles launch in headless and windowed GPU modes, enforce PRJ-008 project trust before executing code, pause simulation and audio and clear held input on window focus loss and restore on focus gain, and report non-host desktop targets as ExternallyUnavailable with the cause",
                location: "crates/unitas-build/src/player.rs",
                evidence: &[
                    "crates/unitas-build/tests/build/build_and_launch.rs::startup_failures_are_distinct_for_missing_manifest_missing_scene_and_corrupt_scene",
                    "crates/unitas-build/tests/trust_gate.rs::untrusted_project_cannot_run_its_code",
                    "crates/unitas-runtime/src/window.rs::focus_loss_clears_input_and_freezes_the_player_until_focus_returns",
                    "crates/unitas-build/src/targets.rs::marks_non_host_targets_unavailable_on_current_host",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "WebGL/WebGPU browser targets, Android and iOS/tvOS mobile targets, UWP, Dedicated Server (UNITY_SERVER) target, Linux Wayland and Steam Deck handheld integration, and licensed console targets are supported",
                location: "crates/unitas-build/src/targets.rs",
                evidence: &[],
                missing: Some(
                    "WebGL/WebGPU browser target, Android and iOS/tvOS mobile targets, UWP, Dedicated Server (UNITY_SERVER) target, Linux Wayland/Steam Deck handheld integration, and licensed console targets are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "LIC-001",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Dual licensing of the product and license obligations of shipped players",
        parts: &[
            OwnedCapabilityPart {
                behavior: "every player build generates a LicensingRecord stating the governing GPLv3 license, conveyance and Corresponding Source obligations, no-linking-exception notice, commercial license route, not-legal-advice disclaimer, and third-party Cargo and NuGet notices, embeds LICENSE and THIRD-PARTY-NOTICES.txt in packaged players, and exposes build.licensing in the editor",
                location: "crates/unitas-build/src/licensing.rs",
                evidence: &[
                    "crates/unitas-editor-abi/src/build_tools.rs::build_player_stages_a_bundle_and_writes_the_licensing_record",
                    "crates/unitas-build/tests/build/target_availability.rs::build_report_carries_build_identifier_licensing_record_and_startup_time",
                    "editor/Unitas.Editor.Tests/BuildToolsTests.cs::LicensingQueryReturnsTheRecordTheBundleCarries",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "commercial license configuration recognition and pre-build expiry checks, and the per-target GPLv3 conflict table flagging console and restricted-store targets as requiring a commercial license, are enforced",
                location: "crates/unitas-build/src/licensing.rs",
                evidence: &[],
                missing: Some(
                    "commercial license configuration recognition and pre-build expiry checks, and the per-target GPLv3 conflict table flagging console and restricted-store targets as requiring a commercial license, are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "PRD-001",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Installation, updates, and target toolchain acquisition",
        parts: &[
            OwnedCapabilityPart {
                behavior: "target toolchain discovery probes the host for each desktop target's required C compiler, resource compiler, and Rust standard library target, recording available or externally_unavailable with the cause (current_host or missing_toolchain), tool name, found version, required range, and resolving action",
                location: "crates/unitas-build/src/toolchain.rs",
                evidence: &[
                    "crates/unitas-build/tests/build/target_availability.rs::every_reported_target_carries_a_complete_packaging_record",
                    "crates/unitas-build/tests/build/target_availability.rs::a_missing_cross_toolchain_is_recorded_with_the_tool_the_range_and_the_fix",
                    "crates/unitas-build/tests/build/target_availability.rs::windows_and_linux_packaging_name_the_cross_toolchain_each_target_requires",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "side-by-side unitas installation management, recording the last-saved unitas version in project state, stable and preview update channels with signature verification and rollback, offline installation packages, and Android SDK/NDK/JDK, Xcode, browser, and .NET toolchain acquisition are provided",
                location: "crates/unitas-build/src/toolchain.rs",
                evidence: &[],
                missing: Some(
                    "side-by-side unitas installation management, recording the last-saved unitas version in project state, stable/preview update channels with signature verification and rollback, offline installation packages, and Android SDK/NDK/JDK, Xcode, browser, and .NET toolchain acquisition are not implemented",
                ),
            },
        ],
    },
    OwnedFeatureCapability {
        feature_id: "XR-001",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Tracking, views, interaction, haptics, focus, and comfort",
        parts: &[OwnedCapabilityPart {
            behavior: "OpenXR and XR Plugin Management loader lifecycle, stereo HMD pose tracking and view projection, XR Interaction Toolkit controllers/interactors/locomotion, XR Hands joint tracking, and XR haptics are executed",
            location: "crates/unitas-core/src/compat.rs",
            evidence: &[],
            missing: Some(
                "OpenXR and XR Plugin Management loader lifecycle, stereo HMD pose tracking and view projection, XR Interaction Toolkit controllers/interactors/locomotion, XR Hands joint tracking, and XR haptics are not implemented",
            ),
        }],
    },
    OwnedFeatureCapability {
        feature_id: "NET-001",
        owning_spec: "build-platforms-xr-networking.md",
        capability: "Sessions, connections, ownership, synchronization, and failure",
        parts: &[OwnedCapabilityPart {
            behavior: "Unity Transport (NetworkDriver, pipelines, network simulator), Netcode for GameObjects (NetworkManager, NetworkObject, NetworkBehaviour, NetworkVariable, RPCs, scene synchronization), and Netcode for Entities are executed",
            location: "crates/unitas-core/src/compat.rs",
            evidence: &[],
            missing: Some(
                "Unity Transport (NetworkDriver, pipelines, network simulator), Netcode for GameObjects (NetworkManager, NetworkObject, NetworkBehaviour, NetworkVariable, RPCs, scene synchronization), and Netcode for Entities are not implemented",
            ),
        }],
    },
    OwnedFeatureCapability {
        feature_id: "DIA-003",
        owning_spec: "diagnostics-profiling-playtesting.md",
        capability: "Player crash reports, symbols, and device logs",
        parts: &[
            OwnedCapabilityPart {
                behavior: "when a play-mode worker process crashes or exits unexpectedly, the editor play session detects the crash, marks crashed: true and state: session-ended, fails the active operation without taking down sibling sessions or the MCP host, and windowed player startup failures report their stage and exit nonzero",
                location: "editor/Unitas.Editor/Play/PlaySession.cs",
                evidence: &[
                    "editor/Unitas.Editor.Tests/Play/PlayAndCaptureTests.cs::WorkerCrashEndsOnlyThatPlaySessionAndHostContinues",
                    "crates/unitas-build/tests/windowed_startup_failure.rs::windowed_startup_failures_name_their_reason_and_exit_nonzero",
                ],
                missing: None,
            },
            OwnedCapabilityPart {
                behavior: "structured player crash reports capturing build identifier, device model, OS, and all managed and native thread stacks, per-build symbol file emission and symbolication, configurable crash report destinations, and live device log streaming are provided",
                location: "crates/unitas-editor-abi/src/diagnostics_tools.rs",
                evidence: &[],
                missing: Some(
                    "structured player crash reports capturing build identifier, device model, OS, and all managed and native thread stacks, per-build symbol file emission (PDB, dSYM, Android native symbols, Web source maps) and symbolication, configurable crash report destinations, and live device log streaming (Player.log, logcat, iOS console, browser console) are not implemented",
                ),
            },
        ],
    },
];
pub fn lookup_owned_feature_capability(
    feature_id: &str,
) -> Option<&'static OwnedFeatureCapability> {
    OWNED_FEATURE_CAPABILITIES
        .iter()
        .find(|entry| entry.feature_id == feature_id)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OwnedFeatureState {
    pub feature_id: String,
    pub tier: Tier,
    pub state: CompatibilityState,
    pub capability: String,
    pub gap: Option<String>,
}

pub fn probe_owned_feature(feature_id: &str) -> Option<OwnedFeatureState> {
    let capability = lookup_owned_feature_capability(feature_id)?;
    Some(OwnedFeatureState {
        feature_id: capability.feature_id.to_string(),
        tier: owning_feature_tier(capability.feature_id, None),
        state: capability.state(),
        capability: capability.capability.to_string(),
        gap: capability.gap(),
    })
}

pub fn probe_owned_features() -> Vec<OwnedFeatureState> {
    OWNED_FEATURE_CAPABILITIES
        .iter()
        .filter_map(|capability| probe_owned_feature(capability.feature_id))
        .collect()
}

pub(super) fn owned_capability_gap_suffix(
    base: &str,
    capability: Option<&'static OwnedFeatureCapability>,
) -> String {
    let Some(capability) = capability else {
        return String::new();
    };
    let mut suffix = String::new();
    for part in capability.missing_parts() {
        let missing = part.missing.unwrap_or(part.behavior);
        if base.contains(missing) {
            suffix.push_str(&format!(" ({})", part.location));
        } else {
            suffix.push_str(&format!("; {missing} ({})", part.location));
        }
    }
    suffix
}
