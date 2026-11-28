use super::compat_catalog::lookup_catalog_feature;
use super::compat_finding::sanitize_content_identity;
use super::compat_finding::validate_finding_input;
use super::compat_finding::{FindingInput, FindingSubmissionError};
use super::{CompatibilityState, ContentIdentity, Finding, Tier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FamiliarBehaviorDifferenceEntry {
    pub feature_id: &'static str,
    pub owning_spec: &'static str,
    pub differences: &'static [&'static str],
}

pub const FAMILIAR_BEHAVIOR_DIFFERENCES: &[FamiliarBehaviorDifferenceEntry] = &[
    FamiliarBehaviorDifferenceEntry {
        feature_id: "ANI-001",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "AnimationClip::sample caps event firing across multi-cycle steps in a single frame to at most 64 loop/ping-pong cycles (max_cycles = 64), whereas Unity 2022.3 LTS does not specify a 64-cycle cap.",
            "PropertyCurve::Quaternion evaluation and pose blending use shortest-path component-aligned Hermite/Linear interpolation followed by unit normalization (nlerp_shortest) rather than Unity's internal slerp/squad curve resampling.",
            "StateMotion::BlendTree2D computes child blend weights via inverse-squared Euclidean distance in (parameter_x, parameter_y) space across SimpleDirectional, FreeformDirectional, and FreeformCartesian modes rather than polar-angle/gradient-band Delaunay triangulation.",
            "Within a single frame, animation events across different animators fire in deterministic ascending EntityId evaluation order rather than native job-thread completion order.",
            "Controller default property values for Write Defaults are captured from the controller's curves evaluated at t = 0.0 upon binding rather than snapshotting live scene values at OnEnable.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "ANI-002",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "Humanoid muscle space models 19 HumanBones (7 required core bones) and 17 primary normalized MuscleChannel axes with analytical twist extraction and linear hips_scale root retargeting rather than Unity's 55-muscle + finger-DoF solver.",
            "OnAnimatorIK limb solving uses a deterministic closed-form analytical two-bone law-of-cosines solver (solve_two_bone_ik) with pole-vector projection and clamped slerp look-at distribution rather than iterative cyclic coordinate descent (CCD).",
            "PlayableGraph invokes ScriptPlayable callbacks in deterministic ascending node-creation index order among reachable nodes (all PrepareFrame before any ProcessFrame) rather than depth-first topological traversal from outputs.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "CIN-001",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "TimelineWorld::drive_animation drives bound AnimationWorld entities by gating animator playback speed and tracking clip local time rather than constructing an internal AnimationLayerMixerPlayable sub-graph per track.",
            "CinemachineWorld resolves equal-priority active virtual cameras deterministically by a monotonically increasing activation counter and reports normalized linear blend progress elapsed / duration rather than custom AnimationCurve tangents.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "REN-004",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "Particle emission and shape sampling use a platform-independent SplitMix64 PRNG with Marsaglia sphere rejection and Newton-Raphson cube root so fixed seeds produce identical particle counts and positions across all hosts/targets, whereas numeric sample positions for a given seed differ from Unity 2022.3 LTS's internal generator.",
            "ParticleShape::Mesh samples uniformly inside the mesh's axis-aligned bounding box rather than barycentric triangle/vertex/edge surface weighting.",
            "Property-over-lifetime modules evaluate two-keyframe linear curves (Curve<N>) at normalized age age / lifetime.",
            "3D and 2D particle collision and trigger queries evaluate sphere/circle overlaps of radius particle.size * 0.5 with radial contact normals rather than sub-step raycast trajectory deflection.",
            "ParticleSystemForceField evaluates Sphere and Box containment with directional, gravity, radial gravity_focus, and linear drag forces without 3D vector-field textures or curl-noise tables.",
            "Visual Effect Graph (com.unity.visualeffectgraph) is excluded and reported as Unsupported under REN-004.",
            "A scene-loaded ParticleSystem (class 198) binds only m_Duration, m_Loop, m_MaxParticles, m_StartLifetime, m_StartSpeed, m_StartSize, m_StartColor, m_EmissionRate, and m_Bursts, so Unity 2022.3 LTS's shape, rotation, noise, sub-emitter, texture-sheet-animation, trails, lights, collision, simulation-space, scaling-mode, culling-mode, and prewarm settings are dropped at import and the bound system keeps the default point shape.",
            "Burst emission schedules every burst at m_Time + cycle * m_RepeatInterval and emits its fixed count once per cycle with at least one cycle on simulation-tick boundaries, whereas Unity 2022.3 LTS's Burst module also offers randomized count ranges, per-burst probability, the Single/Continuous/Random cycle modes, a zero cycle count that repeats indefinitely, and exact-time scheduling inside the frame.",
            "Scene-loaded particles are exposed as a per-tick draw list of entity, position, size, and color, whereas Unity 2022.3 LTS's ParticleSystemRenderer draws billboarded, stretched, or mesh particles with axis and velocity stretch scaling, texture-sheet animation, and material selection.",
            "A scene-loaded TrailRenderer (class 96) binds only m_Time, m_Width or the first and last m_WidthCurve keyframe values, m_MinVertexDistance, m_StartColor, and m_EndColor, so Unity 2022.3 LTS's m_ColorGradient, m_Material, m_TextureMode, m_Alignment, m_LightProbeUsage, and shadow-casting settings are dropped at import.",
            "The trail is simulated as a CPU point history whose points expire after m_Time, that appends a point only when it is at least m_MinVertexDistance from the previously kept point, and whose width and color fade linearly from the start to the end values over age / m_Time, whereas Unity 2022.3 LTS's TrailRenderer animates the width along the trail life with a per-vertex curve, builds corner and cap geometry from numCapVertices, and renders the ribbon with a material.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "PHY-004",
        owning_spec: "physics-navigation-terrain.md",
        differences: &[
            "WheelCollider uses unitas's deterministic sphere-cast suspension and piecewise-linear slip solver rather than PhysX 4.1's vehicle SDK solver; trajectories are repeatable per target but not bit-for-bit identical to Unity 2022.3 LTS.",
            "ArticulationBody steps hierarchical Revolute and Prismatic joint chains deterministically in reduced coordinates with PD target drives clamped to [lower, upper].",
            "Cloth simulates a deterministic Verlet particle grid with distance constraints, gravity, damping, ClearTransformMotion, and collision resolution exclusively against assigned ClothCollider capsules.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "NAV-001",
        owning_spec: "physics-navigation-terrain.md",
        differences: &[
            "Calling SetDestination or path-dependent members on an off-NavMesh NavMeshAgent (isOnNavMesh == false) raises an explicit error naming the agent and member (NavigationError::AgentOffNavMesh) rather than failing silently.",
            "Path queries and agents expose structured UnreachableReason (StartNotOnNavMesh, GoalNotOnNavMesh, NoRouteToGoal) alongside PathComplete, PathPartial, and PathInvalid.",
            "Saved navigation data that no longer matches its source geometry or settings is marked and reported is_stale() == true and is never silently rebuilt on entering play mode.",
            "Navigation mesh building, NavMeshObstacle polygon carving, portal A* pathfinding, and reciprocal priority-weighted avoidance use unitas's deterministic solver rather than Recast/Detour's internal voxel/crowd numeric output.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "ENV-001",
        owning_spec: "physics-navigation-terrain.md",
        differences: &[
            "SpeedTree 7 (.spm) and SpeedTree 8 (.st) geometry, materials, and imported mesh LODs are supported (PartiallySupported), whereas SpeedTree wind animation is excluded and reported on each affected prototype (wind_excluded == true) while Tree Creator trees and terrain grass wave with WindZone.",
            "World positions remain 32-bit single-precision without silent origin rebasing; objects at >= 100,000 units report a structured LargeWorldImpact identifying the affected systems (physics, rendering, animation, navigation).",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "UI-001",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "Screen-space overlay Canvas, CanvasScaler, and RectTransform anchor and pivot rectangles are resolved in unitas's deterministic layout pass and composited with a screen-space UI pass rather than Unity's native Canvas batch builder; screen-space camera and world-space canvases, layout groups, masks, ScrollRect, and UI Toolkit runtime documents are not implemented.",
            "Button and label focus navigation cycles deterministically in ascending entity hierarchy order on Tab/gamepad D-pad and dispatches Button.onClick callbacks across the bridge without full EventSystem raycaster module stacks.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "UI-002",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "MonoBehaviour.OnGUI immediate-mode runtime GUI dispatch, Event.current, GUI and GUILayout controls, GUISkin, and GUIStyle are not implemented and are reported as Unsupported.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "TXT-001",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "TrueType and OpenType fonts are rasterized into a deterministic shelf-packed GlyphAtlas with horizontal advance and kerning pair shaping rather than Unity's native dynamic FontTexture or TextMeshPro SDF atlas generator.",
            "TextMeshPro assets, SDF shaders, rich-text tag parsing, complex script and bidirectional UAX #9 layout, and IME composition windows are not implemented.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "INP-001",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "Legacy Input axes and key/mouse button transitions are stepped once per frame from deterministic queued InputEvents and InputManager.asset axis definitions without OS-level raw input thread polling; multi-touch Input.touches, Input.inputString, Cursor.lockState, and mobile sensors are not implemented.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "INP-002",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "Gamepad stick dead zones, button transitions, hot-plug events, and rumble commands are routed through the runtime's unified InputState, whereas Input System 1.x .inputactions asset compilation, InputAction phase state machines, PlayerInput, and interactive rebinding are not implemented.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "AUD-001",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "AudioWorld mixes decoded WAV, AIFF, Ogg Vorbis, and MP3 clips in deterministic 32-bit float stereo frames with linear/logarithmic/custom 3D distance attenuation, equal-power stereo panning, Doppler pitch ratio clamping, and Sfx/Music/Menu bus gains rather than FMOD Studio's internal DSP graph.",
            "AudioMixer asset snapshots, exposed parameters, built-in mixer DSP effects, filter components, AudioReverbZone, ambisonic decoding, spectrum analysis, and Microphone capture are not implemented.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "MED-001",
        owning_spec: "ui-audio-input.md",
        differences: &[
            "VideoClip import probes MP4/MOV/WebM/IVF container metadata into cooked cache artifacts and VideoPlayer decodes cooked clips to RGBA frames on a worker thread via dynamically loaded system FFmpeg libraries (libavformat, libavcodec, libavutil, libswscale), presenting frames onto the target entity's sprite surface; when host FFmpeg shared libraries are unavailable or on WebAssembly targets, VideoPlayer reports a platform-limited diagnostic.",
            "Audio-track routing from video containers into the AudioMixer, camera near/far plane and arbitrary material-property render modes, DSP/game clock drift resynchronization, and HTTP/HLS/DASH network streaming are not implemented.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "XR-001",
        owning_spec: "build-platforms-xr-networking.md",
        differences: &[
            "OpenXR and XR Plugin Management loader lifecycle, stereo HMD pose tracking and view projection, XR Interaction Toolkit, XR Hands, and XR haptics are not implemented and are reported as Unsupported.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "NET-001",
        owning_spec: "build-platforms-xr-networking.md",
        differences: &[
            "Unity Transport, Netcode for GameObjects, and Netcode for Entities session management, state synchronization, and RPCs are not implemented and are reported as Unsupported.",
        ],
    },
];

pub fn lookup_familiar_behavior_differences(
    feature_id: &str,
) -> Option<&'static FamiliarBehaviorDifferenceEntry> {
    FAMILIAR_BEHAVIOR_DIFFERENCES
        .iter()
        .find(|entry| entry.feature_id == feature_id)
}

pub fn build_semantic_finding(
    feature_id: &str,
    state: CompatibilityState,
    content: ContentIdentity,
    behavior: &str,
    cmp_001_benchmark_id: &str,
    found_difference: Option<&str>,
) -> Result<Finding, FindingSubmissionError> {
    let trimmed_id = feature_id.trim();
    if trimmed_id.is_empty() {
        return Err(FindingSubmissionError::EmptyFeatureId);
    }
    let Some(catalog_entry) = lookup_catalog_feature(trimmed_id) else {
        return Err(FindingSubmissionError::UnknownCatalogFeatureId {
            feature_id: trimmed_id.to_string(),
        });
    };
    if catalog_entry.tier != Tier::Semantic {
        return Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id: trimmed_id.to_string(),
            expected: catalog_entry.tier,
            submitted: Tier::Semantic,
        });
    }
    let benchmark = cmp_001_benchmark_id.trim();
    if benchmark.is_empty() {
        return Err(FindingSubmissionError::EmptyObservableConsequence);
    }
    let consequence = match found_difference {
        Some(diff) if !diff.trim().is_empty() => format!(
            "assessed against Unity 2022.3 LTS reference ({}) and backed by CMP-001 benchmark {}; difference: {}",
            catalog_entry.owning_spec,
            benchmark,
            diff.trim()
        ),
        _ => format!(
            "matches Unity 2022.3 LTS documented observable behavior ({}) as verified by CMP-001 benchmark {}",
            catalog_entry.owning_spec, benchmark
        ),
    };
    let mut input = FindingInput::new(
        trimmed_id,
        Tier::Semantic,
        state,
        content,
        behavior,
        consequence,
    )
    .with_condition(format!(
        "reference: Unity 2022.3 LTS ({})",
        catalog_entry.owning_spec
    ))
    .with_condition(format!("benchmark: {benchmark}"));
    if let Some(diff) = found_difference {
        if !diff.trim().is_empty() {
            input = input.with_unity_difference(diff.trim());
        }
    }
    validate_finding_input(input, &[])
}

pub fn build_familiar_finding(
    feature_id: &str,
    state: CompatibilityState,
    content: ContentIdentity,
    behavior: &str,
    additional_differences: &[&str],
) -> Result<Finding, FindingSubmissionError> {
    let trimmed_id = feature_id.trim();
    if trimmed_id.is_empty() {
        return Err(FindingSubmissionError::EmptyFeatureId);
    }
    let Some(catalog_entry) = lookup_catalog_feature(trimmed_id) else {
        return Err(FindingSubmissionError::UnknownCatalogFeatureId {
            feature_id: trimmed_id.to_string(),
        });
    };
    if catalog_entry.tier != Tier::Familiar {
        return Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id: trimmed_id.to_string(),
            expected: catalog_entry.tier,
            submitted: Tier::Familiar,
        });
    }
    let mut differences: Vec<String> = lookup_familiar_behavior_differences(trimmed_id)
        .map(|entry| {
            entry
                .differences
                .iter()
                .map(|diff| (*diff).to_string())
                .collect()
        })
        .unwrap_or_default();
    for extra in additional_differences {
        let trimmed = extra.trim();
        if !trimmed.is_empty() && !differences.iter().any(|existing| existing == trimmed) {
            differences.push(trimmed.to_string());
        }
    }
    let consequence = format!(
        "behavior defined by {} ({}) and listed in the AUT-006 API reference with {} Unity 2022.3 LTS difference(s)",
        catalog_entry.owning_spec,
        trimmed_id,
        differences.len()
    );
    let mut input = FindingInput::new(
        trimmed_id,
        Tier::Familiar,
        state,
        content,
        behavior,
        consequence,
    )
    .with_condition(format!(
        "owning specification: {} ({})",
        catalog_entry.owning_spec, trimmed_id
    ))
    .with_condition("listed in AUT-006 API reference".to_string());
    for diff in differences {
        input = input.with_unity_difference(diff);
    }
    validate_finding_input(input, &[])
}

