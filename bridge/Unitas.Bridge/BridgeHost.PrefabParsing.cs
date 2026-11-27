using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost
    {
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
            List<PrefabModification> composed = new List<PrefabModification>();
            for (int index = 0; index < instances.Count; index++)
            {
                PrefabInstanceInfo instance = instances[index];
                ulong parentEntity = 0;
                if (instance.TransformParentFileId.Length > 0
                    && transformGameObjects.TryGetValue(instance.TransformParentFileId, out string? parentGameObjectFileId)
                    && gameObjectEntities.TryGetValue(parentGameObjectFileId, out ulong parentLocalEntity))
                {
                    parentEntity = _nextSceneEntityBase + parentLocalEntity;
                }

                ExpandPrefabInstanceIntoScene(instance, guidIndex, parentEntity, composed, 0, ref nextEntity);
            }

            _nextSceneEntityBase = nextEntity;
        }

        private void ExpandPrefabInstanceIntoScene(
            PrefabInstanceInfo instance,
            PrefabGuidIndex guidIndex,
            ulong parentEntity,
            List<PrefabModification> inheritedComposed,
            int depth,
            ref ulong nextEntity)
        {
            if (depth >= MaxPrefabInstanceDepth)
            {
                return;
            }

            string? prefabPath = guidIndex.Resolve(instance.SourceGuid);
            if (prefabPath == null || !File.Exists(prefabPath))
            {
                return;
            }

            PrefabFileInfo file = ParsePrefabFile(prefabPath);
            List<PrefabModification> direct = new List<PrefabModification>();
            List<PrefabModification> nextComposed = new List<PrefabModification>(inheritedComposed);
            SplitPrefabModifications(instance, inheritedComposed, direct, nextComposed);
            Dictionary<string, ulong> localGoEntities = SeedOwnPrefabEntities(
                file,
                direct,
                parentEntity,
                ref nextEntity,
                out Dictionary<string, string> localTransformGameObjects);

            for (int index = 0; index < file.Instances.Count; index++)
            {
                PrefabInstanceInfo nested = file.Instances[index];
                ulong nestedParent = ResolvePrefabParentEntity(nested.TransformParentFileId, localTransformGameObjects, localGoEntities, parentEntity);
                ExpandPrefabInstanceIntoScene(nested, guidIndex, nestedParent, nextComposed, depth + 1, ref nextEntity);
            }

            nextEntity += (ulong)instance.AddedGameObjectFileIds.Count;
        }

        private static void SplitPrefabModifications(
            PrefabInstanceInfo instance,
            List<PrefabModification> inheritedComposed,
            List<PrefabModification> direct,
            List<PrefabModification> nextComposed)
        {
            string guid = instance.SourceGuid;
            for (int i = 0; i < instance.Modifications.Count; i++)
            {
                PrefabModification mod = instance.Modifications[i];
                if (string.IsNullOrEmpty(mod.TargetGuid) || string.Equals(mod.TargetGuid, guid, StringComparison.Ordinal))
                {
                    direct.Add(mod);
                }
                else
                {
                    nextComposed.Add(mod);
                }
            }

            for (int i = 0; i < inheritedComposed.Count; i++)
            {
                PrefabModification mod = inheritedComposed[i];
                if (string.Equals(mod.TargetGuid, guid, StringComparison.Ordinal))
                {
                    direct.Add(mod);
                }
            }
        }

        private Dictionary<string, ulong> SeedOwnPrefabEntities(
            PrefabFileInfo file,
            List<PrefabModification> direct,
            ulong parentEntity,
            ref ulong nextEntity,
            out Dictionary<string, string> localTransformGameObjects)
        {
            Dictionary<string, ulong> localGoEntities = new Dictionary<string, ulong>(StringComparer.Ordinal);
            localTransformGameObjects = new Dictionary<string, string>(StringComparer.Ordinal);
            for (int i = 0; i < file.GameObjectFileIds.Count; i++)
            {
                string goFileId = file.GameObjectFileIds[i];
                if (!file.IsOwnGameObject(goFileId))
                {
                    continue;
                }

                ulong entity = nextEntity++;
                localGoEntities[goFileId] = entity;
                string name = file.GameObjectNames.TryGetValue(goFileId, out string? parsedName) && !string.IsNullOrEmpty(parsedName)
                    ? parsedName
                    : "GameObject";
                ApplyPrefabNameModifications(ref name, direct, goFileId);
                _sceneObjectNames[entity] = name;
            }

            SeedOwnPrefabTransforms(file, direct, parentEntity, localGoEntities, localTransformGameObjects);
            SeedOwnPrefabPhysics(file, localGoEntities);
            return localGoEntities;
        }

        private void SeedOwnPrefabTransforms(
            PrefabFileInfo file,
            List<PrefabModification> direct,
            ulong parentEntity,
            Dictionary<string, ulong> localGoEntities,
            Dictionary<string, string> localTransformGameObjects)
        {
            for (int i = 0; i < file.Transforms.Count; i++)
            {
                ParsedTransformRecord tr = file.Transforms[i];
                if (!localGoEntities.ContainsKey(tr.GameObjectFileId))
                {
                    continue;
                }

                localTransformGameObjects[tr.TransformFileId] = tr.GameObjectFileId;
            }

            for (int i = 0; i < file.Transforms.Count; i++)
            {
                ParsedTransformRecord tr = file.Transforms[i];
                if (!localGoEntities.TryGetValue(tr.GameObjectFileId, out ulong entity))
                {
                    continue;
                }

                Vector3 position = tr.Position;
                Quaternion rotation = tr.Rotation;
                Vector3 scale = tr.Scale;
                ApplyPrefabModifications(ref position, ref rotation, ref scale, direct, tr.TransformFileId);
                _sceneTransforms[entity] = new SceneTransformMetadata(position, rotation, scale);
                ulong fatherEntity = ResolvePrefabParentEntity(tr.FatherFileId, localTransformGameObjects, localGoEntities, parentEntity);
                if (fatherEntity != 0)
                {
                    _sceneParentEntities[entity] = fatherEntity;
                }
            }
        }

        private static ulong ResolvePrefabParentEntity(
            string fatherTransformFileId,
            Dictionary<string, string> localTransformGameObjects,
            Dictionary<string, ulong> localGoEntities,
            ulong fallbackParentEntity)
        {
            if (!string.IsNullOrEmpty(fatherTransformFileId)
                && fatherTransformFileId != "0"
                && localTransformGameObjects.TryGetValue(fatherTransformFileId, out string? fatherGoId)
                && localGoEntities.TryGetValue(fatherGoId, out ulong fatherEntity))
            {
                return fatherEntity;
            }

            return fallbackParentEntity;
        }

        private static void ApplyPrefabNameModifications(
            ref string name,
            List<PrefabModification> modifications,
            string gameObjectFileId)
        {
            for (int i = 0; i < modifications.Count; i++)
            {
                PrefabModification mod = modifications[i];
                if (string.Equals(mod.TargetFileId, gameObjectFileId, StringComparison.Ordinal)
                    && string.Equals(mod.PropertyPath, "m_Name", StringComparison.Ordinal)
                    && !string.IsNullOrEmpty(mod.Value))
                {
                    name = mod.Value;
                }
            }
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
            SceneDocumentParseState state = new SceneDocumentParseState();
            for (int index = 0; index < lines.Length; index++)
            {
                string line = lines[index].Trim();
                if (line.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    FlushPrefabStateToInfo(info, state);
                    ParseDocumentHeader(line, out state.CurrentClassId, out state.CurrentFileId);
                    ResetPrefabParseState(state);
                    if (state.CurrentClassId == PREFAB_INSTANCE_CLASS)
                    {
                        state.CurrentPrefabInstance = new PrefabInstanceInfo { FileId = state.CurrentFileId };
                        info.Instances.Add(state.CurrentPrefabInstance);
                    }
                    continue;
                }

                ApplyPrefabLineToState(info, state, line);
            }

            FlushPrefabStateToInfo(info, state);
            foreach (PrefabInstanceInfo prefabInstance in info.Instances)
            {
                foreach (string addedFileId in prefabInstance.AddedGameObjectFileIds)
                {
                    info.AddedGameObjectFileIds.Add(addedFileId);
                }
            }

            return info;
        }

        private static void ResetPrefabParseState(SceneDocumentParseState state)
        {
            state.CurrentGameObjectFileId = string.Empty;
            state.CurrentName = string.Empty;
            state.CurrentFatherFileId = string.Empty;
            state.CurrentLocalPosition = Vector3.zero;
            state.CurrentLocalRotation = Quaternion.identity;
            state.CurrentLocalScale = Vector3.one;
            state.CurrentMass = 1f;
            state.CurrentUseGravity = true;
            state.CurrentIsKinematic = false;
            state.CurrentRadius = 0.5f;
            state.CurrentHeight = 2f;
            state.CurrentDirection = 1;
            state.CurrentIsTrigger = false;
            state.CurrentCenter = Vector3.zero;
            state.CurrentSize = Vector3.one;
            state.CurrentPrefabInstance = null;
            state.CurrentModificationTarget = string.Empty;
            state.CurrentModificationTargetGuid = string.Empty;
            state.CurrentModificationProperty = null;
        }

        private static void ApplyPrefabLineToState(PrefabFileInfo info, SceneDocumentParseState state, string line)
        {
            if (state.CurrentClassId == GAME_OBJECT_CLASS)
            {
                if (line.StartsWith("m_PrefabInstance:", StringComparison.Ordinal))
                {
                    string owner = ExtractFileId(line);
                    if (owner.Length > 0 && owner != "0" && state.CurrentFileId.Length > 0)
                    {
                        info.GameObjectPrefabInstanceFileIds[state.CurrentFileId] = owner;
                    }
                }
                else if (line.StartsWith("m_Name:", StringComparison.Ordinal))
                {
                    state.CurrentName = ParseStringValue(line);
                }
            }
            else if (state.CurrentClassId == TRANSFORM_CLASS)
            {
                ApplyPrefabTransformLine(state, line);
            }
            else if (state.CurrentClassId == 54)
            {
                ApplyPrefabRigidbodyLine(state, line);
            }
            else if (state.CurrentClassId == 65 || state.CurrentClassId == 135 || state.CurrentClassId == 136)
            {
                ApplyPrefabColliderLine(state, line);
            }
            else if (state.CurrentClassId == PREFAB_INSTANCE_CLASS && state.CurrentPrefabInstance != null)
            {
                ApplyPrefabInstanceLine(state, line);
            }
        }

        private static void ApplyPrefabTransformLine(SceneDocumentParseState state, string line)
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

        private static void ApplyPrefabRigidbodyLine(SceneDocumentParseState state, string line)
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

        private static void ApplyPrefabColliderLine(SceneDocumentParseState state, string line)
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
                state.CurrentSize = ParseVector3Value(line, Vector3.one);
            }
        }

        private static void ApplyPrefabInstanceLine(SceneDocumentParseState state, string line)
        {
            PrefabInstanceInfo instance = state.CurrentPrefabInstance!;
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
                state.CurrentModificationTargetGuid = ExtractGuidValue(line);
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
                    state.CurrentModificationTargetGuid,
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

        private static void FlushPrefabStateToInfo(PrefabFileInfo info, SceneDocumentParseState state)
        {
            if (state.CurrentClassId == GAME_OBJECT_CLASS && state.CurrentFileId.Length > 0)
            {
                info.GameObjectFileIds.Add(state.CurrentFileId);
                info.GameObjectNames[state.CurrentFileId] = state.CurrentName;
            }
            else if (state.CurrentClassId == TRANSFORM_CLASS && state.CurrentFileId.Length > 0)
            {
                info.Transforms.Add(new ParsedTransformRecord(
                    state.CurrentFileId,
                    state.CurrentGameObjectFileId,
                    state.CurrentFatherFileId,
                    state.CurrentLocalPosition,
                    state.CurrentLocalRotation,
                    state.CurrentLocalScale));
            }
            else
            {
                FlushPrefabPhysicsDocument(
                    info,
                    state.CurrentClassId,
                    state.CurrentGameObjectFileId,
                    state.CurrentMass,
                    state.CurrentUseGravity,
                    state.CurrentIsKinematic,
                    state.CurrentRadius,
                    state.CurrentHeight,
                    state.CurrentDirection,
                    state.CurrentIsTrigger,
                    state.CurrentCenter,
                    state.CurrentSize);
            }
        }

        private static void FlushPrefabPhysicsDocument(
            PrefabFileInfo info,
            int classId,
            string gameObjectFileId,
            float mass,
            bool useGravity,
            bool isKinematic,
            float radius,
            float height,
            int direction,
            bool isTrigger,
            Vector3 center,
            Vector3 size)
        {
            if (string.IsNullOrEmpty(gameObjectFileId))
            {
                return;
            }
            if (classId == 54)
            {
                info.Rigidbodies.Add(new ParsedRigidbodyRecord(gameObjectFileId, mass, useGravity, isKinematic));
            }
            else if (classId == 65)
            {
                info.BoxColliders.Add(new ParsedBoxColliderRecord(gameObjectFileId, size, center, isTrigger));
            }
            else if (classId == 135)
            {
                info.SphereColliders.Add(new ParsedSphereColliderRecord(gameObjectFileId, radius, center, isTrigger));
            }
            else if (classId == 136)
            {
                info.CapsuleColliders.Add(new ParsedCapsuleColliderRecord(gameObjectFileId, radius, height, direction, center, isTrigger));
            }
        }

        private void SeedOwnPrefabPhysics(PrefabFileInfo file, Dictionary<string, ulong> localGoEntities)
        {
            for (int i = 0; i < file.Rigidbodies.Count; i++)
            {
                ParsedRigidbodyRecord r = file.Rigidbodies[i];
                if (localGoEntities.TryGetValue(r.GameObjectFileId, out ulong entity))
                {
                    _sceneRigidbodies[entity] = new SceneRigidbodyMetadata(r.Mass, r.UseGravity, r.IsKinematic);
                }
            }
            for (int i = 0; i < file.SphereColliders.Count; i++)
            {
                ParsedSphereColliderRecord r = file.SphereColliders[i];
                if (localGoEntities.TryGetValue(r.GameObjectFileId, out ulong entity))
                {
                    _sceneSphereColliders[entity] = new SceneSphereColliderMetadata(r.Radius, r.Center, r.IsTrigger);
                }
            }
            for (int i = 0; i < file.BoxColliders.Count; i++)
            {
                ParsedBoxColliderRecord r = file.BoxColliders[i];
                if (localGoEntities.TryGetValue(r.GameObjectFileId, out ulong entity))
                {
                    _sceneBoxColliders[entity] = new SceneBoxColliderMetadata(r.Size, r.Center, r.IsTrigger);
                }
            }
            for (int i = 0; i < file.CapsuleColliders.Count; i++)
            {
                ParsedCapsuleColliderRecord r = file.CapsuleColliders[i];
                if (localGoEntities.TryGetValue(r.GameObjectFileId, out ulong entity))
                {
                    _sceneCapsuleColliders[entity] = new SceneCapsuleColliderMetadata(r.Radius, r.Height, r.Direction, r.Center, r.IsTrigger);
                }
            }
        }
    }
}
