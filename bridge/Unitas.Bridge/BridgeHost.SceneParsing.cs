using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IDisposable, IPhysicsProvider, IPhysics2DProvider, ICharacterControllerProvider, IInputProvider, ISceneHost, IApplicationHost, IAudioProvider, IRuntimeObjectProvider
    {
        private void ParseLoadedSceneRigidbodies(string scenePath, LoadSceneMode mode)
        {
            if (mode == LoadSceneMode.Single)
            {
                _sceneObjectNames.Clear();
                _sceneObjectLayers.Clear();
                _sceneRigidbodies.Clear();
                _sceneSphereColliders.Clear();
                _sceneBoxColliders.Clear();
                _sceneCapsuleColliders.Clear();
                _sceneMeshColliders.Clear();
                _sceneRigidbodies2D.Clear();
                _sceneBoxColliders2D.Clear();
                _sceneCircleColliders2D.Clear();
                _sceneCapsuleColliders2D.Clear();
                _scenePolygonColliders2D.Clear();
                _sceneEdgeColliders2D.Clear();
                _sceneCompositeColliders2D.Clear();
                _sceneTilemapColliders2D.Clear();
                _scenePlatformEffectors2D.Clear();
                _sceneSurfaceEffectors2D.Clear();
                _registeredBodies2D.Clear();
                _sceneTransforms.Clear();
                _sceneCameras.Clear();
                _sceneAudioSources.Clear();
                _sceneUiComponents.Clear();
                _sceneScriptOwners.Clear();
                _sceneEntityKeyShift = 0;
                _alignSceneEntityBase = true;
                DropMaterializedSceneObjects();
                DropMaterializedSceneCameras();
                _nextSceneEntityBase = 1;
                foreach (KeyValuePair<ulong, MonoBehaviour> entry in _scripts)
                {
                    GameObject? owner = entry.Value == null ? null : entry.Value.gameObject;
                    if (owner != null && owner.IsPersistent && owner.NativeEntityId >= _nextSceneEntityBase)
                    {
                        _nextSceneEntityBase = owner.NativeEntityId + 1;
                    }
                }
            }

            if (string.IsNullOrEmpty(scenePath) || !File.Exists(scenePath))
            {
                return;
            }

            string[] lines = File.ReadAllLines(scenePath);
            SceneDocumentParseState state = new SceneDocumentParseState();
            ParseSceneDocumentLines(state, lines);
            FinalizeSceneDocument(state, scenePath);
        }

        private void ParseSceneDocumentLines(SceneDocumentParseState state, string[] lines)
        {
            for (int index = 0; index < lines.Length; index++)
            {
                string rawLine = lines[index];
                ParseSceneDocumentLine(state, rawLine.Trim(), rawLine);
            }
        }

        private void ParseSceneDocumentLine(SceneDocumentParseState state, string line, string rawLine)
        {
            if (line.StartsWith("--- !u!", StringComparison.Ordinal))
            {
                CollectSceneDocumentRecords(state);
                RecordScenePhysics2DComponent(
                    state.CurrentClassId,
                    state.CurrentGameObjectFileId,
                    state.Current2D,
                    state.Rigidbodies2D,
                    state.BoxColliders2D,
                    state.CircleColliders2D,
                    state.PlatformEffectors2D,
                    state.SurfaceEffectors2D);
                RecordExtendedScenePhysics2DComponent(state);
                FlushSceneDocument(state);
                BeginSceneDocument(state, line);
                return;
            }

            if (state.CurrentClassId == TILEMAP_CLASS)
            {
                state.CurrentTilemapRawLines.Add(rawLine);
            }

            ApplySceneDocumentProperty(state, line);
        }

        private void CollectSceneDocumentRecords(SceneDocumentParseState state)
        {
            if (state.CurrentClassId == 1 && !string.IsNullOrEmpty(state.CurrentFileId) && state.CurrentLayer != 0)
            {
                state.GameObjectLayers[state.CurrentFileId] = state.CurrentLayer;
            }
            if (state.CurrentClassId == 4 && state.CurrentFileId.Length > 0)
            {
                state.Transforms.Add(new ParsedTransformRecord(
                    state.CurrentFileId,
                    state.CurrentGameObjectFileId,
                    state.CurrentFatherFileId,
                    state.CurrentLocalPosition,
                    state.CurrentLocalRotation,
                    state.CurrentLocalScale));
            }
            if (state.CurrentClassId == AUDIO_SOURCE_CLASS && state.CurrentAudioGameObjectFileId.Length > 0)
            {
                state.AudioSources.Add(new ParsedAudioSourceRecord(
                    state.CurrentAudioGameObjectFileId,
                    state.CurrentAudioClipGuid,
                    state.CurrentAudioVolume,
                    state.CurrentAudioPitch,
                    state.CurrentAudioSpatialBlend,
                    state.CurrentAudioMinDistance,
                    state.CurrentAudioMaxDistance,
                    state.CurrentAudioPriority,
                    state.CurrentAudioLoop,
                    state.CurrentAudioPlayOnAwake,
                    state.CurrentAudioMute,
                    state.CurrentAudioEnabled));
            }
            if (state.CurrentClassId == MONO_BEHAVIOUR_CLASS
                && !string.IsNullOrEmpty(state.CurrentUiGuid)
                && TryParseScriptGuid(state.CurrentUiGuid, out ulong finalScriptGuidHi, out ulong finalScriptGuidLo))
            {
                RecordSceneScriptOwner(finalScriptGuidHi, finalScriptGuidLo, state.CurrentUiGameObjectFileId, state.GameObjectEntities);
                ScriptGlueRegistry.BuiltInUiComponent finalKind = ScriptGlueRegistry.GetBuiltInUiComponent(finalScriptGuidHi, finalScriptGuidLo);
                if (finalKind != ScriptGlueRegistry.BuiltInUiComponent.None)
                {
                    state.UiComponents.Add(new ParsedUiComponentRecord(
                        state.CurrentUiGameObjectFileId,
                        finalKind,
                        state.CurrentUiEnabled,
                        state.CurrentUiInteractable,
                        state.CurrentUiText,
                        state.CurrentUiFontSize,
                        state.CurrentUiColor,
                        state.CurrentUiSprite,
                        state.CurrentUiScaleMode,
                        state.CurrentUiReferenceResolution,
                        state.CurrentUiScreenMatchMode,
                        state.CurrentUiMatchWidthOrHeight));
                }
            }
        }

        private void FlushSceneDocument(SceneDocumentParseState state)
        {
            FlushDocument(
                state.CurrentClassId,
                state.CurrentFileId,
                state.CurrentGameObjectFileId,
                state.CurrentName,
                state.CurrentMass,
                state.CurrentUseGravity,
                state.CurrentIsKinematic,
                state.CurrentRadius,
                state.CurrentHeight,
                state.CurrentDirection,
                state.CurrentIsTrigger,
                state.CurrentCenter,
                state.CurrentSize,
                state.CurrentConvex,
                state.CurrentMeshFileId,
                state.CurrentMeshGuid,
                new SceneCameraSettings(state.CurrentCameraBackground, state.CurrentCameraFieldOfView, state.CurrentCameraNearClipPlane, state.CurrentCameraFarClipPlane, state.CurrentCameraOrthographic, state.CurrentCameraOrthographicSize),
                ref state.LocalEntityIndex,
                state.GameObjectEntities,
                state.GameObjectNames,
                state.Rigidbodies,
                state.SphereColliders,
                state.BoxColliders,
                state.CapsuleColliders,
                state.MeshColliders,
                state.Cameras);
        }

        private void BeginSceneDocument(SceneDocumentParseState state, string headerLine)
        {
            ParseDocumentHeader(headerLine, out state.CurrentClassId, out state.CurrentFileId);
            state.CurrentGameObjectFileId = string.Empty;
            state.CurrentName = string.Empty;
            state.CurrentLayer = 0;
            state.CurrentMass = 1f;
            state.CurrentUseGravity = true;
            state.CurrentIsKinematic = false;
            state.CurrentRadius = 0.5f;
            state.CurrentHeight = 2f;
            state.CurrentDirection = 1;
            state.CurrentIsTrigger = false;
            state.CurrentCenter = Vector3.zero;
            state.CurrentSize = new Vector3(1f, 1f, 1f);
            state.CurrentConvex = false;
            state.Current2D = new ScenePhysics2DFields();
            state.CurrentTilemapRawLines.Clear();
            if (state.CurrentClassId == TILEMAP_CLASS)
            {
                state.CurrentTilemapRawLines.Add("%YAML 1.1");
                state.CurrentTilemapRawLines.Add("%TAG !u! tag:unity3d.com,2011:");
                state.CurrentTilemapRawLines.Add(headerLine);
            }
            state.CurrentMeshFileId = string.Empty;
            state.CurrentMeshGuid = string.Empty;
            state.CurrentFatherFileId = string.Empty;
            state.CurrentLocalPosition = Vector3.zero;
            state.CurrentLocalRotation = Quaternion.identity;
            state.CurrentLocalScale = Vector3.one;
            state.CurrentCameraBackground = Color.black;
            state.CurrentCameraFieldOfView = 60f;
            state.CurrentCameraNearClipPlane = 0.3f;
            state.CurrentCameraFarClipPlane = 1000f;
            state.CurrentCameraOrthographic = false;
            state.CurrentCameraOrthographicSize = 5f;
            state.CurrentAudioGameObjectFileId = string.Empty;
            state.CurrentAudioClipGuid = string.Empty;
            state.CurrentAudioEnabled = true;
            state.CurrentAudioVolume = 1f;
            state.CurrentAudioPitch = 1f;
            state.CurrentAudioLoop = false;
            state.CurrentAudioPlayOnAwake = true;
            state.CurrentAudioMute = false;
            state.CurrentAudioPriority = 128;
            state.CurrentAudioSpatialBlend = 0f;
            state.CurrentAudioMinDistance = 1f;
            state.CurrentAudioMaxDistance = 500f;
            state.CurrentUiGuid = string.Empty;
            state.CurrentUiGameObjectFileId = string.Empty;
            state.CurrentUiEnabled = true;
            state.CurrentUiInteractable = true;
            state.CurrentUiText = string.Empty;
            state.CurrentUiFontSize = 14f;
            state.CurrentUiColor = Color.white;
            state.CurrentUiSprite = string.Empty;
            state.CurrentUiScaleMode = 1;
            state.CurrentUiReferenceResolution = new Vector2(800f, 600f);
            state.CurrentUiScreenMatchMode = 0;
            state.CurrentUiMatchWidthOrHeight = 0.5f;
            state.CurrentPrefabInstance = null;
            state.CurrentModificationTarget = string.Empty;
            state.CurrentModificationProperty = null;
            if (state.CurrentClassId == PREFAB_INSTANCE_CLASS)
            {
                state.CurrentPrefabInstance = new PrefabInstanceInfo { FileId = state.CurrentFileId };
                state.PrefabInstances.Add(state.CurrentPrefabInstance);
            }
        }

        private void ApplySceneDocumentProperty(SceneDocumentParseState state, string line)
        {
            switch (state.CurrentClassId)
            {
                case 1:
                    ApplySceneGameObjectProperty(state, line);
                    break;
                case 4:
                    ApplySceneTransformProperty(state, line);
                    break;
                case 54:
                    ApplySceneRigidbodyProperty(state, line);
                    break;
                case 65:
                case 135:
                case 136:
                    ApplySceneColliderProperty(state, line);
                    break;
                case MESH_COLLIDER_CLASS:
                    ApplySceneMeshColliderProperty(state, line);
                    break;
                case RIGIDBODY2D_CLASS:
                    ApplySceneRigidbody2DProperty(state, line);
                    break;
                case BOX_COLLIDER2D_CLASS:
                    ApplySceneBoxCollider2DProperty(state, line);
                    break;
                case CIRCLE_COLLIDER2D_CLASS:
                    ApplySceneCircleCollider2DProperty(state, line);
                    break;
                case CAPSULE_COLLIDER2D_CLASS:
                    ApplySceneCapsuleCollider2DProperty(state, line);
                    break;
                case POLYGON_COLLIDER2D_CLASS:
                    ApplyScenePolygonCollider2DProperty(state, line);
                    break;
                case EDGE_COLLIDER2D_CLASS:
                    ApplySceneEdgeCollider2DProperty(state, line);
                    break;
                case COMPOSITE_COLLIDER2D_CLASS:
                    ApplySceneCompositeCollider2DProperty(state, line);
                    break;
                case TILEMAP_COLLIDER2D_CLASS:
                    ApplySceneTilemapCollider2DProperty(state, line);
                    break;
                case TILEMAP_CLASS:
                    ApplySceneTilemapProperty(state, line);
                    break;
                case PLATFORM_EFFECTOR2D_CLASS:
                    ApplyScenePlatformEffector2DProperty(state, line);
                    break;
                case SURFACE_EFFECTOR2D_CLASS:
                    ApplySceneSurfaceEffector2DProperty(state, line);
                    break;
                case 20:
                    ApplySceneCameraProperty(state, line);
                    break;
                case AUDIO_SOURCE_CLASS:
                    ApplySceneAudioSourceProperty(state, line);
                    break;
                case MONO_BEHAVIOUR_CLASS:
                    ApplySceneMonoBehaviourProperty(state, line);
                    break;
                case PREFAB_INSTANCE_CLASS:
                    ApplyScenePrefabInstanceProperty(state, line);
                    break;
            }
        }

        private void ApplySceneGameObjectProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_Name:", StringComparison.Ordinal))
            {
                state.CurrentName = ParseStringValue(line);
            }
            else if (line.StartsWith("m_Layer:", StringComparison.Ordinal))
            {
                state.CurrentLayer = (int)ParseFloatValue(line, 0f);
            }
        }

        private void ApplySceneTransformProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_Father:", StringComparison.Ordinal))
            {
                state.CurrentFatherFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_LocalPosition:", StringComparison.Ordinal))
            {
                state.CurrentLocalPosition = ParseVector3Value(line, Vector3.zero);
            }
            else if (line.StartsWith("m_LocalRotation:", StringComparison.Ordinal))
            {
                state.CurrentLocalRotation = ParseQuaternionValue(line, Quaternion.identity);
            }
            else if (line.StartsWith("m_LocalScale:", StringComparison.Ordinal))
            {
                state.CurrentLocalScale = ParseVector3Value(line, Vector3.one);
            }
        }

        private void ApplySceneRigidbodyProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_Mass:", StringComparison.Ordinal))
            {
                state.CurrentMass = ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_UseGravity:", StringComparison.Ordinal))
            {
                state.CurrentUseGravity = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_IsKinematic:", StringComparison.Ordinal))
            {
                state.CurrentIsKinematic = ParseBoolValue(line, false);
            }
        }

        private void ApplySceneColliderProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_IsTrigger:", StringComparison.Ordinal))
            {
                state.CurrentIsTrigger = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_Radius:", StringComparison.Ordinal))
            {
                state.CurrentRadius = ParseFloatValue(line, 0.5f);
            }
            else if (line.StartsWith("m_Height:", StringComparison.Ordinal))
            {
                state.CurrentHeight = ParseFloatValue(line, 2f);
            }
            else if (line.StartsWith("m_Direction:", StringComparison.Ordinal))
            {
                state.CurrentDirection = (int)ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_Center:", StringComparison.Ordinal))
            {
                state.CurrentCenter = ParseVector3Value(line, Vector3.zero);
            }
            else if (line.StartsWith("m_Size:", StringComparison.Ordinal))
            {
                state.CurrentSize = ParseVector3Value(line, new Vector3(1f, 1f, 1f));
            }
        }

        private void ApplySceneMeshColliderProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_IsTrigger:", StringComparison.Ordinal))
            {
                state.CurrentIsTrigger = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_Center:", StringComparison.Ordinal))
            {
                state.CurrentCenter = ParseVector3Value(line, Vector3.zero);
            }
            else if (line.StartsWith("m_Convex:", StringComparison.Ordinal))
            {
                state.CurrentConvex = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_Mesh:", StringComparison.Ordinal))
            {
                state.CurrentMeshFileId = ExtractFileId(line);
                state.CurrentMeshGuid = ExtractGuidValue(line);
            }
        }

        private void ApplySceneRigidbody2DProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_BodyType:", StringComparison.Ordinal))
            {
                state.Current2D.BodyType = (int)ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_Simulated:", StringComparison.Ordinal))
            {
                state.Current2D.Simulated = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_Mass:", StringComparison.Ordinal))
            {
                state.Current2D.Mass = ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_GravityScale:", StringComparison.Ordinal))
            {
                state.Current2D.GravityScale = ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_Interpolate:", StringComparison.Ordinal))
            {
                state.Current2D.Interpolation = (int)ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_CollisionDetection:", StringComparison.Ordinal))
            {
                state.Current2D.CollisionDetectionMode = (int)ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_Constraints:", StringComparison.Ordinal))
            {
                state.Current2D.Constraints = (int)ParseFloatValue(line, 0f);
            }
        }

        private void ApplySceneCollider2DCommonProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_IsTrigger:", StringComparison.Ordinal))
            {
                state.Current2D.IsTrigger = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_UsedByEffector:", StringComparison.Ordinal))
            {
                state.Current2D.UsedByEffector = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_UsedByComposite:", StringComparison.Ordinal))
            {
                state.Current2D.UsedByComposite = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_Offset:", StringComparison.Ordinal))
            {
                state.Current2D.Offset = ParseVector2Value(line, Vector2.zero);
            }
            else if (line.StartsWith("m_Material:", StringComparison.Ordinal))
            {
                state.Current2D.MaterialFileId = ExtractFileId(line);
                state.Current2D.MaterialGuid = ExtractGuidValue(line);
            }
            else if (line.StartsWith("m_Friction:", StringComparison.Ordinal) || line.StartsWith("friction:", StringComparison.Ordinal))
            {
                state.Current2D.Friction = ParseFloatValue(line, 0.4f);
                state.Current2D.HasInlineMaterial = true;
            }
            else if (line.StartsWith("m_Bounciness:", StringComparison.Ordinal) || line.StartsWith("bounciness:", StringComparison.Ordinal))
            {
                state.Current2D.Bounciness = ParseFloatValue(line, 0f);
                state.Current2D.HasInlineMaterial = true;
            }
        }

        private void ApplySceneBoxCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("m_Size:", StringComparison.Ordinal))
            {
                state.Current2D.Size = ParseVector2Value(line, Vector2.one);
            }
        }

        private void ApplySceneCircleCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("m_Radius:", StringComparison.Ordinal))
            {
                state.Current2D.Radius = ParseFloatValue(line, 0.5f);
            }
        }

        private void ApplySceneCapsuleCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("m_Size:", StringComparison.Ordinal))
            {
                state.Current2D.CapsuleSize = ParseVector2Value(line, new Vector2(0.5f, 1f));
            }
            else if (line.StartsWith("m_Direction:", StringComparison.Ordinal))
            {
                state.Current2D.CapsuleDirection = (int)ParseFloatValue(line, 1f);
            }
        }

        private void ApplyScenePolygonCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("- {x:", StringComparison.Ordinal) || line.StartsWith("- - {x:", StringComparison.Ordinal))
            {
                state.Current2D.Points.Add(ParseVector2Value(line, Vector2.zero));
            }
        }

        private void ApplySceneEdgeCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("- {x:", StringComparison.Ordinal) || line.StartsWith("- - {x:", StringComparison.Ordinal))
            {
                state.Current2D.Points.Add(ParseVector2Value(line, Vector2.zero));
            }
            else if (line.StartsWith("m_EdgeRadius:", StringComparison.Ordinal))
            {
                state.Current2D.EdgeRadius = ParseFloatValue(line, 0f);
            }
        }

        private void ApplySceneCompositeCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("m_GeometryType:", StringComparison.Ordinal))
            {
                state.Current2D.GeometryType = (int)ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_GenerationType:", StringComparison.Ordinal))
            {
                state.Current2D.GenerationType = (int)ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_VertexDistance:", StringComparison.Ordinal))
            {
                state.Current2D.VertexDistance = ParseFloatValue(line, 0.00005f);
            }
            else if (line.StartsWith("m_OffsetDistance:", StringComparison.Ordinal))
            {
                state.Current2D.OffsetDistance = ParseFloatValue(line, 0.000005f);
            }
            else if (line.StartsWith("m_EdgeRadius:", StringComparison.Ordinal))
            {
                state.Current2D.EdgeRadius = ParseFloatValue(line, 0f);
            }
        }

        private void ApplySceneTilemapCollider2DProperty(SceneDocumentParseState state, string line)
        {
            ApplySceneCollider2DCommonProperty(state, line);
            if (line.StartsWith("m_MaximumTileChangeCount:", StringComparison.Ordinal))
            {
                state.Current2D.MaximumTileChangeCount = (uint)Math.Max(0f, ParseFloatValue(line, 1000f));
            }
            else if (line.StartsWith("m_ExtrusionFactor:", StringComparison.Ordinal))
            {
                state.Current2D.ExtrusionFactor = ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_TilemapAsset:", StringComparison.Ordinal))
            {
                string guid = ExtractGuidValue(line);
                if (!string.IsNullOrEmpty(guid))
                {
                    state.Current2D.TilemapAssetGuid = guid;
                }
                else
                {
                    string val = ParseStringValue(line);
                    if (!val.StartsWith('{'))
                    {
                        state.Current2D.TilemapAssetPath = val;
                    }
                }
            }
            else if (line.StartsWith("m_TilemapPath:", StringComparison.Ordinal))
            {
                state.Current2D.TilemapAssetPath = ParseStringValue(line);
            }
        }

        private void ApplySceneTilemapProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_TilemapAsset:", StringComparison.Ordinal))
            {
                string guid = ExtractGuidValue(line);
                if (!string.IsNullOrEmpty(guid))
                {
                    state.Current2D.TilemapAssetGuid = guid;
                }
                else
                {
                    string val = ParseStringValue(line);
                    if (!val.StartsWith('{'))
                    {
                        state.Current2D.TilemapAssetPath = val;
                    }
                }
            }
            else if (line.StartsWith("m_TilemapPath:", StringComparison.Ordinal))
            {
                state.Current2D.TilemapAssetPath = ParseStringValue(line);
            }
        }

        private void ApplyScenePlatformEffector2DProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_UseOneWay:", StringComparison.Ordinal))
            {
                state.Current2D.UseOneWay = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_SurfaceArc:", StringComparison.Ordinal))
            {
                state.Current2D.SurfaceArc = ParseFloatValue(line, 180f);
            }
            else if (line.StartsWith("m_RotationalOffset:", StringComparison.Ordinal))
            {
                state.Current2D.RotationalOffset = ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_SideArc:", StringComparison.Ordinal))
            {
                state.Current2D.SideArc = ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_UseSideFriction:", StringComparison.Ordinal))
            {
                state.Current2D.UseSideFriction = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_UseSideBounce:", StringComparison.Ordinal))
            {
                state.Current2D.UseSideBounce = ParseBoolValue(line, false);
            }
        }

        private void ApplySceneSurfaceEffector2DProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_Speed:", StringComparison.Ordinal))
            {
                state.Current2D.Speed = ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_ForceScale:", StringComparison.Ordinal))
            {
                state.Current2D.ForceScale = ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_UseFriction:", StringComparison.Ordinal))
            {
                state.Current2D.UseFriction = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_UseBounce:", StringComparison.Ordinal))
            {
                state.Current2D.UseBounce = ParseBoolValue(line, false);
            }
        }

        private void ApplySceneCameraProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_BackGroundColor:", StringComparison.Ordinal))
            {
                state.CurrentCameraBackground = ParseColorValue(line, state.CurrentCameraBackground);
            }
            else if (line.StartsWith("near clip plane:", StringComparison.Ordinal))
            {
                state.CurrentCameraNearClipPlane = ParseFloatValue(line, state.CurrentCameraNearClipPlane);
            }
            else if (line.StartsWith("far clip plane:", StringComparison.Ordinal))
            {
                state.CurrentCameraFarClipPlane = ParseFloatValue(line, state.CurrentCameraFarClipPlane);
            }
            else if (line.StartsWith("field of view:", StringComparison.Ordinal))
            {
                state.CurrentCameraFieldOfView = ParseFloatValue(line, state.CurrentCameraFieldOfView);
            }
            else if (line.StartsWith("orthographic:", StringComparison.Ordinal))
            {
                state.CurrentCameraOrthographic = ParseBoolValue(line, state.CurrentCameraOrthographic);
            }
            else if (line.StartsWith("orthographic size:", StringComparison.Ordinal))
            {
                state.CurrentCameraOrthographicSize = ParseFloatValue(line, state.CurrentCameraOrthographicSize);
            }
        }

        private void ApplySceneAudioSourceProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentAudioGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_audioClip:", StringComparison.Ordinal) || line.StartsWith("m_AudioClip:", StringComparison.Ordinal))
            {
                state.CurrentAudioClipGuid = ExtractGuidValue(line);
            }
            else if (line.StartsWith("m_Enabled:", StringComparison.Ordinal))
            {
                state.CurrentAudioEnabled = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_Volume:", StringComparison.Ordinal))
            {
                state.CurrentAudioVolume = Math.Clamp(ParseFloatValue(line, 1f), 0f, 1f);
            }
            else if (line.StartsWith("m_Pitch:", StringComparison.Ordinal))
            {
                state.CurrentAudioPitch = ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("Loop:", StringComparison.Ordinal) || line.StartsWith("m_Loop:", StringComparison.Ordinal))
            {
                state.CurrentAudioLoop = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_PlayOnAwake:", StringComparison.Ordinal))
            {
                state.CurrentAudioPlayOnAwake = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("Mute:", StringComparison.Ordinal) || line.StartsWith("m_Mute:", StringComparison.Ordinal))
            {
                state.CurrentAudioMute = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("Priority:", StringComparison.Ordinal) || line.StartsWith("m_Priority:", StringComparison.Ordinal))
            {
                state.CurrentAudioPriority = Math.Clamp((int)ParseFloatValue(line, 128f), 0, 255);
            }
            else if (line.StartsWith("m_SpatialBlend:", StringComparison.Ordinal))
            {
                state.CurrentAudioSpatialBlend = Math.Clamp(ParseFloatValue(line, 0f), 0f, 1f);
            }
            else if (line.StartsWith("MinDistance:", StringComparison.Ordinal) || line.StartsWith("m_MinDistance:", StringComparison.Ordinal))
            {
                state.CurrentAudioMinDistance = Math.Max(0f, ParseFloatValue(line, 1f));
            }
            else if (line.StartsWith("MaxDistance:", StringComparison.Ordinal) || line.StartsWith("m_MaxDistance:", StringComparison.Ordinal))
            {
                state.CurrentAudioMaxDistance = Math.Max(0f, ParseFloatValue(line, 500f));
            }
        }

        private void ApplySceneMonoBehaviourProperty(SceneDocumentParseState state, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                state.CurrentUiGameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_Script:", StringComparison.Ordinal))
            {
                state.CurrentUiGuid = ExtractGuidValue(line);
            }
            else if (line.StartsWith("m_Enabled:", StringComparison.Ordinal))
            {
                state.CurrentUiEnabled = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_Interactable:", StringComparison.Ordinal))
            {
                state.CurrentUiInteractable = ParseBoolValue(line, true);
            }
            else if (line.StartsWith("m_Text:", StringComparison.Ordinal))
            {
                state.CurrentUiText = ParseStringValue(line);
            }
            else if (line.StartsWith("m_FontSize:", StringComparison.Ordinal))
            {
                state.CurrentUiFontSize = ParseFloatValue(line, 14f);
            }
            else if (line.StartsWith("m_Color:", StringComparison.Ordinal))
            {
                state.CurrentUiColor = ParseColorValue(line, state.CurrentUiColor);
            }
            else if (line.StartsWith("m_Sprite:", StringComparison.Ordinal))
            {
                state.CurrentUiSprite = ExtractGuidValue(line);
            }
            else if (line.StartsWith("m_UiScaleMode:", StringComparison.Ordinal))
            {
                state.CurrentUiScaleMode = (int)ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_ReferenceResolution:", StringComparison.Ordinal))
            {
                Vector3 resolution = ParseVector3Value(line, Vector3.zero);
                state.CurrentUiReferenceResolution = new Vector2(resolution.x, resolution.y);
            }
            else if (line.StartsWith("m_ScreenMatchMode:", StringComparison.Ordinal))
            {
                state.CurrentUiScreenMatchMode = (int)ParseFloatValue(line, 0f);
            }
            else if (line.StartsWith("m_MatchWidthOrHeight:", StringComparison.Ordinal))
            {
                state.CurrentUiMatchWidthOrHeight = ParseFloatValue(line, 0.5f);
            }
        }

        private void ApplyScenePrefabInstanceProperty(SceneDocumentParseState state, string line)
        {
            PrefabInstanceInfo instance = state.CurrentPrefabInstance ?? new PrefabInstanceInfo();
            if (line.StartsWith("m_SourcePrefab:", StringComparison.Ordinal))
            {
                instance.SourceGuid = ExtractGuidValue(line);
            }
            else if (line.StartsWith("m_TransformParent:", StringComparison.Ordinal))
            {
                instance.TransformParentFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("- target:", StringComparison.Ordinal))
            {
                state.CurrentModificationTarget = ExtractFileId(line);
                state.CurrentModificationProperty = null;
            }
            else if (line.StartsWith("propertyPath:", StringComparison.Ordinal))
            {
                state.CurrentModificationProperty = ParseStringValue(line);
            }
            else if (line.StartsWith("value:", StringComparison.Ordinal) && state.CurrentModificationProperty != null)
            {
                instance.Modifications.Add(new PrefabModification(
                    state.CurrentModificationTarget,
                    state.CurrentModificationProperty,
                    ParseStringValue(line)));
                state.CurrentModificationProperty = null;
            }
            else if (line.StartsWith("addedObject:", StringComparison.Ordinal))
            {
                string addedFileId = ExtractFileId(line);
                if (addedFileId.Length > 0)
                {
                    instance.AddedGameObjectFileIds.Add(addedFileId);
                }
            }
        }

        private static void RecordExtendedScenePhysics2DComponent(SceneDocumentParseState state)
        {
            string id = state.CurrentGameObjectFileId;
            if (string.IsNullOrEmpty(id))
            {
                return;
            }
            ScenePhysics2DFields f = state.Current2D;
            switch (state.CurrentClassId)
            {
                case CAPSULE_COLLIDER2D_CLASS:
                    state.CapsuleColliders2D.Add(new ParsedCapsuleCollider2DRecord(id, f.CapsuleSize, f.CapsuleDirection, f.Offset, f.IsTrigger, f.UsedByEffector, f.UsedByComposite, f.MaterialGuid, f.HasInlineMaterial, f.Friction, f.Bounciness));
                    break;
                case POLYGON_COLLIDER2D_CLASS:
                    state.PolygonColliders2D.Add(new ParsedPolygonCollider2DRecord(id, f.Points.ToArray(), f.Offset, f.IsTrigger, f.UsedByEffector, f.UsedByComposite, f.MaterialGuid, f.HasInlineMaterial, f.Friction, f.Bounciness));
                    break;
                case EDGE_COLLIDER2D_CLASS:
                    state.EdgeColliders2D.Add(new ParsedEdgeCollider2DRecord(id, f.Points.ToArray(), f.EdgeRadius, f.Offset, f.IsTrigger, f.UsedByEffector, f.UsedByComposite, f.MaterialGuid, f.HasInlineMaterial, f.Friction, f.Bounciness));
                    break;
                case COMPOSITE_COLLIDER2D_CLASS:
                    state.CompositeColliders2D.Add(new ParsedCompositeCollider2DRecord(id, f.GeometryType, f.GenerationType, f.VertexDistance, f.OffsetDistance, f.EdgeRadius, f.Offset, f.IsTrigger, f.UsedByEffector, f.MaterialGuid, f.HasInlineMaterial, f.Friction, f.Bounciness));
                    break;
                case TILEMAP_COLLIDER2D_CLASS:
                    state.TilemapColliders2D.Add(new ParsedTilemapCollider2DRecord(id, f.MaximumTileChangeCount, f.ExtrusionFactor, f.TilemapAssetPath, f.TilemapAssetGuid, f.Offset, f.IsTrigger, f.UsedByEffector, f.UsedByComposite, f.MaterialGuid, f.HasInlineMaterial, f.Friction, f.Bounciness));
                    break;
                case TILEMAP_CLASS:
                    if (!string.IsNullOrEmpty(f.TilemapAssetGuid))
                    {
                        state.TilemapAssetGuids[id] = f.TilemapAssetGuid;
                    }
                    if (!string.IsNullOrEmpty(f.TilemapAssetPath))
                    {
                        state.TilemapAssetPaths[id] = f.TilemapAssetPath;
                    }
                    if (state.CurrentTilemapRawLines.Count > 3)
                    {
                        state.TilemapInlineYamls[id] = string.Join("\n", state.CurrentTilemapRawLines);
                    }
                    break;
            }
        }

        private void FinalizeSceneDocument(SceneDocumentParseState state, string scenePath)
        {
            CollectSceneDocumentRecords(state);
            FlushSceneDocument(state);
            RecordScenePhysics2DComponent(
                state.CurrentClassId,
                state.CurrentGameObjectFileId,
                state.Current2D,
                state.Rigidbodies2D,
                state.BoxColliders2D,
                state.CircleColliders2D,
                state.PlatformEffectors2D,
                state.SurfaceEffectors2D);
            RecordExtendedScenePhysics2DComponent(state);
            ApplySceneObjectNames(state);
            ApplySceneRigidbodyRecords(state);
            ApplySceneColliderRecords(state);
            ApplyScenePhysics2DRecords(state, scenePath);
            ApplySceneTransformRecords(state);
            ApplySceneUiRecords(state);
            ApplySceneParentEntities(state, scenePath);
            ApplySceneCameraRecords(state);
            ApplySceneAudioSourceRecords(state, scenePath);
            _nextSceneEntityBase += state.LocalEntityIndex;
        }

        private void ApplySceneObjectNames(SceneDocumentParseState state)
        {
            foreach (KeyValuePair<string, ulong> entry in state.GameObjectEntities)
            {
                ulong shiftedEntity = _nextSceneEntityBase + entry.Value;
                if (state.GameObjectNames.TryGetValue(entry.Key, out string? name) && !string.IsNullOrEmpty(name))
                {
                    _sceneObjectNames[shiftedEntity] = name;
                }
                if (state.GameObjectLayers.TryGetValue(entry.Key, out int layer) && layer != 0)
                {
                    _sceneObjectLayers[shiftedEntity] = layer;
                }
            }
        }

        private void ApplySceneRigidbodyRecords(SceneDocumentParseState state)
        {
            for (int index = 0; index < state.Rigidbodies.Count; index++)
            {
                ParsedRigidbodyRecord record = state.Rigidbodies[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneRigidbodies[shiftedEntity] = new SceneRigidbodyMetadata(
                        record.Mass,
                        record.UseGravity,
                        record.IsKinematic);
                }
            }
        }

        private void ApplySceneColliderRecords(SceneDocumentParseState state)
        {
            for (int index = 0; index < state.SphereColliders.Count; index++)
            {
                ParsedSphereColliderRecord record = state.SphereColliders[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneSphereColliders[shiftedEntity] = new SceneSphereColliderMetadata(
                        record.Radius,
                        record.Center,
                        record.IsTrigger);
                }
            }

            for (int index = 0; index < state.BoxColliders.Count; index++)
            {
                ParsedBoxColliderRecord record = state.BoxColliders[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneBoxColliders[shiftedEntity] = new SceneBoxColliderMetadata(
                        record.Size,
                        record.Center,
                        record.IsTrigger);
                }
            }

            for (int index = 0; index < state.CapsuleColliders.Count; index++)
            {
                ParsedCapsuleColliderRecord record = state.CapsuleColliders[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneCapsuleColliders[shiftedEntity] = new SceneCapsuleColliderMetadata(
                        record.Radius,
                        record.Height,
                        record.Direction,
                        record.Center,
                        record.IsTrigger);
                }
            }

            for (int index = 0; index < state.MeshColliders.Count; index++)
            {
                ParsedMeshColliderRecord record = state.MeshColliders[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneMeshColliders[shiftedEntity] = new SceneMeshColliderMetadata(
                        record.MeshFileId,
                        record.MeshGuid,
                        record.Convex,
                        record.IsTrigger,
                        record.Center);
                }
            }
        }

        private void ApplyScenePhysics2DRecords(SceneDocumentParseState state, string scenePath)
        {
            PrefabGuidIndex? guidIndex = null;
            for (int index = 0; index < state.Rigidbodies2D.Count; index++)
            {
                ParsedRigidbody2DRecord record = state.Rigidbodies2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneRigidbodies2D[shiftedEntity] = new SceneRigidbody2DMetadata(
                        ToRigidbodyType2D(record.BodyType),
                        record.Simulated,
                        record.Mass,
                        record.GravityScale,
                        ToRigidbodyInterpolation2D(record.Interpolation),
                        ToCollisionDetectionMode2D(record.CollisionDetectionMode),
                        (RigidbodyConstraints2D)record.Constraints);
                }
            }

            for (int index = 0; index < state.BoxColliders2D.Count; index++)
            {
                ParsedBoxCollider2DRecord record = state.BoxColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    _sceneBoxColliders2D[_nextSceneEntityBase + localEntity] = new SceneBoxCollider2DMetadata(record.Size, record.Offset, record.IsTrigger, record.UsedByEffector, record.UsedByComposite, material);
                }
            }

            for (int index = 0; index < state.CircleColliders2D.Count; index++)
            {
                ParsedCircleCollider2DRecord record = state.CircleColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    _sceneCircleColliders2D[_nextSceneEntityBase + localEntity] = new SceneCircleCollider2DMetadata(record.Radius, record.Offset, record.IsTrigger, record.UsedByEffector, record.UsedByComposite, material);
                }
            }

            for (int index = 0; index < state.CapsuleColliders2D.Count; index++)
            {
                ParsedCapsuleCollider2DRecord record = state.CapsuleColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    _sceneCapsuleColliders2D[_nextSceneEntityBase + localEntity] = new SceneCapsuleCollider2DMetadata(record.Size, record.Direction, record.Offset, record.IsTrigger, record.UsedByEffector, record.UsedByComposite, material);
                }
            }

            for (int index = 0; index < state.PolygonColliders2D.Count; index++)
            {
                ParsedPolygonCollider2DRecord record = state.PolygonColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    _scenePolygonColliders2D[_nextSceneEntityBase + localEntity] = new ScenePolygonCollider2DMetadata(record.Points, record.Offset, record.IsTrigger, record.UsedByEffector, record.UsedByComposite, material);
                }
            }

            for (int index = 0; index < state.EdgeColliders2D.Count; index++)
            {
                ParsedEdgeCollider2DRecord record = state.EdgeColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    _sceneEdgeColliders2D[_nextSceneEntityBase + localEntity] = new SceneEdgeCollider2DMetadata(record.Points, record.EdgeRadius, record.Offset, record.IsTrigger, record.UsedByEffector, record.UsedByComposite, material);
                }
            }

            for (int index = 0; index < state.CompositeColliders2D.Count; index++)
            {
                ParsedCompositeCollider2DRecord record = state.CompositeColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    _sceneCompositeColliders2D[_nextSceneEntityBase + localEntity] = new SceneCompositeCollider2DMetadata(record.GeometryType, record.GenerationType, record.VertexDistance, record.OffsetDistance, record.EdgeRadius, record.Offset, record.IsTrigger, record.UsedByEffector, material);
                }
            }

            for (int index = 0; index < state.TilemapColliders2D.Count; index++)
            {
                ParsedTilemapCollider2DRecord record = state.TilemapColliders2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    PhysicsMaterial2D? material = ResolveScenePhysicsMaterial2D(record.MaterialGuid, record.HasInlineMaterial, record.Friction, record.Bounciness, scenePath, ref guidIndex);
                    string resolvedTilemapPath = ResolveSceneTilemapPath(state, record, scenePath, ref guidIndex);
                    _sceneTilemapColliders2D[_nextSceneEntityBase + localEntity] = new SceneTilemapCollider2DMetadata(record.MaximumTileChangeCount, record.ExtrusionFactor, resolvedTilemapPath, record.Offset, record.IsTrigger, record.UsedByEffector, record.UsedByComposite, material);
                }
            }

            for (int index = 0; index < state.PlatformEffectors2D.Count; index++)
            {
                ParsedPlatformEffector2DRecord record = state.PlatformEffectors2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _scenePlatformEffectors2D[shiftedEntity] = new ScenePlatformEffector2DMetadata(
                        record.UseOneWay,
                        record.SurfaceArc,
                        record.RotationalOffset,
                        record.SideArc,
                        record.UseSideFriction,
                        record.UseSideBounce);
                }
            }

            for (int index = 0; index < state.SurfaceEffectors2D.Count; index++)
            {
                ParsedSurfaceEffector2DRecord record = state.SurfaceEffectors2D[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneSurfaceEffectors2D[shiftedEntity] = new SceneSurfaceEffector2DMetadata(
                        record.Speed,
                        record.ForceScale,
                        record.UseFriction,
                        record.UseBounce);
                }
            }
        }

        private static PhysicsMaterial2D? ResolveScenePhysicsMaterial2D(
            string materialGuid,
            bool hasInlineMaterial,
            float inlineFriction,
            float inlineBounciness,
            string scenePath,
            ref PrefabGuidIndex? guidIndex)
        {
            if (!string.IsNullOrEmpty(materialGuid))
            {
                guidIndex ??= PrefabGuidIndex.Build(scenePath);
                string? assetPath = guidIndex.Resolve(materialGuid) ?? ResolveGuidNearScene(scenePath, materialGuid);
                if (!string.IsNullOrEmpty(assetPath) && File.Exists(assetPath))
                {
                    return ParsePhysicsMaterial2DFile(assetPath);
                }
            }
            if (hasInlineMaterial)
            {
                return new PhysicsMaterial2D
                {
                    friction = inlineFriction,
                    bounciness = inlineBounciness,
                };
            }
            return null;
        }

        private static PhysicsMaterial2D ParsePhysicsMaterial2DFile(string assetPath)
        {
            string name = Path.GetFileNameWithoutExtension(assetPath);
            float friction = 0.4f;
            float bounciness = 0f;
            string[] lines = File.ReadAllLines(assetPath);
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i].Trim();
                if (line.StartsWith("m_Name:", StringComparison.Ordinal))
                {
                    string parsedName = ParseStringValue(line);
                    if (!string.IsNullOrEmpty(parsedName))
                    {
                        name = parsedName;
                    }
                }
                else if (line.StartsWith("friction:", StringComparison.Ordinal) || line.StartsWith("m_Friction:", StringComparison.Ordinal))
                {
                    friction = ParseFloatValue(line, friction);
                }
                else if (line.StartsWith("bounciness:", StringComparison.Ordinal) || line.StartsWith("m_Bounciness:", StringComparison.Ordinal))
                {
                    bounciness = ParseFloatValue(line, bounciness);
                }
            }
            return new PhysicsMaterial2D(name)
            {
                friction = friction,
                bounciness = bounciness,
            };
        }

        private static string ResolveSceneTilemapPath(
            SceneDocumentParseState state,
            ParsedTilemapCollider2DRecord record,
            string scenePath,
            ref PrefabGuidIndex? guidIndex)
        {
            string guid = record.TilemapAssetGuid;
            if (string.IsNullOrEmpty(guid) && state.TilemapAssetGuids.TryGetValue(record.GameObjectFileId, out string? siblingGuid))
            {
                guid = siblingGuid ?? string.Empty;
            }
            if (!string.IsNullOrEmpty(guid))
            {
                guidIndex ??= PrefabGuidIndex.Build(scenePath);
                string? resolvedGuidPath = guidIndex.Resolve(guid) ?? ResolveGuidNearScene(scenePath, guid);
                if (!string.IsNullOrEmpty(resolvedGuidPath) && File.Exists(resolvedGuidPath))
                {
                    return resolvedGuidPath;
                }
            }

            string path = record.TilemapAssetPath;
            if (string.IsNullOrEmpty(path) && state.TilemapAssetPaths.TryGetValue(record.GameObjectFileId, out string? siblingPath))
            {
                path = siblingPath ?? string.Empty;
            }
            if (!string.IsNullOrEmpty(path))
            {
                if (File.Exists(path))
                {
                    return Path.GetFullPath(path);
                }
                string? sceneDir = Path.GetDirectoryName(Path.GetFullPath(scenePath));
                if (!string.IsNullOrEmpty(sceneDir))
                {
                    string candidate = Path.Combine(sceneDir, path);
                    if (File.Exists(candidate))
                    {
                        return Path.GetFullPath(candidate);
                    }
                }
                return path;
            }

            if (state.TilemapInlineYamls.TryGetValue(record.GameObjectFileId, out string? inlineYaml) && !string.IsNullOrEmpty(inlineYaml))
            {
                return inlineYaml;
            }

            return string.Empty;
        }

        private static string? ResolveGuidNearScene(string scenePath, string guid)
        {
            if (string.IsNullOrEmpty(scenePath) || string.IsNullOrEmpty(guid))
            {
                return null;
            }
            string? sceneDir = Path.GetDirectoryName(Path.GetFullPath(scenePath));
            if (string.IsNullOrEmpty(sceneDir) || !Directory.Exists(sceneDir))
            {
                return null;
            }
            string[] metaFiles = Directory.GetFiles(sceneDir, "*.meta", SearchOption.AllDirectories);
            for (int i = 0; i < metaFiles.Length; i++)
            {
                string[] lines = File.ReadAllLines(metaFiles[i]);
                for (int j = 0; j < lines.Length; j++)
                {
                    string line = lines[j].Trim();
                    if (line.StartsWith("guid:", StringComparison.Ordinal))
                    {
                        string candidateGuid = line.Substring(5).Trim();
                        if (string.Equals(candidateGuid, guid, StringComparison.Ordinal))
                        {
                            return metaFiles[i].Substring(0, metaFiles[i].Length - ".meta".Length);
                        }
                    }
                }
            }
            return null;
        }

        private void ApplySceneTransformRecords(SceneDocumentParseState state)
        {
            for (int index = 0; index < state.Transforms.Count; index++)
            {
                ParsedTransformRecord record = state.Transforms[index];
                if (state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong localEntity))
                {
                    ulong shiftedEntity = _nextSceneEntityBase + localEntity;
                    _sceneTransforms[shiftedEntity] = new SceneTransformMetadata(
                        record.Position,
                        record.Rotation,
                        record.Scale);
                }
            }
        }

        private void ApplySceneUiRecords(SceneDocumentParseState state)
        {
            for (int index = 0; index < state.UiComponents.Count; index++)
            {
                ParsedUiComponentRecord record = state.UiComponents[index];
                if (!state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong uiLocalEntity))
                {
                    continue;
                }

                _sceneUiComponents[(_nextSceneEntityBase + uiLocalEntity, record.Kind)] = record;
            }
        }

        private void ApplySceneParentEntities(SceneDocumentParseState state, string scenePath)
        {
            Dictionary<string, string> transformGameObjects = new Dictionary<string, string>(StringComparer.Ordinal);
            for (int index = 0; index < state.Transforms.Count; index++)
            {
                transformGameObjects[state.Transforms[index].TransformFileId] = state.Transforms[index].GameObjectFileId;
            }
            for (int index = 0; index < state.Transforms.Count; index++)
            {
                ParsedTransformRecord record = state.Transforms[index];
                if (record.FatherFileId.Length == 0 || record.FatherFileId == "0")
                {
                    continue;
                }
                if (!transformGameObjects.TryGetValue(record.FatherFileId, out string? parentGameObjectFileId))
                {
                    continue;
                }
                if (!state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong childLocal)
                    || !state.GameObjectEntities.TryGetValue(parentGameObjectFileId, out ulong parentLocal))
                {
                    continue;
                }
                _sceneParentEntities[_nextSceneEntityBase + childLocal] = _nextSceneEntityBase + parentLocal;
            }

            SeedPrefabInstanceTransforms(scenePath, state.PrefabInstances, state.GameObjectEntities, transformGameObjects);
        }

        private void ApplySceneCameraRecords(SceneDocumentParseState state)
        {
            for (int index = 0; index < state.Cameras.Count; index++)
            {
                KeyValuePair<string, SceneCameraSettings> cameraEntry = state.Cameras[index];
                if (!state.GameObjectEntities.TryGetValue(cameraEntry.Key, out ulong cameraLocalEntity))
                {
                    continue;
                }

                ulong cameraEntity = _nextSceneEntityBase + cameraLocalEntity;
                _sceneCameras[cameraEntity] = cameraEntry.Value;
                bool created = !_entityGameObjects.ContainsKey(cameraEntity);
                GameObject cameraOwner = EnsureSceneGameObject(cameraEntity);
                WireSceneParent(cameraEntity, cameraOwner);
                ApplySceneCameraComponent(cameraEntity, cameraOwner);
                if (created)
                {
                    _materializedSceneCameras.Add(cameraOwner);
                }
            }
        }

        private void ApplySceneAudioSourceRecords(SceneDocumentParseState state, string scenePath)
        {
            if (state.AudioSources.Count > 0)
            {
                PrefabGuidIndex guidIndex = PrefabGuidIndex.Build(scenePath);
                for (int index = 0; index < state.AudioSources.Count; index++)
                {
                    ParsedAudioSourceRecord record = state.AudioSources[index];
                    if (!state.GameObjectEntities.TryGetValue(record.GameObjectFileId, out ulong audioLocalEntity))
                    {
                        continue;
                    }

                    string clipPath = !string.IsNullOrEmpty(record.ClipGuid)
                        ? (guidIndex.Resolve(record.ClipGuid) ?? string.Empty)
                        : string.Empty;
                    string clipName = !string.IsNullOrEmpty(clipPath)
                        ? Path.GetFileNameWithoutExtension(clipPath)
                        : "SceneAudioClip";
                    ulong audioEntity = _nextSceneEntityBase + audioLocalEntity;
                    _sceneAudioSources[audioEntity] = new SceneAudioSourceMetadata(
                        record.ClipGuid,
                        clipPath,
                        clipName,
                        record.Volume,
                        record.Pitch,
                        record.SpatialBlend,
                        record.MinDistance,
                        Math.Max(record.MaxDistance, record.MinDistance),
                        record.Priority,
                        record.Looping,
                        record.PlayOnAwake,
                        record.Mute,
                        record.Enabled);
                }
            }
        }

        private void SeedPrefabInstanceTransforms(
            string scenePath,
            List<PrefabInstanceInfo> instances,
            Dictionary<string, ulong> gameObjectEntities,
            Dictionary<string, string> transformGameObjects)
        {
            if (instances.Count == 0)
            {
                return;
            }

            PrefabGuidIndex guidIndex = PrefabGuidIndex.Build(scenePath);
            ulong nextEntity = _nextSceneEntityBase + (ulong)gameObjectEntities.Count;
            for (int index = 0; index < instances.Count; index++)
            {
                PrefabInstanceInfo instance = instances[index];
                string? prefabPath = guidIndex.Resolve(instance.SourceGuid);
                if (prefabPath == null)
                {
                    continue;
                }

                PrefabRootResolution resolution = ResolvePrefabRoot(prefabPath, guidIndex, 0);
                Vector3 position = resolution.Position;
                Quaternion rotation = resolution.Rotation;
                Vector3 scale = resolution.Scale;
                ApplyPrefabModifications(
                    ref position,
                    ref rotation,
                    ref scale,
                    instance.Modifications,
                    resolution.RootTransformFileId);
                _sceneTransforms[nextEntity] = new SceneTransformMetadata(position, rotation, scale);
                if (instance.TransformParentFileId.Length > 0
                    && transformGameObjects.TryGetValue(instance.TransformParentFileId, out string? parentGameObjectFileId)
                    && gameObjectEntities.TryGetValue(parentGameObjectFileId, out ulong parentLocalEntity))
                {
                    _sceneParentEntities[nextEntity] = _nextSceneEntityBase + parentLocalEntity;
                }

                nextEntity += (ulong)resolution.EntityCount + (ulong)instance.AddedGameObjectFileIds.Count;
            }
        }

        private static PrefabRootResolution ResolvePrefabRoot(string prefabPath, PrefabGuidIndex guidIndex, int depth)
        {
            PrefabFileInfo file = ParsePrefabFile(prefabPath);
            if (file.Instances.Count == 1 && depth < MaxPrefabInstanceDepth)
            {
                PrefabInstanceInfo variant = file.Instances[0];
                string? sourcePath = guidIndex.Resolve(variant.SourceGuid);
                if (sourcePath != null)
                {
                    PrefabRootResolution inner = ResolvePrefabRoot(sourcePath, guidIndex, depth + 1);
                    Vector3 position = inner.Position;
                    Quaternion rotation = inner.Rotation;
                    Vector3 scale = inner.Scale;
                    ApplyPrefabModifications(
                        ref position,
                        ref rotation,
                        ref scale,
                        variant.Modifications,
                        inner.RootTransformFileId);
                    return new PrefabRootResolution(
                        position,
                        rotation,
                        scale,
                        inner.EntityCount + variant.AddedGameObjectFileIds.Count,
                        inner.RootTransformFileId);
                }
            }

            return new PrefabRootResolution(
                file.RootLocalPosition,
                file.RootLocalRotation,
                file.RootLocalScale,
                file.OwnEntityCount,
                file.RootTransformFileId);
        }

        private static void ApplyPrefabModifications(
            ref Vector3 position,
            ref Quaternion rotation,
            ref Vector3 scale,
            List<PrefabModification> modifications,
            string rootTransformFileId)
        {
            for (int index = 0; index < modifications.Count; index++)
            {
                PrefabModification modification = modifications[index];
                if (!string.Equals(modification.TargetFileId, rootTransformFileId, StringComparison.Ordinal))
                {
                    continue;
                }

                string propertyPath = modification.PropertyPath;
                if (!propertyPath.StartsWith("m_LocalPosition.", StringComparison.Ordinal)
                    && !propertyPath.StartsWith("m_LocalRotation.", StringComparison.Ordinal)
                    && !propertyPath.StartsWith("m_LocalScale.", StringComparison.Ordinal))
                {
                    continue;
                }

                if (!float.TryParse(modification.Value, NumberStyles.Float, CultureInfo.InvariantCulture, out float value))
                {
                    continue;
                }

                string axis = propertyPath.Substring(propertyPath.LastIndexOf('.') + 1);
                if (propertyPath.StartsWith("m_LocalPosition.", StringComparison.Ordinal))
                {
                    position = axis switch
                    {
                        "x" => new Vector3(value, position.y, position.z),
                        "y" => new Vector3(position.x, value, position.z),
                        "z" => new Vector3(position.x, position.y, value),
                        _ => position,
                    };
                }
                else if (propertyPath.StartsWith("m_LocalRotation.", StringComparison.Ordinal))
                {
                    rotation = axis switch
                    {
                        "x" => new Quaternion(value, rotation.y, rotation.z, rotation.w),
                        "y" => new Quaternion(rotation.x, value, rotation.z, rotation.w),
                        "z" => new Quaternion(rotation.x, rotation.y, value, rotation.w),
                        "w" => new Quaternion(rotation.x, rotation.y, rotation.z, value),
                        _ => rotation,
                    };
                }
                else
                {
                    scale = axis switch
                    {
                        "x" => new Vector3(value, scale.y, scale.z),
                        "y" => new Vector3(scale.x, value, scale.z),
                        "z" => new Vector3(scale.x, scale.y, value),
                        _ => scale,
                    };
                }
            }
        }

        private static PrefabFileInfo ParsePrefabFile(string path)
        {
            PrefabFileInfo info = new PrefabFileInfo();
            string[] lines = File.ReadAllLines(path);
            int classId = 0;
            string fileId = string.Empty;
            string gameObjectFileId = string.Empty;
            string fatherFileId = string.Empty;
            string prefabInstanceOwner = string.Empty;
            Vector3 localPosition = Vector3.zero;
            Quaternion localRotation = Quaternion.identity;
            Vector3 localScale = Vector3.one;
            PrefabInstanceInfo? instance = null;
            string modificationTarget = string.Empty;
            string? modificationProperty = null;

            for (int index = 0; index < lines.Length; index++)
            {
                string line = lines[index].Trim();
                if (line.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    FlushPrefabDocument(
                        info,
                        classId,
                        fileId,
                        gameObjectFileId,
                        fatherFileId,
                        prefabInstanceOwner,
                        localPosition,
                        localRotation,
                        localScale);
                    ParseDocumentHeader(line, out classId, out fileId);
                    gameObjectFileId = string.Empty;
                    fatherFileId = string.Empty;
                    prefabInstanceOwner = string.Empty;
                    localPosition = Vector3.zero;
                    localRotation = Quaternion.identity;
                    localScale = Vector3.one;
                    instance = null;
                    modificationTarget = string.Empty;
                    modificationProperty = null;
                    if (classId == PREFAB_INSTANCE_CLASS)
                    {
                        instance = new PrefabInstanceInfo { FileId = fileId };
                        info.Instances.Add(instance);
                    }
                    continue;
                }

                if (classId == GAME_OBJECT_CLASS)
                {
                    if (line.StartsWith("m_PrefabInstance:", StringComparison.Ordinal))
                    {
                        prefabInstanceOwner = ExtractFileId(line);
                    }
                }
                else if (classId == TRANSFORM_CLASS)
                {
                    if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                    {
                        gameObjectFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_Father:", StringComparison.Ordinal))
                    {
                        fatherFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("m_LocalPosition:", StringComparison.Ordinal))
                    {
                        localPosition = ParseVector3Value(line, Vector3.zero);
                    }
                    else if (line.StartsWith("m_LocalRotation:", StringComparison.Ordinal))
                    {
                        localRotation = ParseQuaternionValue(line, Quaternion.identity);
                    }
                    else if (line.StartsWith("m_LocalScale:", StringComparison.Ordinal))
                    {
                        localScale = ParseVector3Value(line, Vector3.one);
                    }
                }
                else if (classId == PREFAB_INSTANCE_CLASS && instance != null)
                {
                    if (line.StartsWith("m_SourcePrefab:", StringComparison.Ordinal))
                    {
                        instance.SourceGuid = ExtractGuidValue(line);
                    }
                    else if (line.StartsWith("m_TransformParent:", StringComparison.Ordinal))
                    {
                        instance.TransformParentFileId = ExtractFileId(line);
                    }
                    else if (line.StartsWith("- target:", StringComparison.Ordinal))
                    {
                        modificationTarget = ExtractFileId(line);
                        modificationProperty = null;
                    }
                    else if (line.StartsWith("propertyPath:", StringComparison.Ordinal))
                    {
                        modificationProperty = ParseStringValue(line);
                    }
                    else if (line.StartsWith("value:", StringComparison.Ordinal) && modificationProperty != null)
                    {
                        instance.Modifications.Add(new PrefabModification(
                            modificationTarget,
                            modificationProperty,
                            ParseStringValue(line)));
                        modificationProperty = null;
                    }
                    else if (line.StartsWith("addedObject:", StringComparison.Ordinal))
                    {
                        string addedFileId = ExtractFileId(line);
                        if (addedFileId.Length > 0)
                        {
                            instance.AddedGameObjectFileIds.Add(addedFileId);
                        }
                    }
                }
            }

            FlushPrefabDocument(
                info,
                classId,
                fileId,
                gameObjectFileId,
                fatherFileId,
                prefabInstanceOwner,
                localPosition,
                localRotation,
                localScale);

            foreach (PrefabInstanceInfo prefabInstance in info.Instances)
            {
                foreach (string addedFileId in prefabInstance.AddedGameObjectFileIds)
                {
                    info.AddedGameObjectFileIds.Add(addedFileId);
                }
            }

            return info;
        }

        private static void FlushPrefabDocument(
            PrefabFileInfo info,
            int classId,
            string fileId,
            string gameObjectFileId,
            string fatherFileId,
            string prefabInstanceOwner,
            Vector3 localPosition,
            Quaternion localRotation,
            Vector3 localScale)
        {
            if (classId == GAME_OBJECT_CLASS && fileId.Length > 0)
            {
                info.GameObjectFileIds.Add(fileId);
                if (prefabInstanceOwner.Length > 0 && prefabInstanceOwner != "0")
                {
                    info.GameObjectPrefabInstanceFileIds[fileId] = prefabInstanceOwner;
                }
            }
            else if (classId == TRANSFORM_CLASS && fileId.Length > 0)
            {
                if (fatherFileId.Length > 0 && fatherFileId != "0")
                {
                    return;
                }

                info.RootTransformFileId = fileId;
                info.RootGameObjectFileId = gameObjectFileId;
                info.RootLocalPosition = localPosition;
                info.RootLocalRotation = localRotation;
                info.RootLocalScale = localScale;
            }
        }

        private static string ExtractGuidValue(string line)
        {
            int index = line.IndexOf("guid:", StringComparison.Ordinal);
            if (index < 0)
            {
                return string.Empty;
            }

            string tail = line.Substring(index + 5).Trim();
            int endIndex = tail.IndexOfAny(new char[] { ',', '}', ' ' });
            return endIndex >= 0 ? tail.Substring(0, endIndex).Trim() : tail;
        }

        private static void FlushDocument(
            int classId,
            string fileId,
            string gameObjectFileId,
            string name,
            float mass,
            bool useGravity,
            bool isKinematic,
            float radius,
            float height,
            int direction,
            bool isTrigger,
            Vector3 center,
            Vector3 size,
            bool convex,
            string meshFileId,
            string meshGuid,
            SceneCameraSettings camera,
            ref ulong localEntityIndex,
            Dictionary<string, ulong> gameObjectEntities,
            Dictionary<string, string> gameObjectNames,
            List<ParsedRigidbodyRecord> rigidbodies,
            List<ParsedSphereColliderRecord> sphereColliders,
            List<ParsedBoxColliderRecord> boxColliders,
            List<ParsedCapsuleColliderRecord> capsuleColliders,
            List<ParsedMeshColliderRecord> meshColliders,
            List<KeyValuePair<string, SceneCameraSettings>> cameras)
        {
            if (classId == 1 && !string.IsNullOrEmpty(fileId))
            {
                gameObjectEntities[fileId] = localEntityIndex;
                gameObjectNames[fileId] = name;
                localEntityIndex++;
            }
            else if (classId == 54 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                rigidbodies.Add(new ParsedRigidbodyRecord(gameObjectFileId, mass, useGravity, isKinematic));
            }
            else if (classId == 65 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                boxColliders.Add(new ParsedBoxColliderRecord(gameObjectFileId, size, center, isTrigger));
            }
            else if (classId == 135 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                sphereColliders.Add(new ParsedSphereColliderRecord(gameObjectFileId, radius, center, isTrigger));
            }
            else if (classId == 136 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                capsuleColliders.Add(new ParsedCapsuleColliderRecord(gameObjectFileId, radius, height, direction, center, isTrigger));
            }
            else if (classId == MESH_COLLIDER_CLASS && !string.IsNullOrEmpty(gameObjectFileId))
            {
                meshColliders.Add(new ParsedMeshColliderRecord(gameObjectFileId, meshFileId, meshGuid, convex, isTrigger, center));
            }
            else if (classId == 20 && !string.IsNullOrEmpty(gameObjectFileId))
            {
                cameras.Add(new KeyValuePair<string, SceneCameraSettings>(gameObjectFileId, camera));
            }
        }

        private static void ParseDocumentHeader(string headerLine, out int classId, out string fileId)
        {
            classId = 0;
            fileId = string.Empty;
            if (headerLine.EndsWith("stripped", StringComparison.Ordinal))
            {
                return;
            }
            int tagStart = headerLine.IndexOf("!u!", StringComparison.Ordinal);
            int ampIndex = headerLine.IndexOf('&', StringComparison.Ordinal);
            if (tagStart < 0 || ampIndex <= tagStart + 3)
            {
                return;
            }
            string classPart = headerLine.Substring(tagStart + 3, ampIndex - (tagStart + 3)).Trim();
            if (!int.TryParse(classPart, NumberStyles.Integer, CultureInfo.InvariantCulture, out classId))
            {
                classId = 0;
                return;
            }
            fileId = headerLine.Substring(ampIndex + 1).Trim();
        }

        private static string ExtractFileId(string line)
        {
            int keyIndex = line.IndexOf("fileID:", StringComparison.Ordinal);
            if (keyIndex < 0)
            {
                return string.Empty;
            }
            string tail = line.Substring(keyIndex + 7).Trim();
            int endIndex = tail.IndexOfAny(new char[] { ',', '}', ' ' });
            return endIndex >= 0 ? tail.Substring(0, endIndex).Trim() : tail;
        }

        private static float ParseFloatValue(string line, float fallback)
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return fallback;
            }
            string raw = line.Substring(colon + 1).Trim();
            if (float.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out float parsed))
            {
                return parsed;
            }
            return fallback;
        }

        private static Vector2 ParseVector2Value(string line, Vector2 fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float x = fallback.x;
            float y = fallback.y;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("x:", StringComparison.Ordinal))
                {
                    x = ParseFloatValue(part, fallback.x);
                }
                else if (part.StartsWith("y:", StringComparison.Ordinal))
                {
                    y = ParseFloatValue(part, fallback.y);
                }
            }
            return new Vector2(x, y);
        }

        private static RigidbodyType2D ToRigidbodyType2D(int bodyType)
        {
            switch (bodyType)
            {
                case 1:
                    return RigidbodyType2D.Kinematic;
                case 2:
                    return RigidbodyType2D.Static;
                default:
                    return RigidbodyType2D.Dynamic;
            }
        }

        private static RigidbodyInterpolation2D ToRigidbodyInterpolation2D(int interpolation)
        {
            switch (interpolation)
            {
                case 1:
                    return RigidbodyInterpolation2D.Interpolate;
                case 2:
                    return RigidbodyInterpolation2D.Extrapolate;
                default:
                    return RigidbodyInterpolation2D.None;
            }
        }

        private static CollisionDetectionMode2D ToCollisionDetectionMode2D(int mode)
        {
            switch (mode)
            {
                case 1:
                    return CollisionDetectionMode2D.Continuous;
                case 2:
                    return CollisionDetectionMode2D.ContinuousSpeculative;
                default:
                    return CollisionDetectionMode2D.Discrete;
            }
        }

        private static bool ParseBoolValue(string line, bool fallback)
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return fallback;
            }
            string raw = line.Substring(colon + 1).Trim();
            if (string.Equals(raw, "true", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            if (string.Equals(raw, "false", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }
            if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out double numeric))
            {
                return numeric != 0.0;
            }
            return fallback;
        }

        private static string ParseStringValue(string line)
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return string.Empty;
            }
            return line.Substring(colon + 1).Trim().Trim('"', '\'');
        }

        private static Color ParseColorValue(string line, Color fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float red = fallback.r;
            float green = fallback.g;
            float blue = fallback.b;
            float alpha = fallback.a;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("r:", StringComparison.Ordinal))
                {
                    red = ParseFloatValue(part, fallback.r);
                }
                else if (part.StartsWith("g:", StringComparison.Ordinal))
                {
                    green = ParseFloatValue(part, fallback.g);
                }
                else if (part.StartsWith("b:", StringComparison.Ordinal))
                {
                    blue = ParseFloatValue(part, fallback.b);
                }
                else if (part.StartsWith("a:", StringComparison.Ordinal))
                {
                    alpha = ParseFloatValue(part, fallback.a);
                }
            }
            return new Color(red, green, blue, alpha);
        }

        private static Vector3 ParseVector3Value(string line, Vector3 fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float x = fallback.x;
            float y = fallback.y;
            float z = fallback.z;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("x:", StringComparison.Ordinal))
                {
                    x = ParseFloatValue(part, fallback.x);
                }
                else if (part.StartsWith("y:", StringComparison.Ordinal))
                {
                    y = ParseFloatValue(part, fallback.y);
                }
                else if (part.StartsWith("z:", StringComparison.Ordinal))
                {
                    z = ParseFloatValue(part, fallback.z);
                }
            }
            return new Vector3(x, y, z);
        }

        private static Quaternion ParseQuaternionValue(string line, Quaternion fallback)
        {
            int braceStart = line.IndexOf('{', StringComparison.Ordinal);
            int braceEnd = line.LastIndexOf('}');
            if (braceStart < 0 || braceEnd <= braceStart)
            {
                return fallback;
            }

            float x = fallback.x;
            float y = fallback.y;
            float z = fallback.z;
            float w = fallback.w;
            string inner = line.Substring(braceStart + 1, braceEnd - braceStart - 1);
            string[] parts = inner.Split(',');
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("x:", StringComparison.Ordinal))
                {
                    x = ParseFloatValue(part, fallback.x);
                }
                else if (part.StartsWith("y:", StringComparison.Ordinal))
                {
                    y = ParseFloatValue(part, fallback.y);
                }
                else if (part.StartsWith("z:", StringComparison.Ordinal))
                {
                    z = ParseFloatValue(part, fallback.z);
                }
                else if (part.StartsWith("w:", StringComparison.Ordinal))
                {
                    w = ParseFloatValue(part, fallback.w);
                }
            }
            return new Quaternion(x, y, z, w);
        }

        private readonly struct ParsedTransformRecord
        {
            public ParsedTransformRecord(
                string transformFileId,
                string gameObjectFileId,
                string fatherFileId,
                Vector3 position,
                Quaternion rotation,
                Vector3 scale)
            {
                TransformFileId = transformFileId;
                GameObjectFileId = gameObjectFileId;
                FatherFileId = fatherFileId;
                Position = position;
                Rotation = rotation;
                Scale = scale;
            }

            public string TransformFileId { get; }

            public string GameObjectFileId { get; }

            public string FatherFileId { get; }

            public Vector3 Position { get; }

            public Quaternion Rotation { get; }

            public Vector3 Scale { get; }
        }

        private sealed class PrefabInstanceInfo
        {
            public string FileId { get; set; } = string.Empty;

            public string SourceGuid { get; set; } = string.Empty;

            public string TransformParentFileId { get; set; } = string.Empty;

            public List<PrefabModification> Modifications { get; } = new List<PrefabModification>();

            public List<string> AddedGameObjectFileIds { get; } = new List<string>();
        }

        private readonly struct PrefabModification
        {
            public PrefabModification(string targetFileId, string propertyPath, string value)
            {
                TargetFileId = targetFileId;
                PropertyPath = propertyPath;
                Value = value;
            }

            public string TargetFileId { get; }

            public string PropertyPath { get; }

            public string Value { get; }
        }

        private sealed class PrefabFileInfo
        {
            public List<string> GameObjectFileIds { get; } = new List<string>();

            public Dictionary<string, string> GameObjectPrefabInstanceFileIds { get; } = new Dictionary<string, string>(StringComparer.Ordinal);

            public HashSet<string> AddedGameObjectFileIds { get; } = new HashSet<string>(StringComparer.Ordinal);

            public List<PrefabInstanceInfo> Instances { get; } = new List<PrefabInstanceInfo>();

            public string RootTransformFileId { get; set; } = string.Empty;

            public string RootGameObjectFileId { get; set; } = string.Empty;

            public Vector3 RootLocalPosition { get; set; } = Vector3.zero;

            public Quaternion RootLocalRotation { get; set; } = Quaternion.identity;

            public Vector3 RootLocalScale { get; set; } = Vector3.one;

            public int OwnEntityCount
            {
                get
                {
                    int count = 0;
                    for (int index = 0; index < GameObjectFileIds.Count; index++)
                    {
                        string fileId = GameObjectFileIds[index];
                        if (AddedGameObjectFileIds.Contains(fileId))
                        {
                            continue;
                        }

                        if (GameObjectPrefabInstanceFileIds.TryGetValue(fileId, out string? ownerFileId)
                            && IsInstanceDocument(ownerFileId))
                        {
                            continue;
                        }

                        count++;
                    }

                    return count;
                }
            }

            private bool IsInstanceDocument(string fileId)
            {
                for (int index = 0; index < Instances.Count; index++)
                {
                    if (string.Equals(Instances[index].FileId, fileId, StringComparison.Ordinal))
                    {
                        return true;
                    }
                }

                return false;
            }
        }

        private readonly struct PrefabRootResolution
        {
            public PrefabRootResolution(
                Vector3 position,
                Quaternion rotation,
                Vector3 scale,
                int entityCount,
                string rootTransformFileId)
            {
                Position = position;
                Rotation = rotation;
                Scale = scale;
                EntityCount = entityCount;
                RootTransformFileId = rootTransformFileId;
            }

            public Vector3 Position { get; }

            public Quaternion Rotation { get; }

            public Vector3 Scale { get; }

            public int EntityCount { get; }

            public string RootTransformFileId { get; }
        }

        private sealed class PrefabGuidIndex
        {
            private readonly Dictionary<string, string> _assetPaths = new Dictionary<string, string>(StringComparer.Ordinal);

            public static PrefabGuidIndex Build(string scenePath)
            {
                PrefabGuidIndex index = new PrefabGuidIndex();
                string? projectRoot = FindProjectRoot(scenePath);
                if (projectRoot == null)
                {
                    return index;
                }

                string assets = Path.Combine(projectRoot, "Assets");
                if (!Directory.Exists(assets))
                {
                    assets = projectRoot;
                }

                string[] metaFiles = Directory.GetFiles(assets, "*.meta", SearchOption.AllDirectories);
                Array.Sort(metaFiles, StringComparer.Ordinal);
                for (int metaIndex = 0; metaIndex < metaFiles.Length; metaIndex++)
                {
                    string? guid = ReadMetaGuid(metaFiles[metaIndex]);
                    if (guid == null)
                    {
                        continue;
                    }

                    index._assetPaths[guid] = metaFiles[metaIndex].Substring(0, metaFiles[metaIndex].Length - ".meta".Length);
                }

                return index;
            }

            public string? Resolve(string guid)
            {
                if (guid.Length == 0)
                {
                    return null;
                }

                return _assetPaths.TryGetValue(guid, out string? path) ? path : null;
            }

            private static string? FindProjectRoot(string scenePath)
            {
                string? directory = Path.GetDirectoryName(Path.GetFullPath(scenePath));
                while (!string.IsNullOrEmpty(directory))
                {
                    if (Directory.Exists(Path.Combine(directory, "ProjectSettings")))
                    {
                        return directory;
                    }

                    directory = Path.GetDirectoryName(directory);
                }

                return null;
            }

            private static string? ReadMetaGuid(string metaPath)
            {
                string[] lines = File.ReadAllLines(metaPath);
                for (int index = 0; index < lines.Length; index++)
                {
                    string line = lines[index].Trim();
                    if (!line.StartsWith("guid:", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string guid = line.Substring("guid:".Length).Trim();
                    return guid.Length > 0 ? guid : null;
                }

                return null;
            }
        }

        private readonly struct ParsedRigidbodyRecord
        {
            public ParsedRigidbodyRecord(string gameObjectFileId, float mass, bool useGravity, bool isKinematic)
            {
                GameObjectFileId = gameObjectFileId;
                Mass = mass;
                UseGravity = useGravity;
                IsKinematic = isKinematic;
            }

            public string GameObjectFileId { get; }

            public float Mass { get; }

            public bool UseGravity { get; }

            public bool IsKinematic { get; }
        }

        private readonly struct ParsedRigidbody2DRecord
        {
            public ParsedRigidbody2DRecord(string gameObjectFileId, int bodyType, bool simulated, float mass, float gravityScale, int interpolation, int collisionDetectionMode, int constraints = 0)
            {
                GameObjectFileId = gameObjectFileId;
                BodyType = bodyType;
                Simulated = simulated;
                Mass = mass;
                GravityScale = gravityScale;
                Interpolation = interpolation;
                CollisionDetectionMode = collisionDetectionMode;
                Constraints = constraints;
            }

            public string GameObjectFileId { get; }

            public int BodyType { get; }

            public bool Simulated { get; }

            public float Mass { get; }

            public float GravityScale { get; }

            public int Interpolation { get; }

            public int CollisionDetectionMode { get; }

            public int Constraints { get; }
        }

        private readonly struct ParsedBoxCollider2DRecord
        {
            public ParsedBoxCollider2DRecord(string gameObjectFileId, Vector2 size, Vector2 offset, bool isTrigger, bool usedByEffector = false, bool usedByComposite = false, string materialGuid = "", bool hasInlineMaterial = false, float friction = 0.4f, float bounciness = 0f)
            {
                GameObjectFileId = gameObjectFileId;
                Size = size;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public Vector2 Size { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public bool UsedByComposite { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedCircleCollider2DRecord
        {
            public ParsedCircleCollider2DRecord(string gameObjectFileId, float radius, Vector2 offset, bool isTrigger, bool usedByEffector = false, bool usedByComposite = false, string materialGuid = "", bool hasInlineMaterial = false, float friction = 0.4f, float bounciness = 0f)
            {
                GameObjectFileId = gameObjectFileId;
                Radius = radius;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public float Radius { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public bool UsedByComposite { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedCapsuleCollider2DRecord
        {
            public ParsedCapsuleCollider2DRecord(string gameObjectFileId, Vector2 size, int direction, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, string materialGuid, bool hasInlineMaterial, float friction, float bounciness)
            {
                GameObjectFileId = gameObjectFileId;
                Size = size;
                Direction = direction;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public Vector2 Size { get; }
            public int Direction { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public bool UsedByComposite { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedPolygonCollider2DRecord
        {
            public ParsedPolygonCollider2DRecord(string gameObjectFileId, Vector2[] points, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, string materialGuid, bool hasInlineMaterial, float friction, float bounciness)
            {
                GameObjectFileId = gameObjectFileId;
                Points = points ?? Array.Empty<Vector2>();
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public Vector2[] Points { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public bool UsedByComposite { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedEdgeCollider2DRecord
        {
            public ParsedEdgeCollider2DRecord(string gameObjectFileId, Vector2[] points, float edgeRadius, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, string materialGuid, bool hasInlineMaterial, float friction, float bounciness)
            {
                GameObjectFileId = gameObjectFileId;
                Points = points ?? Array.Empty<Vector2>();
                EdgeRadius = edgeRadius;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public Vector2[] Points { get; }
            public float EdgeRadius { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public bool UsedByComposite { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedCompositeCollider2DRecord
        {
            public ParsedCompositeCollider2DRecord(string gameObjectFileId, int geometryType, int generationType, float vertexDistance, float offsetDistance, float edgeRadius, Vector2 offset, bool isTrigger, bool usedByEffector, string materialGuid, bool hasInlineMaterial, float friction, float bounciness)
            {
                GameObjectFileId = gameObjectFileId;
                GeometryType = geometryType;
                GenerationType = generationType;
                VertexDistance = vertexDistance;
                OffsetDistance = offsetDistance;
                EdgeRadius = edgeRadius;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public int GeometryType { get; }
            public int GenerationType { get; }
            public float VertexDistance { get; }
            public float OffsetDistance { get; }
            public float EdgeRadius { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedTilemapCollider2DRecord
        {
            public ParsedTilemapCollider2DRecord(string gameObjectFileId, uint maximumTileChangeCount, float extrusionFactor, string tilemapAssetPath, string tilemapAssetGuid, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, string materialGuid, bool hasInlineMaterial, float friction, float bounciness)
            {
                GameObjectFileId = gameObjectFileId;
                MaximumTileChangeCount = maximumTileChangeCount;
                ExtrusionFactor = extrusionFactor;
                TilemapAssetPath = tilemapAssetPath ?? string.Empty;
                TilemapAssetGuid = tilemapAssetGuid ?? string.Empty;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                MaterialGuid = materialGuid ?? string.Empty;
                HasInlineMaterial = hasInlineMaterial;
                Friction = friction;
                Bounciness = bounciness;
            }
            public string GameObjectFileId { get; }
            public uint MaximumTileChangeCount { get; }
            public float ExtrusionFactor { get; }
            public string TilemapAssetPath { get; }
            public string TilemapAssetGuid { get; }
            public Vector2 Offset { get; }
            public bool IsTrigger { get; }
            public bool UsedByEffector { get; }
            public bool UsedByComposite { get; }
            public string MaterialGuid { get; }
            public bool HasInlineMaterial { get; }
            public float Friction { get; }
            public float Bounciness { get; }
        }

        private readonly struct ParsedPlatformEffector2DRecord
        {
            public ParsedPlatformEffector2DRecord(string gameObjectFileId, bool useOneWay, float surfaceArc, float rotationalOffset, float sideArc, bool useSideFriction, bool useSideBounce)
            {
                GameObjectFileId = gameObjectFileId;
                UseOneWay = useOneWay;
                SurfaceArc = surfaceArc;
                RotationalOffset = rotationalOffset;
                SideArc = sideArc;
                UseSideFriction = useSideFriction;
                UseSideBounce = useSideBounce;
            }

            public string GameObjectFileId { get; }

            public bool UseOneWay { get; }

            public float SurfaceArc { get; }

            public float RotationalOffset { get; }

            public float SideArc { get; }

            public bool UseSideFriction { get; }

            public bool UseSideBounce { get; }
        }

        private readonly struct ParsedSurfaceEffector2DRecord
        {
            public ParsedSurfaceEffector2DRecord(string gameObjectFileId, float speed, float forceScale, bool useFriction, bool useBounce)
            {
                GameObjectFileId = gameObjectFileId;
                Speed = speed;
                ForceScale = forceScale;
                UseFriction = useFriction;
                UseBounce = useBounce;
            }

            public string GameObjectFileId { get; }

            public float Speed { get; }

            public float ForceScale { get; }

            public bool UseFriction { get; }

            public bool UseBounce { get; }
        }

        private readonly struct ParsedUiComponentRecord
        {
            public ParsedUiComponentRecord(
                string gameObjectFileId,
                ScriptGlueRegistry.BuiltInUiComponent kind,
                bool enabled,
                bool interactable,
                string text,
                float fontSize,
                Color color,
                string sprite,
                int uiScaleMode,
                Vector2 referenceResolution,
                int screenMatchMode,
                float matchWidthOrHeight)
            {
                GameObjectFileId = gameObjectFileId;
                Kind = kind;
                Enabled = enabled;
                Interactable = interactable;
                Text = text;
                FontSize = fontSize;
                Color = color;
                Sprite = sprite;
                UiScaleMode = uiScaleMode;
                ReferenceResolution = referenceResolution;
                ScreenMatchMode = screenMatchMode;
                MatchWidthOrHeight = matchWidthOrHeight;
            }

            public string GameObjectFileId { get; }

            public ScriptGlueRegistry.BuiltInUiComponent Kind { get; }

            public bool Enabled { get; }

            public bool Interactable { get; }

            public string Text { get; }

            public float FontSize { get; }

            public Color Color { get; }

            public string Sprite { get; }

            public int UiScaleMode { get; }

            public Vector2 ReferenceResolution { get; }

            public int ScreenMatchMode { get; }

            public float MatchWidthOrHeight { get; }
        }

        private readonly struct ParsedSphereColliderRecord
        {
            public ParsedSphereColliderRecord(string gameObjectFileId, float radius, Vector3 center, bool isTrigger)
            {
                GameObjectFileId = gameObjectFileId;
                Radius = radius;
                Center = center;
                IsTrigger = isTrigger;
            }

            public string GameObjectFileId { get; }

            public float Radius { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ParsedBoxColliderRecord
        {
            public ParsedBoxColliderRecord(string gameObjectFileId, Vector3 size, Vector3 center, bool isTrigger)
            {
                GameObjectFileId = gameObjectFileId;
                Size = size;
                Center = center;
                IsTrigger = isTrigger;
            }

            public string GameObjectFileId { get; }

            public Vector3 Size { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ParsedCapsuleColliderRecord
        {
            public ParsedCapsuleColliderRecord(string gameObjectFileId, float radius, float height, int direction, Vector3 center, bool isTrigger)
            {
                GameObjectFileId = gameObjectFileId;
                Radius = radius;
                Height = height;
                Direction = direction;
                Center = center;
                IsTrigger = isTrigger;
            }

            public string GameObjectFileId { get; }

            public float Radius { get; }

            public float Height { get; }

            public int Direction { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ParsedMeshColliderRecord
        {
            public ParsedMeshColliderRecord(string gameObjectFileId, string meshFileId, string meshGuid, bool convex, bool isTrigger, Vector3 center)
            {
                GameObjectFileId = gameObjectFileId;
                MeshFileId = meshFileId;
                MeshGuid = meshGuid;
                Convex = convex;
                IsTrigger = isTrigger;
                Center = center;
            }

            public string GameObjectFileId { get; }

            public string MeshFileId { get; }

            public string MeshGuid { get; }

            public bool Convex { get; }

            public bool IsTrigger { get; }

            public Vector3 Center { get; }
        }

        private readonly struct ParsedAudioSourceRecord
        {
            public ParsedAudioSourceRecord(
                string gameObjectFileId,
                string clipGuid,
                float volume,
                float pitch,
                float spatialBlend,
                float minDistance,
                float maxDistance,
                int priority,
                bool looping,
                bool playOnAwake,
                bool mute,
                bool enabled)
            {
                GameObjectFileId = gameObjectFileId;
                ClipGuid = clipGuid;
                Volume = volume;
                Pitch = pitch;
                SpatialBlend = spatialBlend;
                MinDistance = minDistance;
                MaxDistance = maxDistance;
                Priority = priority;
                Looping = looping;
                PlayOnAwake = playOnAwake;
                Mute = mute;
                Enabled = enabled;
            }

            public string GameObjectFileId { get; }

            public string ClipGuid { get; }

            public float Volume { get; }

            public float Pitch { get; }

            public float SpatialBlend { get; }

            public float MinDistance { get; }

            public float MaxDistance { get; }

            public int Priority { get; }

            public bool Looping { get; }

            public bool PlayOnAwake { get; }

            public bool Mute { get; }

            public bool Enabled { get; }
        }

        private sealed class SceneDocumentParseState
        {
            public ulong LocalEntityIndex;
            public Dictionary<string, ulong> GameObjectEntities = new Dictionary<string, ulong>(StringComparer.Ordinal);
            public Dictionary<string, string> GameObjectNames = new Dictionary<string, string>(StringComparer.Ordinal);
            public Dictionary<string, int> GameObjectLayers = new Dictionary<string, int>(StringComparer.Ordinal);
            public int CurrentClassId;
            public string CurrentFileId = string.Empty;
            public string CurrentGameObjectFileId = string.Empty;
            public string CurrentName = string.Empty;
            public int CurrentLayer;
            public float CurrentMass = 1f;
            public bool CurrentUseGravity = true;
            public bool CurrentIsKinematic;
            public float CurrentRadius = 0.5f;
            public float CurrentHeight = 2f;
            public int CurrentDirection = 1;
            public bool CurrentIsTrigger;
            public Vector3 CurrentCenter = Vector3.zero;
            public Vector3 CurrentSize = new Vector3(1f, 1f, 1f);
            public bool CurrentConvex;
            public ScenePhysics2DFields Current2D = new ScenePhysics2DFields();
            public string CurrentMeshFileId = string.Empty;
            public string CurrentMeshGuid = string.Empty;
            public List<ParsedRigidbodyRecord> Rigidbodies = new List<ParsedRigidbodyRecord>();
            public List<ParsedRigidbody2DRecord> Rigidbodies2D = new List<ParsedRigidbody2DRecord>();
            public List<ParsedBoxCollider2DRecord> BoxColliders2D = new List<ParsedBoxCollider2DRecord>();
            public List<ParsedCircleCollider2DRecord> CircleColliders2D = new List<ParsedCircleCollider2DRecord>();
            public List<ParsedCapsuleCollider2DRecord> CapsuleColliders2D = new List<ParsedCapsuleCollider2DRecord>();
            public List<ParsedPolygonCollider2DRecord> PolygonColliders2D = new List<ParsedPolygonCollider2DRecord>();
            public List<ParsedEdgeCollider2DRecord> EdgeColliders2D = new List<ParsedEdgeCollider2DRecord>();
            public List<ParsedCompositeCollider2DRecord> CompositeColliders2D = new List<ParsedCompositeCollider2DRecord>();
            public List<ParsedTilemapCollider2DRecord> TilemapColliders2D = new List<ParsedTilemapCollider2DRecord>();
            public Dictionary<string, string> TilemapAssetGuids = new Dictionary<string, string>(StringComparer.Ordinal);
            public Dictionary<string, string> TilemapAssetPaths = new Dictionary<string, string>(StringComparer.Ordinal);
            public Dictionary<string, string> TilemapInlineYamls = new Dictionary<string, string>(StringComparer.Ordinal);
            public List<string> CurrentTilemapRawLines = new List<string>();
            public List<ParsedPlatformEffector2DRecord> PlatformEffectors2D = new List<ParsedPlatformEffector2DRecord>();
            public List<ParsedSurfaceEffector2DRecord> SurfaceEffectors2D = new List<ParsedSurfaceEffector2DRecord>();
            public List<ParsedSphereColliderRecord> SphereColliders = new List<ParsedSphereColliderRecord>();
            public List<ParsedBoxColliderRecord> BoxColliders = new List<ParsedBoxColliderRecord>();
            public List<ParsedCapsuleColliderRecord> CapsuleColliders = new List<ParsedCapsuleColliderRecord>();
            public List<ParsedMeshColliderRecord> MeshColliders = new List<ParsedMeshColliderRecord>();
            public List<KeyValuePair<string, SceneCameraSettings>> Cameras = new List<KeyValuePair<string, SceneCameraSettings>>();
            public List<ParsedAudioSourceRecord> AudioSources = new List<ParsedAudioSourceRecord>();
            public string CurrentAudioGameObjectFileId = string.Empty;
            public string CurrentAudioClipGuid = string.Empty;
            public bool CurrentAudioEnabled = true;
            public float CurrentAudioVolume = 1f;
            public float CurrentAudioPitch = 1f;
            public bool CurrentAudioLoop;
            public bool CurrentAudioPlayOnAwake = true;
            public bool CurrentAudioMute;
            public int CurrentAudioPriority = 128;
            public float CurrentAudioSpatialBlend;
            public float CurrentAudioMinDistance = 1f;
            public float CurrentAudioMaxDistance = 500f;
            public List<ParsedUiComponentRecord> UiComponents = new List<ParsedUiComponentRecord>();
            public string CurrentUiGuid = string.Empty;
            public string CurrentUiGameObjectFileId = string.Empty;
            public bool CurrentUiEnabled = true;
            public bool CurrentUiInteractable = true;
            public string CurrentUiText = string.Empty;
            public float CurrentUiFontSize = 14f;
            public Color CurrentUiColor = Color.white;
            public string CurrentUiSprite = string.Empty;
            public int CurrentUiScaleMode = 1;
            public Vector2 CurrentUiReferenceResolution = new Vector2(800f, 600f);
            public int CurrentUiScreenMatchMode;
            public float CurrentUiMatchWidthOrHeight = 0.5f;
            public List<ParsedTransformRecord> Transforms = new List<ParsedTransformRecord>();
            public List<PrefabInstanceInfo> PrefabInstances = new List<PrefabInstanceInfo>();
            public PrefabInstanceInfo? CurrentPrefabInstance;
            public string CurrentModificationTarget = string.Empty;
            public string? CurrentModificationProperty;
            public string CurrentFatherFileId = string.Empty;
            public Vector3 CurrentLocalPosition = Vector3.zero;
            public Quaternion CurrentLocalRotation = Quaternion.identity;
            public Vector3 CurrentLocalScale = Vector3.one;
            public Color CurrentCameraBackground = Color.black;
            public float CurrentCameraFieldOfView = 60f;
            public float CurrentCameraNearClipPlane = 0.3f;
            public float CurrentCameraFarClipPlane = 1000f;
            public bool CurrentCameraOrthographic;
            public float CurrentCameraOrthographicSize = 5f;
        }
    }
}
