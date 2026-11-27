using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost
    {
        public void EnsureLoadedSceneJointsMaterialized()
        {
            if (_loadedScenes.Count == 0)
            {
                return;
            }
            string scenePath = _loadedScenes[_loadedScenes.Count - 1].path;
            if (string.IsNullOrEmpty(scenePath) || string.Equals(scenePath, _lastJointScenePath, StringComparison.Ordinal) || !File.Exists(scenePath))
            {
                return;
            }
            _lastJointScenePath = scenePath;
            ParseAndMaterializeSceneJoints(scenePath);
        }

        private void ParseAndMaterializeSceneJoints(string scenePath)
        {
            string[] lines = File.ReadAllLines(scenePath);
            Dictionary<string, ulong> goEntities = new Dictionary<string, ulong>(StringComparer.Ordinal);
            Dictionary<string, string> compToGo = new Dictionary<string, string>(StringComparer.Ordinal);
            List<ParsedJointRecord> joints = new List<ParsedJointRecord>();
            ScanSceneJointDocuments(lines, goEntities, compToGo, joints);
            ulong baseEntity = _nextSceneEntityBase > (ulong)goEntities.Count
                ? _nextSceneEntityBase - (ulong)goEntities.Count
                : 1;
            List<string> keys = new List<string>(goEntities.Keys);
            for (int i = 0; i < keys.Count; i++)
            {
                goEntities[keys[i]] = baseEntity + goEntities[keys[i]];
            }
            for (int i = 0; i < joints.Count; i++)
            {
                MaterializeParsedJoint(joints[i], goEntities, compToGo);
            }
        }

        private void ScanSceneJointDocuments(
            string[] lines,
            Dictionary<string, ulong> goEntities,
            Dictionary<string, string> compToGo,
            List<ParsedJointRecord> joints)
        {
            ulong localIdx = 0;
            int classId = 0;
            string fileId = string.Empty;
            string section = string.Empty;
            ParsedJointRecord? currentJoint = null;
            for (int i = 0; i < lines.Length; i++)
            {
                string raw = lines[i];
                string line = raw.Trim();
                if (line.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    if (currentJoint != null && !string.IsNullOrEmpty(currentJoint.GameObjectFileId))
                    {
                        joints.Add(currentJoint);
                    }
                    ParseDocumentHeader(line, out classId, out fileId);
                    section = string.Empty;
                    if (classId == 1 && !string.IsNullOrEmpty(fileId))
                    {
                        goEntities[fileId] = localIdx;
                        compToGo[fileId] = fileId;
                        localIdx++;
                    }
                    currentJoint = IsSceneJointClass(classId) ? new ParsedJointRecord(classId) : null;
                    continue;
                }
                if ((classId == 54 || classId == 50) && line.StartsWith("m_GameObject:", StringComparison.Ordinal))
                {
                    compToGo[fileId] = ExtractFileId(line);
                }
                else if (currentJoint != null)
                {
                    ApplyParsedJointLine(currentJoint, line, raw, ref section);
                }
            }
            if (currentJoint != null && !string.IsNullOrEmpty(currentJoint.GameObjectFileId))
            {
                joints.Add(currentJoint);
            }
        }

        private static bool IsSceneJointClass(int classId)
        {
            return classId == 59 || classId == 138 || classId == 144 || classId == 145 || classId == 153
                || classId == 231 || classId == 232 || classId == 233 || classId == 234 || classId == 235
                || classId == 253 || classId == 255 || classId == 256 || classId == 257;
        }

        private static void ApplyParsedJointLine(ParsedJointRecord rec, string line, string raw, ref string section)
        {
            if (!raw.StartsWith("    ", StringComparison.Ordinal))
            {
                if (line.EndsWith(':'))
                {
                    section = line.TrimEnd(':');
                    return;
                }
                section = string.Empty;
            }
            if (section.Length > 0)
            {
                ApplyParsedJointNestedLine(rec, section, line);
                return;
            }
            ApplyParsedJointTopLine(rec, line);
        }

        private static void ApplyParsedJointTopLine(ParsedJointRecord rec, string line)
        {
            if (line.StartsWith("m_GameObject:", StringComparison.Ordinal))
            {
                rec.GameObjectFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_ConnectedBody:", StringComparison.Ordinal) || line.StartsWith("m_ConnectedRigidBody:", StringComparison.Ordinal))
            {
                rec.ConnectedBodyFileId = ExtractFileId(line);
            }
            else if (line.StartsWith("m_Anchor:", StringComparison.Ordinal))
            {
                rec.Anchor = ParseVector3Value(line, Vector3.zero);
            }
            else if (line.StartsWith("m_ConnectedAnchor:", StringComparison.Ordinal))
            {
                rec.ConnectedAnchor = ParseVector3Value(line, Vector3.zero);
            }
            else if (line.StartsWith("m_Axis:", StringComparison.Ordinal))
            {
                rec.Axis = ParseVector3Value(line, Vector3.right);
            }
            else
            {
                ApplyParsedJointScalarLine(rec, line);
            }
        }

        private static void ApplyParsedJointScalarLine(ParsedJointRecord rec, string line)
        {
            if (line.StartsWith("m_UseLimits:", StringComparison.Ordinal))
            {
                rec.UseLimits = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_UseMotor:", StringComparison.Ordinal))
            {
                rec.UseMotor = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_UseSpring:", StringComparison.Ordinal))
            {
                rec.UseSpring = ParseBoolValue(line, false);
            }
            else if (line.StartsWith("m_Spring:", StringComparison.Ordinal) && !line.EndsWith(':'))
            {
                rec.Spring = ParseFloatValue(line, 10f);
            }
            else if (line.StartsWith("m_Damper:", StringComparison.Ordinal))
            {
                rec.Damper = ParseFloatValue(line, 0.2f);
            }
            else if (line.StartsWith("m_Distance:", StringComparison.Ordinal))
            {
                rec.Distance = ParseFloatValue(line, 1f);
            }
            else if (line.StartsWith("m_BreakForce:", StringComparison.Ordinal))
            {
                rec.BreakForce = ParseFloatValue(line, float.PositiveInfinity);
            }
            else if (line.StartsWith("m_BreakTorque:", StringComparison.Ordinal))
            {
                rec.BreakTorque = ParseFloatValue(line, float.PositiveInfinity);
            }
        }

        private static void ApplyParsedJointNestedLine(ParsedJointRecord rec, string section, string line)
        {
            if (section == "m_Limits" || section == "m_AngleLimits" || section == "m_TranslationLimits")
            {
                if (line.StartsWith("min:", StringComparison.Ordinal) || line.StartsWith("m_LowerAngle:", StringComparison.Ordinal) || line.StartsWith("m_LowerTranslation:", StringComparison.Ordinal))
                {
                    rec.LimitMin = ParseFloatValue(line, 0f);
                }
                else if (line.StartsWith("max:", StringComparison.Ordinal) || line.StartsWith("m_UpperAngle:", StringComparison.Ordinal) || line.StartsWith("m_UpperTranslation:", StringComparison.Ordinal))
                {
                    rec.LimitMax = ParseFloatValue(line, 0f);
                }
            }
            else if (section == "m_Motor")
            {
                if (line.StartsWith("targetVelocity:", StringComparison.Ordinal) || line.StartsWith("m_MotorSpeed:", StringComparison.Ordinal))
                {
                    rec.MotorSpeed = ParseFloatValue(line, 0f);
                }
                else if (line.StartsWith("force:", StringComparison.Ordinal) || line.StartsWith("m_MaximumMotorForce:", StringComparison.Ordinal))
                {
                    rec.MotorForce = ParseFloatValue(line, 0f);
                }
                else if (line.StartsWith("freeSpin:", StringComparison.Ordinal))
                {
                    rec.MotorFreeSpin = ParseBoolValue(line, false);
                }
            }
            else if (section == "m_Spring")
            {
                if (line.StartsWith("spring:", StringComparison.Ordinal))
                {
                    rec.Spring = ParseFloatValue(line, 0f);
                }
                else if (line.StartsWith("damper:", StringComparison.Ordinal))
                {
                    rec.Damper = ParseFloatValue(line, 0f);
                }
                else if (line.StartsWith("targetPosition:", StringComparison.Ordinal))
                {
                    rec.TargetPosition = ParseFloatValue(line, 0f);
                }
            }
        }

        private void MaterializeParsedJoint(
            ParsedJointRecord rec,
            Dictionary<string, ulong> goEntities,
            Dictionary<string, string> compToGo)
        {
            if (!goEntities.TryGetValue(rec.GameObjectFileId, out ulong ownerEntity))
            {
                return;
            }
            GameObject? owner = ResolveGameObjectByEntity(ownerEntity);
            if (owner == null)
            {
                return;
            }
            GameObject? connectedGo = null;
            if (!string.IsNullOrEmpty(rec.ConnectedBodyFileId)
                && compToGo.TryGetValue(rec.ConnectedBodyFileId, out string? connGoId)
                && !string.IsNullOrEmpty(connGoId)
                && goEntities.TryGetValue(connGoId, out ulong connEntity))
            {
                connectedGo = ResolveGameObjectByEntity(connEntity);
            }
            AttachMaterializedJointComponent(rec, owner, connectedGo);
        }

        private void AttachMaterializedJointComponent(ParsedJointRecord rec, GameObject owner, GameObject? connectedGo)
        {
            Rigidbody? connRb = connectedGo != null ? (connectedGo.GetComponent<Rigidbody>() ?? connectedGo.AddComponent<Rigidbody>()) : null;
            if (rec.ClassId == 59)
            {
                HingeJoint hj = owner.GetComponent<HingeJoint>() ?? owner.AddComponent<HingeJoint>();
                hj.connectedBody = connRb;
                hj.anchor = rec.Anchor;
                hj.connectedAnchor = rec.ConnectedAnchor;
                hj.axis = rec.Axis;
                hj.useLimits = rec.UseLimits;
                hj.limits = new JointLimits { min = rec.LimitMin, max = rec.LimitMax };
                hj.useMotor = rec.UseMotor;
                hj.motor = new JointMotor { targetVelocity = rec.MotorSpeed, force = rec.MotorForce, freeSpin = rec.MotorFreeSpin };
                hj.useSpring = rec.UseSpring;
                hj.spring = new JointSpring { spring = rec.Spring, damper = rec.Damper, targetPosition = rec.TargetPosition };
                hj.breakForce = rec.BreakForce;
                hj.breakTorque = rec.BreakTorque;
                TrySyncJoint3D(hj);
            }
            else if (rec.ClassId == 138)
            {
                FixedJoint fj = owner.GetComponent<FixedJoint>() ?? owner.AddComponent<FixedJoint>();
                fj.connectedBody = connRb;
                fj.anchor = rec.Anchor;
                fj.connectedAnchor = rec.ConnectedAnchor;
                fj.breakForce = rec.BreakForce;
                fj.breakTorque = rec.BreakTorque;
                TrySyncJoint3D(fj);
            }
            else if (rec.ClassId == 145)
            {
                SpringJoint sj = owner.GetComponent<SpringJoint>() ?? owner.AddComponent<SpringJoint>();
                sj.connectedBody = connRb;
                sj.anchor = rec.Anchor;
                sj.connectedAnchor = rec.ConnectedAnchor;
                sj.spring = rec.Spring;
                sj.damper = rec.Damper;
                TrySyncJoint3D(sj);
            }
            else if (rec.ClassId == 232)
            {
                Rigidbody2D? connRb2D = connectedGo != null ? (connectedGo.GetComponent<Rigidbody2D>() ?? connectedGo.AddComponent<Rigidbody2D>()) : null;
                DistanceJoint2D dj = owner.GetComponent<DistanceJoint2D>() ?? owner.AddComponent<DistanceJoint2D>();
                dj.connectedBody = connRb2D;
                dj.anchor = new Vector2(rec.Anchor.x, rec.Anchor.y);
                dj.connectedAnchor = new Vector2(rec.ConnectedAnchor.x, rec.ConnectedAnchor.y);
                dj.distance = rec.Distance;
                TrySyncJoint2D(dj);
            }
        }

        private sealed class ParsedJointRecord
        {
            public ParsedJointRecord(int classId)
            {
                ClassId = classId;
            }

            public int ClassId { get; }
            public string GameObjectFileId = string.Empty;
            public string ConnectedBodyFileId = string.Empty;
            public Vector3 Anchor = Vector3.zero;
            public Vector3 ConnectedAnchor = Vector3.zero;
            public Vector3 Axis = Vector3.right;
            public bool UseLimits;
            public float LimitMin;
            public float LimitMax;
            public bool UseMotor;
            public float MotorSpeed;
            public float MotorForce;
            public bool MotorFreeSpin;
            public bool UseSpring;
            public float Spring = 10f;
            public float Damper = 0.2f;
            public float TargetPosition;
            public float Distance = 1f;
            public float BreakForce = float.PositiveInfinity;
            public float BreakTorque = float.PositiveInfinity;
        }
    }
}