pub fn assess_translated_material(
    content: ContentIdentity,
    material_description: &str,
    unitas_reference_image: Option<&str>,
    passes_unitas_reference: bool,
    differs_from_unity_screenshot: bool,
) -> Finding {
    let sanitized = sanitize_content_identity(&content);
    let (state, consequence) = match (unitas_reference_image, passes_unitas_reference) {
        (Some(ref_img), true) => (
            CompatibilityState::Supported,
            format!(
                "translated material passes visual validation against unitas-approved reference image '{ref_img}' (TST-002); pixel difference from a Unity screenshot (observed={differs_from_unity_screenshot}) is never a compatibility failure"
            ),
        ),
        (Some(ref_img), false) => (
            CompatibilityState::PartiallySupported,
            format!(
                "translated material diverges from unitas-approved reference image '{ref_img}' (TST-002)"
            ),
        ),
        (None, _) => (
            CompatibilityState::Unknown,
            "translated material has not yet been compared against a unitas-approved reference image (TST-002)".to_string(),
        ),
    };
    Finding {
        feature_id: "REN-002".to_string(),
        tier: Tier::Native,
        state,
        content: sanitized,
        behavior: material_description.trim().to_string(),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: consequence,
        conditions: vec![
            "visual acceptance uses unitas-approved references (TST-002), not Unity screenshots"
                .to_string(),
        ],
        related_findings: Vec::new(),
    }
}
