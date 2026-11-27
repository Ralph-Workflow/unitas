using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using UnityEngine;

namespace Unitas.Bridge
{
    public interface IBridgeJointSceneHost
    {
        void EnsureLoadedSceneJointsMaterialized();
        void SyncJointTransformsAfterStep();
        bool TryGetBodyRotation3D(ulong entity, out Quaternion rotation);
        bool TryGetBodyPose2D(ulong entity, out Vector2 position, out float rotationRadians);
    }

    public sealed partial class BridgeHost : IPhysicsJointProvider, IPhysicsJoint2DProvider, IBridgeJointSceneHost
    {
        private readonly HashSet<ulong> _jointEntities = new HashSet<ulong>();
        private string _lastJointScenePath = string.Empty;
        private bool _jointSceneHookInstalled;

        private void EnsureJointSceneHookInstalled()
        {
            if (_jointSceneHookInstalled)
            {
                return;
            }
            _jointSceneHookInstalled = true;
            UnityEngine.SceneManagement.SceneManager.sceneLoaded += OnJointSceneLoaded;
        }

        private void OnJointSceneLoaded(UnityEngine.SceneManagement.Scene scene, UnityEngine.SceneManagement.LoadSceneMode mode)
        {
            if (_isDisposed || string.IsNullOrEmpty(scene.path) || !File.Exists(scene.path))
            {
                return;
            }
            _lastJointScenePath = scene.path;
            ParseAndMaterializeSceneJoints(scene.path);
        }

