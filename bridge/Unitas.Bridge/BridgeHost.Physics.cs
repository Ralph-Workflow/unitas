using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IDisposable, IPhysicsProvider, IPhysics2DProvider, ICharacterControllerProvider, IInputProvider, ISceneHost, IApplicationHost, IAudioProvider, IRuntimeObjectProvider
    {
        public Collider? GetCollider(ulong entity)
        {
            return ResolveColliderByEntity(entity);
        }

        public bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_raycast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask, QueryTriggerInteraction queryTriggerInteraction)
        {
            hitInfo = default;
            Vector3 delta = end - start;
            float maxDistance = delta.magnitude;
            Vector3 direction = maxDistance > 0f ? delta / maxDistance : Vector3.zero;
            int count = RuntimeInterop.unitas_runtime_linecast(
                _runtimeHandle,
                start.x,
                start.y,
                start.z,
                direction.x,
                direction.y,
                direction.z,
                maxDistance,
                (uint)layerMask,
                queryTriggerInteraction.IncludesTriggers() ? 1 : 0);
            return TakeNearestHit(count, out hitInfo);
        }

        public bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_sphere_cast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                radius,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_box_cast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                halfExtents.x,
                halfExtents.y,
                halfExtents.z,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_sphere(
                _runtimeHandle,
                position.x,
                position.y,
                position.z,
                radius,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            if (count <= 0)
            {
                return Array.Empty<Collider>();
            }
            QueryHit[] hits = ReadQueryHits(count);
            Collider[] colliders = new Collider[hits.Length];
            for (int index = 0; index < hits.Length; index++)
            {
                colliders[index] = GetCollider(hits[index].Entity)!;
            }
            return colliders;
        }

        public bool TryGetBodyPosition(ulong entity, out Vector3 position)
        {
            return TryReadBodyVector(entity, RuntimeInterop.unitas_runtime_body_position, out position);
        }

        public bool TryGetBodyVelocity(ulong entity, out Vector3 velocity)
        {
            return TryReadBodyVector(entity, RuntimeInterop.unitas_runtime_body_velocity, out velocity);
        }

        public bool TrySetBodyPosition(ulong entity, Vector3 position)
        {
            return TryWriteBodyVector(entity, RuntimeInterop.unitas_runtime_set_body_position, position)
                || TryWriteBodyVector(entity, RuntimeInterop.unitas_runtime_teleport_body_position, position);
        }

        public bool TrySetBodyVelocity(ulong entity, Vector3 velocity)
        {
            return TryWriteBodyVector(entity, RuntimeInterop.unitas_runtime_set_body_velocity, velocity);
        }

        public bool TrySetBodyRotation(ulong entity, Quaternion rotation)
        {
            return TryWriteBodyQuaternion(entity, RuntimeInterop.unitas_runtime_set_body_rotation, rotation);
        }

        public bool TrySetBodyCollisionDetectionMode(ulong entity, CollisionDetectionMode mode)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            int nativeMode = mode == CollisionDetectionMode.Discrete ? 0 : 1;
            return RuntimeInterop.unitas_runtime_body_set_collision_detection_mode(_runtimeHandle, entity, nativeMode) == 0;
        }

        public bool TryAddForce(ulong entity, Vector3 force, ForceMode mode)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[32];
            BitConverter.GetBytes((double)force.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)force.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)force.z).CopyTo(bytes, 16);
            BitConverter.GetBytes((double)mode).CopyTo(bytes, 24);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return RuntimeInterop.unitas_runtime_add_body_force(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryGetCollisionSummary(ulong a, ulong b, out CollisionSummary summary)
        {
            summary = default;
            if (_isDisposed || a == 0 || b == 0)
            {
                return false;
            }
            if (!TryReadCollisionSummary(a, b, out RuntimeInterop.AbiCollisionSummary native))
            {
                return false;
            }
            summary = new CollisionSummary
            {
                impulse = native.Impulse,
                relativeVelocity = native.RelativeVelocity,
                contactCount = (int)native.ContactCount,
                point = native.Point,
                normal = native.Normal,
            };
            return true;
        }

        public bool TryGetCollisionContact(ulong a, ulong b, uint index, out ContactPoint contact)
        {
            contact = default;
            if (_isDisposed || a == 0 || b == 0)
            {
                return false;
            }
            if (!TryReadCollisionContact(a, b, index, out RuntimeInterop.AbiCollisionContactPoint native))
            {
                return false;
            }
            contact = new ContactPoint
            {
                point = new Vector3((float)native.PointX, (float)native.PointY, (float)native.PointZ),
                normal = new Vector3((float)native.NormalX, (float)native.NormalY, (float)native.NormalZ),
                impulse = native.Impulse,
            };
            return true;
        }

        public bool TryMoveBodyPosition(ulong entity, Vector3 position)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_move_body_position(entity, position.x, position.y, position.z) == 0;
        }

        public bool Raycast2D(Vector2 origin, Vector2 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit2D hit)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_raycast2d(
                _runtimeHandle,
                origin.x,
                origin.y,
                direction.x,
                direction.y,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            if (count <= 0)
            {
                return false;
            }
            QueryHit2D[] hits = ReadQueryHits2D(count);
            if (hits.Length == 0)
            {
                return false;
            }
            QueryHit2D nearest = hits[0];
            hit.collider = ResolveCollider2DByEntity(nearest.Entity);
            hit.point = new Vector2(nearest.X, nearest.Y);
            hit.normal = new Vector2(nearest.NX, nearest.NY);
            hit.distance = nearest.Distance;
            hit.fraction = maxDistance > 0f ? nearest.Distance / maxDistance : 0f;
            return true;
        }

        public Collider2D[] OverlapPoint2D(Vector2 position, int layerMask)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_point2d(
                _runtimeHandle,
                position.x,
                position.y,
                (uint)layerMask);
            return ReadCollider2DArray(count);
        }

        public Collider2D[] OverlapCircle2D(Vector2 position, float radius, int layerMask)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_circle2d(
                _runtimeHandle,
                position.x,
                position.y,
                radius,
                (uint)layerMask);
            return ReadCollider2DArray(count);
        }

        public Collider2D[] OverlapBox2D(Vector2 position, Vector2 size, float angle, int layerMask)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_box2d(
                _runtimeHandle,
                position.x,
                position.y,
                size.x / 2f,
                size.y / 2f,
                angle,
                (uint)layerMask);
            return ReadCollider2DArray(count);
        }

        public bool TryGetBodyVelocity2D(ulong entity, out Vector2 velocity)
        {
            velocity = default;
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 8);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_body2d_velocity(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[8];
            Marshal.Copy(buffer, bytes, 0, bytes.Length);
            velocity = new Vector2(BitConverter.ToSingle(bytes, 0), BitConverter.ToSingle(bytes, 4));
            return true;
        }

        public bool TrySetBodyVelocity2D(ulong entity, Vector2 velocity)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_set_velocity(_runtimeHandle, entity, velocity.x, velocity.y) == 0;
        }

        public bool TryMoveBodyPosition2D(ulong entity, Vector2 position)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_move_position(_runtimeHandle, entity, position.x, position.y) == 0;
        }

        public bool TrySetBodySimulated2D(ulong entity, bool simulated)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_set_simulated(_runtimeHandle, entity, simulated ? 1 : 0) == 0;
        }

        public bool TrySetBodyType2D(ulong entity, RigidbodyType2D bodyType)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_set_body_type(_runtimeHandle, entity, (int)bodyType) == 0;
        }

        public bool TrySetBodyInterpolation2D(ulong entity, RigidbodyInterpolation2D interpolation)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_set_interpolation(_runtimeHandle, entity, (int)interpolation) == 0;
        }

        public bool TrySetBodyCollisionDetectionMode2D(ulong entity, CollisionDetectionMode2D mode)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_set_collision_detection_mode(_runtimeHandle, entity, (int)mode) == 0;
        }

        public bool TrySetBodyConstraints2D(ulong entity, RigidbodyConstraints2D constraints)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_body2d_set_constraints(_runtimeHandle, entity, (uint)constraints) == 0;
        }

        public bool TryCreateCapsuleCollider2D(ulong entity, float radius, float height, int direction, Vector2 offset, bool isTrigger)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_collider2d_create_capsule(
                _runtimeHandle,
                entity,
                radius,
                height,
                (uint)direction,
                offset.x,
                offset.y,
                isTrigger ? 1 : 0,
                0u) == 0;
        }

        public RaycastHit2D[] BoxCast2D(Vector2 origin, Vector2 size, float angle, Vector2 direction, float distance, int layerMask, ContactFilter2D filter)
        {
            int count = RuntimeInterop.unitas_runtime_box_cast2d(
                _runtimeHandle,
                origin.x,
                origin.y,
                size.x * 0.5f,
                size.y * 0.5f,
                angle,
                direction.x,
                direction.y,
                distance,
                filter.useTriggers ? 1 : 0,
                (uint)layerMask,
                filter.useDepth ? 1 : 0,
                filter.minDepth,
                filter.maxDepth,
                filter.useOutsideNormalAngle ? 1 : 0,
                filter.minNormalAngle,
                filter.maxNormalAngle);
            if (count <= 0)
            {
                return Array.Empty<RaycastHit2D>();
            }
            QueryHit2D[] hits = ReadQueryHits2D(count);
            List<RaycastHit2D> results = new List<RaycastHit2D>(hits.Length);
            for (int index = 0; index < hits.Length; index++)
            {
                QueryHit2D hit = hits[index];
                results.Add(new RaycastHit2D
                {
                    collider = ResolveCollider2DByEntity(hit.Entity),
                    point = new Vector2(hit.X, hit.Y),
                    normal = new Vector2(hit.NX, hit.NY),
                    distance = hit.Distance,
                    fraction = distance > 0f ? hit.Distance / distance : 0f,
                });
            }
            return results.ToArray();
        }

        public bool TryGetCollisionSummary2D(ulong a, ulong b, out CollisionSummary2D summary)
        {
            summary = default;
            if (_isDisposed || a == 0 || b == 0)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_collision_summary_2d(a, b, out RuntimeInterop.AbiCollisionSummary2D native) != 0)
            {
                return false;
            }
            summary = new CollisionSummary2D
            {
                impulse = native.Impulse,
                relativeVelocity = native.RelativeVelocity,
                contactCount = (int)native.ContactCount,
                point = native.Point,
                normal = native.Normal,
            };
            return true;
        }

        public bool TryGetCollisionContact2D(ulong a, ulong b, uint index, out ContactPoint2D contact)
        {
            contact = default;
            if (_isDisposed || a == 0 || b == 0)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_collision_contact_2d(a, b, index, out RuntimeInterop.AbiCollisionContactPoint2D native) != 0)
            {
                return false;
            }
            contact = new ContactPoint2D
            {
                point = native.Point,
                normal = native.Normal,
                impulse = native.Impulse,
            };
            return true;
        }

        private Collider2D[] ReadCollider2DArray(int count)
        {
            if (count <= 0)
            {
                return Array.Empty<Collider2D>();
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_query_buffer_2d(
                _runtimeHandle,
                (uint)(count * RuntimeInterop.QueryRecord2DSize));
            if (buffer == IntPtr.Zero)
            {
                return Array.Empty<Collider2D>();
            }
            QueryHit2D[] hits = RuntimeInterop.ReadQueryHits2D(buffer, count);
            Collider2D[] colliders = new Collider2D[hits.Length];
            for (int index = 0; index < hits.Length; index++)
            {
                colliders[index] = ResolveCollider2DByEntity(hits[index].Entity)!;
            }
            return colliders;
        }

        private QueryHit2D[] ReadQueryHits2D(int count)
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_query_buffer_2d(
                _runtimeHandle,
                (uint)(count * RuntimeInterop.QueryRecord2DSize));
            if (buffer == IntPtr.Zero)
            {
                return Array.Empty<QueryHit2D>();
            }
            return RuntimeInterop.ReadQueryHits2D(buffer, count);
        }

        private bool TryReadCollisionSummary(ulong a, ulong b, out RuntimeInterop.AbiCollisionSummary native)
        {
            if (RuntimeInterop.unitas_runtime_collision_summary(a, b, out native) == 0)
            {
                return true;
            }
            if (a != b && RuntimeInterop.unitas_runtime_collision_summary(b, a, out native) == 0)
            {
                return true;
            }
            native = default;
            return false;
        }

        private bool TryReadCollisionContact(ulong a, ulong b, uint index, out RuntimeInterop.AbiCollisionContactPoint native)
        {
            if (RuntimeInterop.unitas_runtime_collision_contact(a, b, index, out native) == 0)
            {
                return true;
            }
            if (a != b && RuntimeInterop.unitas_runtime_collision_contact(b, a, index, out native) == 0)
            {
                return true;
            }
            native = default;
            return false;
        }

        public bool TryMove(CharacterMoveInput input, out CharacterMoveOutput output)
        {
            output = default;
            if (_isDisposed)
            {
                return false;
            }
            RuntimeInterop.AbiCharacterMoveInput abiInput = new RuntimeInterop.AbiCharacterMoveInput
            {
                EntityId = input.Entity,
                PositionX = input.Position.x,
                PositionY = input.Position.y,
                PositionZ = input.Position.z,
                MotionX = input.Motion.x,
                MotionY = input.Motion.y,
                MotionZ = input.Motion.z,
                Radius = input.Radius,
                Height = input.Height,
                CenterX = input.Center.x,
                CenterY = input.Center.y,
                CenterZ = input.Center.z,
                SlopeLimitDeg = input.SlopeLimit,
                StepOffset = input.StepOffset,
                SkinWidth = input.SkinWidth,
                MinMoveDistance = input.MinMoveDistance,
                DeltaSeconds = input.DeltaSeconds,
                LayerMask = input.LayerMask,
                Enabled = input.Enabled ? (byte)1 : (byte)0,
            };
            if (RuntimeInterop.unitas_runtime_character_move(_runtimeHandle, ref abiInput, out RuntimeInterop.AbiCharacterMoveOutput abiOutput) != 0)
            {
                return false;
            }
            output = new CharacterMoveOutput(
                new Vector3((float)abiOutput.PositionX, (float)abiOutput.PositionY, (float)abiOutput.PositionZ),
                abiOutput.GroundEntityId,
                (CollisionFlags)abiOutput.CollisionFlags,
                abiOutput.IsGrounded != 0);
            return true;
        }

        public bool TryTeleport(ulong entity, Vector3 position)
        {
            if (_isDisposed)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            BitConverter.GetBytes((double)position.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)position.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)position.z).CopyTo(bytes, 16);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return RuntimeInterop.unitas_runtime_character_teleport(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryGetConfig(ulong entity, out CharacterControllerConfig config)
        {
            config = default;
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 76);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_character_controller_config(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[76];
            Marshal.Copy(buffer, bytes, 0, bytes.Length);
            float radius = (float)BitConverter.ToDouble(bytes, 0);
            float height = (float)BitConverter.ToDouble(bytes, 8);
            float centerX = (float)BitConverter.ToDouble(bytes, 16);
            float centerY = (float)BitConverter.ToDouble(bytes, 24);
            float centerZ = (float)BitConverter.ToDouble(bytes, 32);
            float slopeLimit = (float)BitConverter.ToDouble(bytes, 40);
            float stepOffset = (float)BitConverter.ToDouble(bytes, 48);
            float skinWidth = (float)BitConverter.ToDouble(bytes, 56);
            float minMoveDistance = (float)BitConverter.ToDouble(bytes, 64);
            bool enabled = bytes[72] != 0;
            config = new CharacterControllerConfig(
                radius,
                height,
                new Vector3(centerX, centerY, centerZ),
                slopeLimit,
                stepOffset,
                skinWidth,
                minMoveDistance,
                enabled);
            return true;
        }

        private bool TryReadBodyVector(ulong entity, Func<ulong, ulong, IntPtr, int> query, out Vector3 value)
        {
            value = Vector3.zero;
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero || query(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            Marshal.Copy(buffer, bytes, 0, 24);
            float x = (float)BitConverter.ToDouble(bytes, 0);
            float y = (float)BitConverter.ToDouble(bytes, 8);
            float z = (float)BitConverter.ToDouble(bytes, 16);
            value = new Vector3(x, y, z);
            return true;
        }

        private bool TryWriteBodyVector(ulong entity, Func<ulong, ulong, IntPtr, int> setter, Vector3 value)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 24);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[24];
            BitConverter.GetBytes((double)value.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)value.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)value.z).CopyTo(bytes, 16);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return setter(_runtimeHandle, entity, buffer) == 0;
        }

        private bool TryWriteBodyQuaternion(ulong entity, Func<ulong, ulong, IntPtr, int> setter, Quaternion value)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            byte[] bytes = new byte[32];
            BitConverter.GetBytes((double)value.x).CopyTo(bytes, 0);
            BitConverter.GetBytes((double)value.y).CopyTo(bytes, 8);
            BitConverter.GetBytes((double)value.z).CopyTo(bytes, 16);
            BitConverter.GetBytes((double)value.w).CopyTo(bytes, 24);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            return setter(_runtimeHandle, entity, buffer) == 0;
        }

        private bool TakeNearestHit(int count, out RaycastHit hit)
        {
            hit = default;
            if (count <= 0)
            {
                return false;
            }
            QueryHit[] hits = ReadQueryHits(count);
            if (hits.Length == 0)
            {
                return false;
            }
            QueryHit nearest = hits[0];
            Collider? collider = GetCollider(nearest.Entity);
            hit.collider = collider!;
            hit.point = new Vector3((float)nearest.X, (float)nearest.Y, (float)nearest.Z);
            hit.normal = new Vector3((float)nearest.NX, (float)nearest.NY, (float)nearest.NZ);
            hit.distance = (float)nearest.Distance;
            return true;
        }

        private QueryHit[] ReadQueryHits(int count)
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_query_buffer(
                _runtimeHandle,
                (uint)(count * RuntimeInterop.QueryRecordSize));
            if (buffer == IntPtr.Zero)
            {
                return Array.Empty<QueryHit>();
            }
            return RuntimeInterop.ReadQueryHits(buffer, count);
        }

        private void ApplyScenePhysicsComponents(ulong entity, GameObject owner)
        {
            if (_sceneRigidbodies.TryGetValue(entity, out SceneRigidbodyMetadata metadata))
            {
                Rigidbody body = owner.GetComponent<Rigidbody>() ?? owner.AddComponent<Rigidbody>();
                body.mass = metadata.Mass;
                body.useGravity = metadata.UseGravity;
                body.isKinematic = metadata.IsKinematic;
            }
            if (_sceneSphereColliders.TryGetValue(entity, out SceneSphereColliderMetadata sphereMeta))
            {
                SphereCollider sphere = owner.GetComponent<SphereCollider>() ?? owner.AddComponent<SphereCollider>();
                sphere.radius = sphereMeta.Radius;
                sphere.center = sphereMeta.Center;
                sphere.isTrigger = sphereMeta.IsTrigger;
            }
            if (_sceneBoxColliders.TryGetValue(entity, out SceneBoxColliderMetadata boxMeta))
            {
                BoxCollider box = owner.GetComponent<BoxCollider>() ?? owner.AddComponent<BoxCollider>();
                box.size = boxMeta.Size;
                box.center = boxMeta.Center;
                box.isTrigger = boxMeta.IsTrigger;
            }
            if (_sceneCapsuleColliders.TryGetValue(entity, out SceneCapsuleColliderMetadata capsuleMeta))
            {
                CapsuleCollider capsule = owner.GetComponent<CapsuleCollider>() ?? owner.AddComponent<CapsuleCollider>();
                capsule.radius = capsuleMeta.Radius;
                capsule.height = capsuleMeta.Height;
                capsule.direction = capsuleMeta.Direction;
                capsule.center = capsuleMeta.Center;
                capsule.isTrigger = capsuleMeta.IsTrigger;
            }
            if (_sceneMeshColliders.TryGetValue(entity, out SceneMeshColliderMetadata meshMeta))
            {
                MeshCollider mesh = owner.GetComponent<MeshCollider>() ?? owner.AddComponent<MeshCollider>();
                mesh.convex = meshMeta.Convex;
                mesh.center = meshMeta.Center;
                mesh.isTrigger = meshMeta.IsTrigger;
                mesh.sharedMesh = ResolveSceneMesh(meshMeta.MeshGuid, meshMeta.MeshFileId);
            }
            ApplyScenePhysics2DComponents(entity, owner);
        }

        private void MaterializeScenePhysics2D()
        {
            foreach (ulong entity in _sceneRigidbodies2D.Keys)
            {
                ApplyScenePhysicsComponents(entity, EnsureSceneGameObject(entity));
            }
            foreach (ulong entity in _sceneBoxColliders2D.Keys)
            {
                ApplyScenePhysicsComponents(entity, EnsureSceneGameObject(entity));
            }
            foreach (ulong entity in _sceneCircleColliders2D.Keys)
            {
                ApplyScenePhysicsComponents(entity, EnsureSceneGameObject(entity));
            }
        }

        private void ApplyScenePhysics2DComponents(ulong entity, GameObject owner)
        {
            if (_sceneRigidbodies2D.TryGetValue(entity, out SceneRigidbody2DMetadata rigidbodyMeta))
            {
                Rigidbody2D body = owner.GetComponent<Rigidbody2D>() ?? owner.AddComponent<Rigidbody2D>();
                body.mass = rigidbodyMeta.Mass;
                body.gravityScale = rigidbodyMeta.GravityScale;
            }
            if (_sceneBoxColliders2D.TryGetValue(entity, out SceneBoxCollider2DMetadata boxMeta))
            {
                BoxCollider2D box = owner.GetComponent<BoxCollider2D>() ?? owner.AddComponent<BoxCollider2D>();
                box.size = boxMeta.Size;
                box.offset = boxMeta.Offset;
                box.isTrigger = boxMeta.IsTrigger;
            }
            if (_sceneCircleColliders2D.TryGetValue(entity, out SceneCircleCollider2DMetadata circleMeta))
            {
                CircleCollider2D circle = owner.GetComponent<CircleCollider2D>() ?? owner.AddComponent<CircleCollider2D>();
                circle.radius = circleMeta.Radius;
                circle.offset = circleMeta.Offset;
                circle.isTrigger = circleMeta.IsTrigger;
            }
            if (_scenePlatformEffectors2D.TryGetValue(entity, out ScenePlatformEffector2DMetadata platformMeta))
            {
                PlatformEffector2D platform = owner.GetComponent<PlatformEffector2D>() ?? owner.AddComponent<PlatformEffector2D>();
                platform.useOneWay = platformMeta.UseOneWay;
                platform.surfaceArc = platformMeta.SurfaceArc;
                platform.rotationalOffset = platformMeta.RotationalOffset;
                platform.sideArc = platformMeta.SideArc;
                platform.useSideFriction = platformMeta.UseSideFriction;
                platform.useSideBounce = platformMeta.UseSideBounce;
            }
            if (_sceneSurfaceEffectors2D.TryGetValue(entity, out SceneSurfaceEffector2DMetadata surfaceMeta))
            {
                SurfaceEffector2D surface = owner.GetComponent<SurfaceEffector2D>() ?? owner.AddComponent<SurfaceEffector2D>();
                surface.speed = surfaceMeta.Speed;
                surface.forceScale = surfaceMeta.ForceScale;
                surface.useFriction = surfaceMeta.UseFriction;
                surface.useBounce = surfaceMeta.UseBounce;
            }
            RegisterScenePhysics2D(entity, owner);
            if (_sceneRigidbodies2D.TryGetValue(entity, out SceneRigidbody2DMetadata bodyMeta))
            {
                Rigidbody2D body = owner.GetComponent<Rigidbody2D>()!;
                body.bodyType = bodyMeta.BodyType;
                body.simulated = bodyMeta.Simulated;
                body.interpolation = bodyMeta.Interpolation;
                body.collisionDetectionMode = bodyMeta.CollisionDetectionMode;
            }
        }

        private void RegisterScenePhysics2D(ulong entity, GameObject owner)
        {
            bool hasBody = _sceneRigidbodies2D.ContainsKey(entity);
            bool hasBox = _sceneBoxColliders2D.ContainsKey(entity);
            bool hasCircle = _sceneCircleColliders2D.ContainsKey(entity);
            if (!hasBody && !hasBox && !hasCircle)
            {
                return;
            }
            if (_registeredBodies2D.Contains(entity))
            {
                return;
            }
            _registeredBodies2D.Add(entity);
            Vector3 position = owner.transform.position;
            SceneRigidbody2DMetadata bodyMetadata = hasBody
                ? _sceneRigidbodies2D[entity]
                : default(SceneRigidbody2DMetadata);
            RigidbodyType2D bodyType = hasBody ? bodyMetadata.BodyType : RigidbodyType2D.Static;
            RuntimeInterop.unitas_runtime_body2d_create(
                _runtimeHandle,
                entity,
                position.x,
                position.y,
                0f,
                hasBody ? bodyMetadata.Mass : 0f,
                hasBody ? bodyMetadata.GravityScale : 0f,
                (uint)bodyType,
                0u,
                1u,
                0u,
                0u);
            if (hasBox)
            {
                SceneBoxCollider2DMetadata metadata = _sceneBoxColliders2D[entity];
                RuntimeInterop.unitas_runtime_collider2d_create(
                    _runtimeHandle,
                    entity,
                    entity,
                    0u,
                    metadata.Offset.x,
                    metadata.Offset.y,
                    metadata.Size.x,
                    metadata.Size.y,
                    0f,
                    metadata.IsTrigger ? 1u : 0u,
                    0u);
            }
            if (hasCircle)
            {
                SceneCircleCollider2DMetadata metadata = _sceneCircleColliders2D[entity];
                RuntimeInterop.unitas_runtime_collider2d_create(
                    _runtimeHandle,
                    entity,
                    entity,
                    1u,
                    metadata.Offset.x,
                    metadata.Offset.y,
                    0f,
                    0f,
                    metadata.Radius,
                    metadata.IsTrigger ? 1u : 0u,
                    0u);
            }
            if (_scenePlatformEffectors2D.TryGetValue(entity, out ScenePlatformEffector2DMetadata platformMetadata))
            {
                RuntimeInterop.unitas_runtime_collider2d_set_platform_effector(
                    _runtimeHandle,
                    entity,
                    platformMetadata.UseOneWay ? 1 : 0,
                    platformMetadata.SurfaceArc,
                    platformMetadata.RotationalOffset,
                    platformMetadata.SideArc,
                    platformMetadata.UseSideFriction ? 1 : 0,
                    platformMetadata.UseSideBounce ? 1 : 0);
            }
            if (_sceneSurfaceEffectors2D.TryGetValue(entity, out SceneSurfaceEffector2DMetadata surfaceMetadata))
            {
                RuntimeInterop.unitas_runtime_collider2d_set_surface_effector(
                    _runtimeHandle,
                    entity,
                    surfaceMetadata.Speed,
                    surfaceMetadata.ForceScale,
                    surfaceMetadata.UseFriction ? 1 : 0,
                    surfaceMetadata.UseBounce ? 1 : 0);
            }
        }

        private readonly struct SceneRigidbodyMetadata
        {
            public SceneRigidbodyMetadata(float mass, bool useGravity, bool isKinematic)
            {
                Mass = mass;
                UseGravity = useGravity;
                IsKinematic = isKinematic;
            }

            public float Mass { get; }

            public bool UseGravity { get; }

            public bool IsKinematic { get; }
        }

        private sealed class ScenePhysics2DFields
        {
            public int BodyType { get; set; }

            public bool Simulated { get; set; } = true;

            public float Mass { get; set; } = 1f;

            public float GravityScale { get; set; } = 1f;

            public int Interpolation { get; set; }

            public int CollisionDetectionMode { get; set; }

            public Vector2 Offset { get; set; }

            public Vector2 Size { get; set; } = Vector2.one;

            public float Radius { get; set; } = 0.5f;

            public bool IsTrigger { get; set; }

            public bool UseOneWay { get; set; } = true;

            public float SurfaceArc { get; set; } = 180f;

            public float RotationalOffset { get; set; }

            public float SideArc { get; set; }

            public bool UseSideFriction { get; set; }

            public bool UseSideBounce { get; set; }

            public float Speed { get; set; }

            public float ForceScale { get; set; } = 1f;

            public bool UseFriction { get; set; } = true;

            public bool UseBounce { get; set; }
        }

        private static void RecordScenePhysics2DComponent(
            int classId,
            string gameObjectFileId,
            ScenePhysics2DFields fields,
            List<ParsedRigidbody2DRecord> rigidbodies2D,
            List<ParsedBoxCollider2DRecord> boxColliders2D,
            List<ParsedCircleCollider2DRecord> circleColliders2D,
            List<ParsedPlatformEffector2DRecord> platformEffectors2D,
            List<ParsedSurfaceEffector2DRecord> surfaceEffectors2D)
        {
            if (string.IsNullOrEmpty(gameObjectFileId))
            {
                return;
            }
            switch (classId)
            {
                case RIGIDBODY2D_CLASS:
                    rigidbodies2D.Add(new ParsedRigidbody2DRecord(
                        gameObjectFileId,
                        fields.BodyType,
                        fields.Simulated,
                        fields.Mass,
                        fields.GravityScale,
                        fields.Interpolation,
                        fields.CollisionDetectionMode));
                    break;
                case BOX_COLLIDER2D_CLASS:
                    boxColliders2D.Add(new ParsedBoxCollider2DRecord(
                        gameObjectFileId,
                        fields.Size,
                        fields.Offset,
                        fields.IsTrigger));
                    break;
                case CIRCLE_COLLIDER2D_CLASS:
                    circleColliders2D.Add(new ParsedCircleCollider2DRecord(
                        gameObjectFileId,
                        fields.Radius,
                        fields.Offset,
                        fields.IsTrigger));
                    break;
                case PLATFORM_EFFECTOR2D_CLASS:
                    platformEffectors2D.Add(new ParsedPlatformEffector2DRecord(
                        gameObjectFileId,
                        fields.UseOneWay,
                        fields.SurfaceArc,
                        fields.RotationalOffset,
                        fields.SideArc,
                        fields.UseSideFriction,
                        fields.UseSideBounce));
                    break;
                case SURFACE_EFFECTOR2D_CLASS:
                    surfaceEffectors2D.Add(new ParsedSurfaceEffector2DRecord(
                        gameObjectFileId,
                        fields.Speed,
                        fields.ForceScale,
                        fields.UseFriction,
                        fields.UseBounce));
                    break;
                default:
                    break;
            }
        }

        private readonly struct SceneRigidbody2DMetadata
        {
            public SceneRigidbody2DMetadata(RigidbodyType2D bodyType, bool simulated, float mass, float gravityScale, RigidbodyInterpolation2D interpolation, CollisionDetectionMode2D collisionDetectionMode)
            {
                BodyType = bodyType;
                Simulated = simulated;
                Mass = mass;
                GravityScale = gravityScale;
                Interpolation = interpolation;
                CollisionDetectionMode = collisionDetectionMode;
            }

            public RigidbodyType2D BodyType { get; }

            public bool Simulated { get; }

            public float Mass { get; }

            public float GravityScale { get; }

            public RigidbodyInterpolation2D Interpolation { get; }

            public CollisionDetectionMode2D CollisionDetectionMode { get; }
        }

        private readonly struct SceneBoxCollider2DMetadata
        {
            public SceneBoxCollider2DMetadata(Vector2 size, Vector2 offset, bool isTrigger)
            {
                Size = size;
                Offset = offset;
                IsTrigger = isTrigger;
            }

            public Vector2 Size { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneCircleCollider2DMetadata
        {
            public SceneCircleCollider2DMetadata(float radius, Vector2 offset, bool isTrigger)
            {
                Radius = radius;
                Offset = offset;
                IsTrigger = isTrigger;
            }

            public float Radius { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct ScenePlatformEffector2DMetadata
        {
            public ScenePlatformEffector2DMetadata(bool useOneWay, float surfaceArc, float rotationalOffset, float sideArc, bool useSideFriction, bool useSideBounce)
            {
                UseOneWay = useOneWay;
                SurfaceArc = surfaceArc;
                RotationalOffset = rotationalOffset;
                SideArc = sideArc;
                UseSideFriction = useSideFriction;
                UseSideBounce = useSideBounce;
            }

            public bool UseOneWay { get; }

            public float SurfaceArc { get; }

            public float RotationalOffset { get; }

            public float SideArc { get; }

            public bool UseSideFriction { get; }

            public bool UseSideBounce { get; }
        }

        private readonly struct SceneSurfaceEffector2DMetadata
        {
            public SceneSurfaceEffector2DMetadata(float speed, float forceScale, bool useFriction, bool useBounce)
            {
                Speed = speed;
                ForceScale = forceScale;
                UseFriction = useFriction;
                UseBounce = useBounce;
            }

            public float Speed { get; }

            public float ForceScale { get; }

            public bool UseFriction { get; }

            public bool UseBounce { get; }
        }

        private readonly struct SceneSphereColliderMetadata
        {
            public SceneSphereColliderMetadata(float radius, Vector3 center, bool isTrigger)
            {
                Radius = radius;
                Center = center;
                IsTrigger = isTrigger;
            }

            public float Radius { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneBoxColliderMetadata
        {
            public SceneBoxColliderMetadata(Vector3 size, Vector3 center, bool isTrigger)
            {
                Size = size;
                Center = center;
                IsTrigger = isTrigger;
            }

            public Vector3 Size { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneCapsuleColliderMetadata
        {
            public SceneCapsuleColliderMetadata(float radius, float height, int direction, Vector3 center, bool isTrigger)
            {
                Radius = radius;
                Height = height;
                Direction = direction;
                Center = center;
                IsTrigger = isTrigger;
            }

            public float Radius { get; }

            public float Height { get; }

            public int Direction { get; }

            public Vector3 Center { get; }

            public bool IsTrigger { get; }
        }

        private readonly struct SceneMeshColliderMetadata
        {
            public SceneMeshColliderMetadata(string meshFileId, string meshGuid, bool convex, bool isTrigger, Vector3 center)
            {
                MeshFileId = meshFileId;
                MeshGuid = meshGuid;
                Convex = convex;
                IsTrigger = isTrigger;
                Center = center;
            }

            public string MeshFileId { get; }

            public string MeshGuid { get; }

            public bool Convex { get; }

            public bool IsTrigger { get; }

            public Vector3 Center { get; }
        }
    }
}