        public bool TrySyncJoint3D(Joint joint)
        {
            if (_isDisposed || _runtimeHandle == 0 || joint == null)
            {
                return false;
            }
            EnsureJointSceneHookInstalled();
            EnsureLoadedSceneJointsMaterialized();
            GameObject? owner = joint.OwnerOrNull;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return false;
            }
            ulong entity = owner.NativeEntityId;
            _jointEntities.Add(entity);
            EnsureNativeBody3D(owner);
            ulong connectedEntity = 0;
            Rigidbody? connectedBody = joint.connectedBody;
            if (connectedBody != null && connectedBody.OwnerOrNull != null && !connectedBody.OwnerOrNull.IsDestroyed)
            {
                connectedEntity = connectedBody.OwnerOrNull.NativeEntityId;
                _jointEntities.Add(connectedEntity);
                EnsureNativeBody3D(connectedBody.OwnerOrNull);
            }
            double[] values = PackJoint3DParameters(joint, connectedEntity);
            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)(values.Length * 8)).ToInt64();
            if (buffer == 0)
            {
                return false;
            }
            Marshal.Copy(values, 0, new IntPtr((long)buffer), values.Length);
            return RuntimeInterop.unitas_runtime_joint3d_set(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryRemoveJoint3D(ulong entity, uint kind)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_joint3d_remove(_runtimeHandle, entity, kind) == 0;
        }

        public bool TrySyncJoint2D(Joint2D joint)
        {
            if (_isDisposed || _runtimeHandle == 0 || joint == null)
            {
                return false;
            }
            EnsureJointSceneHookInstalled();
            EnsureLoadedSceneJointsMaterialized();
            GameObject? owner = joint.OwnerOrNull;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return false;
            }
            ulong entity = owner.NativeEntityId;
            _jointEntities.Add(entity);
            EnsureNativeBody2D(owner);
            ulong connectedEntity = 0;
            Rigidbody2D? connectedBody = joint.connectedBody;
            if (connectedBody != null && connectedBody.OwnerOrNull != null && !connectedBody.OwnerOrNull.IsDestroyed)
            {
                connectedEntity = connectedBody.OwnerOrNull.NativeEntityId;
                _jointEntities.Add(connectedEntity);
                EnsureNativeBody2D(connectedBody.OwnerOrNull);
            }
            if (!joint.enabled)
            {
                return TryRemoveJoint2D(entity, joint.NativeJointKind);
            }
            double[] values = PackJoint2DParameters(joint, connectedEntity);
            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)(values.Length * 8)).ToInt64();
            if (buffer == 0)
            {
                return false;
            }
            Marshal.Copy(values, 0, new IntPtr((long)buffer), values.Length);
            return RuntimeInterop.unitas_runtime_joint2d_set(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryRemoveJoint2D(ulong entity, uint kind)
        {
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_joint2d_remove(_runtimeHandle, entity, kind) == 0;
        }

        public bool TryGetBodyRotation3D(ulong entity, out Quaternion rotation)
        {
            rotation = Quaternion.identity;
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32).ToInt64();
            if (buffer == 0 || RuntimeInterop.unitas_runtime_body_rotation(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            double[] vals = new double[4];
            Marshal.Copy(new IntPtr((long)buffer), vals, 0, 4);
            rotation = new Quaternion((float)vals[0], (float)vals[1], (float)vals[2], (float)vals[3]);
            return true;
        }

        public bool TryGetBodyPose2D(ulong entity, out Vector2 position, out float rotationRadians)
        {
            position = Vector2.zero;
            rotationRadians = 0f;
            if (_isDisposed || _runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            ulong buffer = (ulong)RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 12).ToInt64();
            if (buffer == 0 || RuntimeInterop.unitas_runtime_body2d_position(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            float[] vals = new float[3];
            Marshal.Copy(new IntPtr((long)buffer), vals, 0, 3);
            position = new Vector2(vals[0], vals[1]);
            rotationRadians = vals[2];
            return true;
        }

        private void EnsureNativeBody3D(GameObject go)
        {
            ulong entity = go.NativeEntityId;
            if (entity == 0)
            {
                return;
            }
            Transform t = go.transform;
            if (go.IsRuntimeObject)
            {
                SetRuntimeTransform(entity, t.position, t.rotation, t.lossyScale);
            }
            TrySetBodyPosition(entity, t.position);
            TrySetBodyRotation(entity, t.rotation);
        }

        private void EnsureNativeBody2D(GameObject go)
        {
            ulong entity = go.NativeEntityId;
            if (entity == 0 || _registeredBodies2D.Contains(entity))
            {
                return;
            }
            if (TryGetBodyPose2D(entity, out _, out _))
            {
                _registeredBodies2D.Add(entity);
                return;
            }
            _registeredBodies2D.Add(entity);
            Rigidbody2D? rb = go.GetComponent<Rigidbody2D>();
            RigidbodyType2D bodyType = rb != null ? rb.bodyType : RigidbodyType2D.Static;
            float mass = rb != null ? rb.mass : 1f;
            float gravityScale = rb != null ? rb.gravityScale : 1f;
            uint simulated = rb == null || rb.simulated ? 1u : 0u;
            uint interp = rb != null ? (uint)rb.interpolation : 0u;
            uint ccd = rb != null ? (uint)rb.collisionDetectionMode : 0u;
            Vector3 pos = go.transform.position;
            float rotRad = go.transform.eulerAngles.z * Mathf.Deg2Rad;
            RuntimeInterop.unitas_runtime_body2d_create(
                _runtimeHandle,
                entity,
                pos.x,
                pos.y,
                rotRad,
                mass,
                gravityScale,
                (uint)bodyType,
                (uint)go.layer,
                simulated,
                interp,
                ccd);
            if (rb != null && rb.constraints != RigidbodyConstraints2D.None)
            {
                TrySetBodyConstraints2D(entity, rb.constraints);
            }
        }

        public void SyncJointTransformsAfterStep()
        {
            EnsureLoadedSceneJointsMaterialized();
            foreach (ulong entity in _jointEntities)
            {
                if (!_entityGameObjects.TryGetValue(entity, out GameObject? go) || go == null || go.IsDestroyed)
                {
                    continue;
                }
                if (TryGetBodyPosition(entity, out Vector3 pos3D))
                {
                    go.transform.SeedPositionFromNative(pos3D);
                    if (TryGetBodyRotation3D(entity, out Quaternion rot3D))
                    {
                        go.transform.localRotation = rot3D;
                    }
                }
                else if (TryGetBodyPose2D(entity, out Vector2 pos2D, out float rotRad2D))
                {
                    Vector3 cur = go.transform.position;
                    go.transform.SeedPositionFromNative(new Vector3(pos2D.x, pos2D.y, cur.z));
                    go.transform.localRotation = Quaternion.Euler(0f, 0f, rotRad2D * Mathf.Rad2Deg);
                }
            }
        }

        private static double ToRadiansIfDegrees(float angle)
        {
            if (Mathf.Abs(angle) > Mathf.PI * 2f)
            {
                return angle * Mathf.Deg2Rad;
            }
            return angle;
        }

        private static double[] PackJoint3DParameters(Joint joint, ulong connectedEntity)
        {
            double[] v = new double[42];
            v[0] = joint.NativeJointKind;
            v[1] = connectedEntity;
            v[2] = joint.anchor.x;
            v[3] = joint.anchor.y;
            v[4] = joint.anchor.z;
            v[5] = joint.connectedAnchor.x;
            v[6] = joint.connectedAnchor.y;
            v[7] = joint.connectedAnchor.z;
            v[8] = joint.axis.x;
            v[9] = joint.axis.y;
            v[10] = joint.axis.z;
            v[40] = float.IsInfinity(joint.breakForce) ? 0.0 : joint.breakForce;
            v[41] = float.IsInfinity(joint.breakTorque) ? 0.0 : joint.breakTorque;
            if (joint is HingeJoint hj)
            {
                PackHingeJoint3D(hj, v);
            }
            else if (joint is SpringJoint sj)
            {
                v[15] = sj.minDistance;
                v[16] = sj.maxDistance;
                v[18] = sj.spring;
                v[19] = sj.damper;
                v[20] = sj.tolerance;
            }
            else if (joint is CharacterJoint cj)
            {
                PackCharacterJoint3D(cj, v);
            }
            else if (joint is ConfigurableJoint cfg)
            {
                PackConfigurableJoint3D(cfg, v);
            }
            return v;
        }

        private static void PackHingeJoint3D(HingeJoint hj, double[] v)
        {
            v[14] = hj.useLimits ? 1.0 : 0.0;
            v[15] = ToRadiansIfDegrees(hj.limits.min);
            v[16] = ToRadiansIfDegrees(hj.limits.max);
            v[17] = hj.useSpring ? 1.0 : 0.0;
            v[18] = hj.spring.spring;
            v[19] = hj.spring.damper;
            v[20] = ToRadiansIfDegrees(hj.spring.targetPosition);
            v[21] = hj.useMotor ? 1.0 : 0.0;
            v[22] = ToRadiansIfDegrees(hj.motor.targetVelocity);
            v[23] = hj.motor.force;
            v[24] = hj.motor.freeSpin ? 1.0 : 0.0;
        }

        private static void PackCharacterJoint3D(CharacterJoint cj, double[] v)
        {
            v[11] = cj.swingAxis.x;
            v[12] = cj.swingAxis.y;
            v[13] = cj.swingAxis.z;
            v[15] = ToRadiansIfDegrees(cj.lowTwistLimit.limit);
            v[16] = ToRadiansIfDegrees(cj.highTwistLimit.limit);
            v[18] = ToRadiansIfDegrees(cj.swing1Limit.limit);
            v[19] = ToRadiansIfDegrees(cj.swing2Limit.limit);
        }

        private static void PackConfigurableJoint3D(ConfigurableJoint cfg, double[] v)
        {
            v[11] = cfg.secondaryAxis.x;
            v[12] = cfg.secondaryAxis.y;
            v[13] = cfg.secondaryAxis.z;
            v[14] = (int)cfg.xMotion;
            v[15] = (int)cfg.yMotion;
            v[16] = (int)cfg.zMotion;
            v[17] = (int)cfg.angularXMotion;
            v[18] = (int)cfg.angularYMotion;
            v[19] = (int)cfg.angularZMotion;
            v[20] = cfg.linearLimit.limit;
            v[21] = ToRadiansIfDegrees(cfg.lowAngularXLimit.limit);
            v[22] = ToRadiansIfDegrees(cfg.highAngularXLimit.limit);
            v[23] = ToRadiansIfDegrees(cfg.angularYLimit.limit);
            v[24] = ToRadiansIfDegrees(cfg.angularZLimit.limit);
            v[25] = cfg.xDrive.positionSpring;
            v[26] = cfg.xDrive.positionDamper;
            v[27] = cfg.xDrive.maximumForce;
            v[28] = cfg.yDrive.positionSpring;
            v[29] = cfg.yDrive.positionDamper;
            v[30] = cfg.yDrive.maximumForce;
            v[31] = cfg.zDrive.positionSpring;
            v[32] = cfg.zDrive.positionDamper;
            v[33] = cfg.zDrive.maximumForce;
            v[34] = cfg.targetPosition.x;
            v[35] = cfg.targetPosition.y;
            v[36] = cfg.targetPosition.z;
            v[37] = cfg.targetVelocity.x;
            v[38] = cfg.targetVelocity.y;
            v[39] = cfg.targetVelocity.z;
        }

        private static double[] PackJoint2DParameters(Joint2D joint, ulong connectedEntity)
        {
            double[] v = new double[24];
            v[0] = joint.NativeJointKind;
            v[1] = connectedEntity;
            if (joint is AnchoredJoint2D anchored)
            {
                v[2] = anchored.anchor.x;
                v[3] = anchored.anchor.y;
                v[4] = anchored.connectedAnchor.x;
                v[5] = anchored.connectedAnchor.y;
            }
            v[22] = float.IsInfinity(joint.breakForce) ? 0.0 : joint.breakForce;
            v[23] = float.IsInfinity(joint.breakTorque) ? 0.0 : joint.breakTorque;
            PackSpecificJoint2D(joint, v);
            return v;
        }

        private static void PackSpecificJoint2D(Joint2D joint, double[] v)
        {
            switch (joint)
            {
                case DistanceJoint2D dj:
                    v[6] = dj.distance;
                    v[7] = dj.maxDistanceOnly ? 1.0 : 0.0;
                    break;
                case FixedJoint2D fj:
                    v[8] = fj.dampingRatio;
                    v[9] = fj.frequency;
                    v[10] = ToRadiansIfDegrees(fj.referenceAngle);
                    break;
                case FrictionJoint2D frj:
                    v[11] = frj.maxForce;
                    v[12] = frj.maxTorque;
                    break;
                case HingeJoint2D hj:
                    v[15] = hj.useMotor ? 1.0 : 0.0;
                    v[16] = hj.motor.motorSpeed;
                    v[17] = hj.motor.maxMotorTorque;
                    v[18] = hj.useLimits ? 1.0 : 0.0;
                    v[19] = ToRadiansIfDegrees(hj.limits.min);
                    v[20] = ToRadiansIfDegrees(hj.limits.max);
                    break;
                default:
                    PackExtendedJoint2D(joint, v);
                    break;
            }
        }

        private static void PackExtendedJoint2D(Joint2D joint, double[] v)
        {
            switch (joint)
            {
                case RelativeJoint2D rj:
                    v[6] = rj.linearOffset.x;
                    v[7] = rj.linearOffset.y;
                    v[10] = ToRadiansIfDegrees(rj.angularOffset);
                    v[11] = rj.maxForce;
                    v[12] = rj.maxTorque;
                    v[13] = rj.correctionScale;
                    break;
                case SliderJoint2D slj:
                    v[14] = slj.angle;
                    v[15] = slj.useMotor ? 1.0 : 0.0;
                    v[16] = slj.motor.motorSpeed;
                    v[17] = slj.motor.maxMotorTorque;
                    v[18] = slj.useLimits ? 1.0 : 0.0;
                    v[19] = slj.limits.min;
                    v[20] = slj.limits.max;
                    break;
                case SpringJoint2D sj:
                    v[6] = sj.distance;
                    v[8] = sj.dampingRatio;
                    v[9] = sj.frequency;
                    break;
                case TargetJoint2D tj:
                    v[2] = tj.anchor.x;
                    v[3] = tj.anchor.y;
                    v[6] = tj.target.x;
                    v[7] = tj.target.y;
                    v[8] = tj.dampingRatio;
                    v[9] = tj.frequency;
                    v[11] = tj.maxForce;
                    break;
                case WheelJoint2D wj:
                    v[8] = wj.suspension.dampingRatio;
                    v[9] = wj.suspension.frequency;
                    v[14] = wj.suspension.angle;
                    v[15] = wj.useMotor ? 1.0 : 0.0;
                    v[16] = wj.motor.motorSpeed;
                    v[17] = wj.motor.maxMotorTorque;
                    break;
            }
        }
    }
}
