using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
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

        public bool TryCreateCapsuleCollider2D(ulong entity, float radius, float height, int direction, Vector2 offset, bool isTrigger, uint layer)
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
                layer) == 0;
        }

        public bool TryCreatePolygonCollider2D(ulong entity, ulong attachedBody, Vector2[] points, Vector2 offset, bool isTrigger, uint layer)
        {
            if (_isDisposed || entity == 0 || points == null || points.Length < 3)
            {
                return false;
            }
            float[] rawPoints = new float[points.Length * 2];
            for (int index = 0; index < points.Length; index++)
            {
                rawPoints[index * 2] = points[index].x;
                rawPoints[index * 2 + 1] = points[index].y;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)(rawPoints.Length * sizeof(float)));
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            Marshal.Copy(rawPoints, 0, buffer, rawPoints.Length);
            return RuntimeInterop.unitas_runtime_collider2d_create_polygon(
                _runtimeHandle,
                entity,
                attachedBody,
                buffer,
                (uint)points.Length,
                offset.x,
                offset.y,
                isTrigger ? 1u : 0u,
                layer) == 0;
        }

        public bool TryCreateEdgeCollider2D(ulong entity, ulong attachedBody, Vector2[] points, Vector2 offset, bool isTrigger, uint layer)
        {
            if (_isDisposed || entity == 0 || points == null || points.Length < 2)
            {
                return false;
            }
            float[] rawPoints = new float[points.Length * 2];
            for (int index = 0; index < points.Length; index++)
            {
                rawPoints[index * 2] = points[index].x;
                rawPoints[index * 2 + 1] = points[index].y;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)(rawPoints.Length * sizeof(float)));
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            Marshal.Copy(rawPoints, 0, buffer, rawPoints.Length);
            return RuntimeInterop.unitas_runtime_collider2d_create_edge(
                _runtimeHandle,
                entity,
                attachedBody,
                buffer,
                (uint)points.Length,
                offset.x,
                offset.y,
                isTrigger ? 1u : 0u,
                layer) == 0;
        }

        public bool TrySetCompositeCollider2D(ulong entity, ulong compositeEntity, uint geometryType)
        {
            if (_isDisposed || entity == 0 || compositeEntity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_collider2d_set_composite(
                _runtimeHandle,
                entity,
                compositeEntity,
                geometryType) == 0;
        }

        public bool TryBuildTilemapCollider2D(ulong entity, ulong compositeEntity, string tilemapPath)
        {
            if (_isDisposed || entity == 0 || string.IsNullOrEmpty(tilemapPath))
            {
                return false;
            }
            byte[] utf8Bytes = Encoding.UTF8.GetBytes(tilemapPath);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)utf8Bytes.Length);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            Marshal.Copy(utf8Bytes, 0, buffer, utf8Bytes.Length);
            return RuntimeInterop.unitas_runtime_tilemap_collider2d_build(
                _runtimeHandle,
                entity,
                compositeEntity,
                buffer,
                (uint)utf8Bytes.Length) == 0;
        }

        public bool TrySetColliderMaterial2D(ulong entity, float friction, float bounciness)
        {
            if (_isDisposed || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_collider2d_set_material(
                _runtimeHandle,
                entity,
                friction,
                bounciness) == 0;
        }

        public bool TryApplyPhysics2DSettings(Vector2 gravity, int velocityIterations, int positionIterations, bool queriesHitTriggers)
        {
            if (_isDisposed)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_physics2d_apply_settings(
                _runtimeHandle,
                gravity.x,
                gravity.y,
                (uint)Math.Max(1, velocityIterations),
                (uint)Math.Max(1, positionIterations),
                queriesHitTriggers ? 1u : 0u) == 0;
        }

        public bool TrySetLayerCollision2D(int layerA, int layerB, bool enabled)
        {
            if (_isDisposed || layerA < 0 || layerA >= 32 || layerB < 0 || layerB >= 32)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_physics2d_set_layer_collision(
                _runtimeHandle,
                (uint)layerA,
                (uint)layerB,
                enabled ? 1u : 0u) == 0;
        }

        public bool TryGetLayerCollision2D(int layerA, int layerB, out bool enabled)
        {
            enabled = true;
            if (_isDisposed || layerA < 0 || layerA >= 32 || layerB < 0 || layerB >= 32)
            {
                return false;
            }
            int result = RuntimeInterop.unitas_runtime_physics2d_get_layer_collision(
                _runtimeHandle,
                (uint)layerA,
                (uint)layerB);
            if (result < 0)
            {
                return false;
            }
            enabled = result != 0;
            return true;
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
            HashSet<ulong> entities = new HashSet<ulong>();
            entities.UnionWith(_sceneRigidbodies2D.Keys);
            entities.UnionWith(_sceneBoxColliders2D.Keys);
            entities.UnionWith(_sceneCircleColliders2D.Keys);
            entities.UnionWith(_sceneCapsuleColliders2D.Keys);
            entities.UnionWith(_scenePolygonColliders2D.Keys);
            entities.UnionWith(_sceneEdgeColliders2D.Keys);
            entities.UnionWith(_sceneCompositeColliders2D.Keys);
            entities.UnionWith(_sceneTilemapColliders2D.Keys);
            foreach (ulong entity in entities)
            {
                GameObject owner = EnsureSceneGameObject(entity);
                WireSceneParent(entity, owner);
            }
            foreach (ulong entity in entities)
            {
                ApplyScenePhysicsComponents(entity, EnsureSceneGameObject(entity));
            }
            foreach (ulong entity in _sceneCompositeColliders2D.Keys)
            {
                CompositeCollider2D? composite = EnsureSceneGameObject(entity).GetComponent<CompositeCollider2D>();
                composite?.GenerateGeometry();
            }
        }

        private void ApplyScenePhysics2DComponents(ulong entity, GameObject owner)
        {
            if (_registeredBodies2D.Contains(entity))
            {
                return;
            }
            RegisterScenePhysics2D(entity, owner);
            if (_sceneRigidbodies2D.TryGetValue(entity, out SceneRigidbody2DMetadata rigidbodyMeta))
            {
                Rigidbody2D body = owner.GetComponent<Rigidbody2D>() ?? owner.AddComponent<Rigidbody2D>();
                body.mass = rigidbodyMeta.Mass;
                body.gravityScale = rigidbodyMeta.GravityScale;
                body.bodyType = rigidbodyMeta.BodyType;
                body.simulated = rigidbodyMeta.Simulated;
                body.interpolation = rigidbodyMeta.Interpolation;
                body.collisionDetectionMode = rigidbodyMeta.CollisionDetectionMode;
                body.constraints = rigidbodyMeta.Constraints;
            }
            if (_sceneCompositeColliders2D.TryGetValue(entity, out SceneCompositeCollider2DMetadata compositeMeta))
            {
                CompositeCollider2D composite = owner.GetComponent<CompositeCollider2D>() ?? owner.AddComponent<CompositeCollider2D>();
                composite.geometryType = (CompositeCollider2D.GeometryType)compositeMeta.GeometryType;
                composite.generationType = (CompositeCollider2D.GenerationType)compositeMeta.GenerationType;
                composite.vertexDistance = compositeMeta.VertexDistance;
                composite.offsetDistance = compositeMeta.OffsetDistance;
                composite.edgeRadius = compositeMeta.EdgeRadius;
                ApplySceneCollider2DCommon(composite, compositeMeta.Offset, compositeMeta.IsTrigger, compositeMeta.UsedByEffector, false, compositeMeta.Material);
            }
            if (_sceneBoxColliders2D.TryGetValue(entity, out SceneBoxCollider2DMetadata boxMeta))
            {
                BoxCollider2D box = owner.GetComponent<BoxCollider2D>() ?? owner.AddComponent<BoxCollider2D>();
                box.size = boxMeta.Size;
                ApplySceneCollider2DCommon(box, boxMeta.Offset, boxMeta.IsTrigger, boxMeta.UsedByEffector, boxMeta.UsedByComposite, boxMeta.Material);
            }
            if (_sceneCircleColliders2D.TryGetValue(entity, out SceneCircleCollider2DMetadata circleMeta))
            {
                CircleCollider2D circle = owner.GetComponent<CircleCollider2D>() ?? owner.AddComponent<CircleCollider2D>();
                circle.radius = circleMeta.Radius;
                ApplySceneCollider2DCommon(circle, circleMeta.Offset, circleMeta.IsTrigger, circleMeta.UsedByEffector, circleMeta.UsedByComposite, circleMeta.Material);
            }
            if (_sceneCapsuleColliders2D.TryGetValue(entity, out SceneCapsuleCollider2DMetadata capsuleMeta))
            {
                CapsuleCollider2D capsule = owner.GetComponent<CapsuleCollider2D>() ?? owner.AddComponent<CapsuleCollider2D>();
                capsule.size = capsuleMeta.Size;
                capsule.direction = capsuleMeta.Direction;
                ApplySceneCollider2DCommon(capsule, capsuleMeta.Offset, capsuleMeta.IsTrigger, capsuleMeta.UsedByEffector, capsuleMeta.UsedByComposite, capsuleMeta.Material);
            }
            if (_scenePolygonColliders2D.TryGetValue(entity, out ScenePolygonCollider2DMetadata polygonMeta))
            {
                PolygonCollider2D polygon = owner.GetComponent<PolygonCollider2D>() ?? owner.AddComponent<PolygonCollider2D>();
                if (polygonMeta.Points.Length > 0)
                {
                    polygon.points = polygonMeta.Points;
                }
                ApplySceneCollider2DCommon(polygon, polygonMeta.Offset, polygonMeta.IsTrigger, polygonMeta.UsedByEffector, polygonMeta.UsedByComposite, polygonMeta.Material);
            }
            if (_sceneEdgeColliders2D.TryGetValue(entity, out SceneEdgeCollider2DMetadata edgeMeta))
            {
                EdgeCollider2D edge = owner.GetComponent<EdgeCollider2D>() ?? owner.AddComponent<EdgeCollider2D>();
                if (edgeMeta.Points.Length > 0)
                {
                    edge.points = edgeMeta.Points;
                }
                edge.edgeRadius = edgeMeta.EdgeRadius;
                ApplySceneCollider2DCommon(edge, edgeMeta.Offset, edgeMeta.IsTrigger, edgeMeta.UsedByEffector, edgeMeta.UsedByComposite, edgeMeta.Material);
            }
            if (_sceneTilemapColliders2D.TryGetValue(entity, out SceneTilemapCollider2DMetadata tilemapMeta))
            {
                TilemapCollider2D tilemap = owner.GetComponent<TilemapCollider2D>() ?? owner.AddComponent<TilemapCollider2D>();
                tilemap.maximumTileChangeCount = tilemapMeta.MaximumTileChangeCount;
                tilemap.extrusionFactor = tilemapMeta.ExtrusionFactor;
                ApplySceneCollider2DCommon(tilemap, tilemapMeta.Offset, tilemapMeta.IsTrigger, tilemapMeta.UsedByEffector, tilemapMeta.UsedByComposite, tilemapMeta.Material);
                tilemap.tilemapAssetPath = tilemapMeta.TilemapPath;
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
        }

        private static void ApplySceneCollider2DCommon(
            Collider2D collider,
            Vector2 offset,
            bool isTrigger,
            bool usedByEffector,
            bool usedByComposite,
            PhysicsMaterial2D? material)
        {
            collider.offset = offset;
            collider.isTrigger = isTrigger;
            collider.usedByEffector = usedByEffector;
            if (material != null)
            {
                collider.sharedMaterial = material;
            }
            if (usedByComposite)
            {
                collider.usedByComposite = true;
            }
        }

        private bool HasScenePhysics2DOnEntity(ulong entity)
        {
            return _sceneRigidbodies2D.ContainsKey(entity)
                || _sceneBoxColliders2D.ContainsKey(entity)
                || _sceneCircleColliders2D.ContainsKey(entity)
                || _sceneCapsuleColliders2D.ContainsKey(entity)
                || _scenePolygonColliders2D.ContainsKey(entity)
                || _sceneEdgeColliders2D.ContainsKey(entity)
                || _sceneCompositeColliders2D.ContainsKey(entity)
                || _sceneTilemapColliders2D.ContainsKey(entity);
        }

        private void RegisterScenePhysics2D(ulong entity, GameObject owner)
        {
            if (!HasScenePhysics2DOnEntity(entity) || _registeredBodies2D.Contains(entity))
            {
                return;
            }
            _registeredBodies2D.Add(entity);
            bool hasBody = _sceneRigidbodies2D.ContainsKey(entity);
            bool hasBox = _sceneBoxColliders2D.ContainsKey(entity);
            bool hasCircle = _sceneCircleColliders2D.ContainsKey(entity);
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
                (uint)owner.layer,
                hasBody && !bodyMetadata.Simulated ? 0u : 1u,
                hasBody ? (uint)bodyMetadata.Interpolation : 0u,
                hasBody ? (uint)bodyMetadata.CollisionDetectionMode : 0u);
            if (hasBody && bodyMetadata.Constraints != RigidbodyConstraints2D.None)
            {
                TrySetBodyConstraints2D(entity, bodyMetadata.Constraints);
            }
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
                    (uint)owner.layer);
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
                    (uint)owner.layer);
            }
            if (_sceneCapsuleColliders2D.TryGetValue(entity, out SceneCapsuleCollider2DMetadata capsuleMeta))
            {
                float radius = capsuleMeta.Direction != 0 ? capsuleMeta.Size.x * 0.5f : capsuleMeta.Size.y * 0.5f;
                float height = capsuleMeta.Direction != 0 ? capsuleMeta.Size.y : capsuleMeta.Size.x;
                TryCreateCapsuleCollider2D(entity, radius, height, capsuleMeta.Direction, capsuleMeta.Offset, capsuleMeta.IsTrigger, (uint)owner.layer);
            }
            if (_scenePolygonColliders2D.TryGetValue(entity, out ScenePolygonCollider2DMetadata polygonMeta) && polygonMeta.Points.Length >= 3)
            {
                TryCreatePolygonCollider2D(entity, entity, polygonMeta.Points, polygonMeta.Offset, polygonMeta.IsTrigger, (uint)owner.layer);
            }
            if (_sceneEdgeColliders2D.TryGetValue(entity, out SceneEdgeCollider2DMetadata edgeMeta) && edgeMeta.Points.Length >= 2)
            {
                TryCreateEdgeCollider2D(entity, entity, edgeMeta.Points, edgeMeta.Offset, edgeMeta.IsTrigger, (uint)owner.layer);
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

        private const int POLYGON_COLLIDER2D_CLASS = 60;
        private const int COMPOSITE_COLLIDER2D_CLASS = 66;
        private const int EDGE_COLLIDER2D_CLASS = 68;
        private const int CAPSULE_COLLIDER2D_CLASS = 70;
        private const int TILEMAP_COLLIDER2D_CLASS = 19719996;
        private const int TILEMAP_CLASS = 1839735485;

        private readonly Dictionary<ulong, SceneCapsuleCollider2DMetadata> _sceneCapsuleColliders2D = new Dictionary<ulong, SceneCapsuleCollider2DMetadata>();
        private readonly Dictionary<ulong, ScenePolygonCollider2DMetadata> _scenePolygonColliders2D = new Dictionary<ulong, ScenePolygonCollider2DMetadata>();
        private readonly Dictionary<ulong, SceneEdgeCollider2DMetadata> _sceneEdgeColliders2D = new Dictionary<ulong, SceneEdgeCollider2DMetadata>();
        private readonly Dictionary<ulong, SceneCompositeCollider2DMetadata> _sceneCompositeColliders2D = new Dictionary<ulong, SceneCompositeCollider2DMetadata>();
        private readonly Dictionary<ulong, SceneTilemapCollider2DMetadata> _sceneTilemapColliders2D = new Dictionary<ulong, SceneTilemapCollider2DMetadata>();

        private sealed class ScenePhysics2DFields
        {
            public int BodyType { get; set; }

            public bool Simulated { get; set; } = true;

            public float Mass { get; set; } = 1f;

            public float GravityScale { get; set; } = 1f;

            public int Interpolation { get; set; }

            public int CollisionDetectionMode { get; set; }

            public int Constraints { get; set; }

            public Vector2 Offset { get; set; }

            public Vector2 Size { get; set; } = Vector2.one;

            public Vector2 CapsuleSize { get; set; } = new Vector2(0.5f, 1f);

            public int CapsuleDirection { get; set; } = 1;

            public float Radius { get; set; } = 0.5f;

            public float EdgeRadius { get; set; }

            public bool IsTrigger { get; set; }

            public bool UsedByEffector { get; set; }

            public bool UsedByComposite { get; set; }

            public string MaterialFileId { get; set; } = string.Empty;

            public string MaterialGuid { get; set; } = string.Empty;

            public bool HasInlineMaterial { get; set; }

            public float Friction { get; set; } = 0.4f;

            public float Bounciness { get; set; }

            public List<Vector2> Points { get; } = new List<Vector2>();

            public int GeometryType { get; set; }

            public int GenerationType { get; set; }

            public float VertexDistance { get; set; } = 0.00005f;

            public float OffsetDistance { get; set; } = 0.000005f;

            public uint MaximumTileChangeCount { get; set; } = 1000u;

            public float ExtrusionFactor { get; set; }

            public string TilemapAssetPath { get; set; } = string.Empty;

            public string TilemapAssetGuid { get; set; } = string.Empty;

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
                        fields.CollisionDetectionMode,
                        fields.Constraints));
                    break;
                case BOX_COLLIDER2D_CLASS:
                    boxColliders2D.Add(new ParsedBoxCollider2DRecord(
                        gameObjectFileId,
                        fields.Size,
                        fields.Offset,
                        fields.IsTrigger,
                        fields.UsedByEffector,
                        fields.UsedByComposite,
                        fields.MaterialGuid,
                        fields.HasInlineMaterial,
                        fields.Friction,
                        fields.Bounciness));
                    break;
                case CIRCLE_COLLIDER2D_CLASS:
                    circleColliders2D.Add(new ParsedCircleCollider2DRecord(
                        gameObjectFileId,
                        fields.Radius,
                        fields.Offset,
                        fields.IsTrigger,
                        fields.UsedByEffector,
                        fields.UsedByComposite,
                        fields.MaterialGuid,
                        fields.HasInlineMaterial,
                        fields.Friction,
                        fields.Bounciness));
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
            public SceneRigidbody2DMetadata(RigidbodyType2D bodyType, bool simulated, float mass, float gravityScale, RigidbodyInterpolation2D interpolation, CollisionDetectionMode2D collisionDetectionMode, RigidbodyConstraints2D constraints = RigidbodyConstraints2D.None)
            {
                BodyType = bodyType;
                Simulated = simulated;
                Mass = mass;
                GravityScale = gravityScale;
                Interpolation = interpolation;
                CollisionDetectionMode = collisionDetectionMode;
                Constraints = constraints;
            }

            public RigidbodyType2D BodyType { get; }

            public bool Simulated { get; }

            public float Mass { get; }

            public float GravityScale { get; }

            public RigidbodyInterpolation2D Interpolation { get; }

            public CollisionDetectionMode2D CollisionDetectionMode { get; }

            public RigidbodyConstraints2D Constraints { get; }
        }

        private readonly struct SceneBoxCollider2DMetadata
        {
            public SceneBoxCollider2DMetadata(Vector2 size, Vector2 offset, bool isTrigger, bool usedByEffector = false, bool usedByComposite = false, PhysicsMaterial2D? material = null)
            {
                Size = size;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                Material = material;
            }

            public Vector2 Size { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public bool UsedByComposite { get; }

            public PhysicsMaterial2D? Material { get; }
        }

        private readonly struct SceneCircleCollider2DMetadata
        {
            public SceneCircleCollider2DMetadata(float radius, Vector2 offset, bool isTrigger, bool usedByEffector = false, bool usedByComposite = false, PhysicsMaterial2D? material = null)
            {
                Radius = radius;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                Material = material;
            }

            public float Radius { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public bool UsedByComposite { get; }

            public PhysicsMaterial2D? Material { get; }
        }

        private readonly struct SceneCapsuleCollider2DMetadata
        {
            public SceneCapsuleCollider2DMetadata(Vector2 size, int direction, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, PhysicsMaterial2D? material)
            {
                Size = size;
                Direction = direction;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                Material = material;
            }

            public Vector2 Size { get; }

            public int Direction { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public bool UsedByComposite { get; }

            public PhysicsMaterial2D? Material { get; }
        }

        private readonly struct ScenePolygonCollider2DMetadata
        {
            public ScenePolygonCollider2DMetadata(Vector2[] points, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, PhysicsMaterial2D? material)
            {
                Points = points;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                Material = material;
            }

            public Vector2[] Points { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public bool UsedByComposite { get; }

            public PhysicsMaterial2D? Material { get; }
        }

        private readonly struct SceneEdgeCollider2DMetadata
        {
            public SceneEdgeCollider2DMetadata(Vector2[] points, float edgeRadius, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, PhysicsMaterial2D? material)
            {
                Points = points;
                EdgeRadius = edgeRadius;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                Material = material;
            }

            public Vector2[] Points { get; }

            public float EdgeRadius { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public bool UsedByComposite { get; }

            public PhysicsMaterial2D? Material { get; }
        }

        private readonly struct SceneCompositeCollider2DMetadata
        {
            public SceneCompositeCollider2DMetadata(int geometryType, int generationType, float vertexDistance, float offsetDistance, float edgeRadius, Vector2 offset, bool isTrigger, bool usedByEffector, PhysicsMaterial2D? material)
            {
                GeometryType = geometryType;
                GenerationType = generationType;
                VertexDistance = vertexDistance;
                OffsetDistance = offsetDistance;
                EdgeRadius = edgeRadius;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                Material = material;
            }

            public int GeometryType { get; }

            public int GenerationType { get; }

            public float VertexDistance { get; }

            public float OffsetDistance { get; }

            public float EdgeRadius { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public PhysicsMaterial2D? Material { get; }
        }

        private readonly struct SceneTilemapCollider2DMetadata
        {
            public SceneTilemapCollider2DMetadata(uint maximumTileChangeCount, float extrusionFactor, string tilemapPath, Vector2 offset, bool isTrigger, bool usedByEffector, bool usedByComposite, PhysicsMaterial2D? material)
            {
                MaximumTileChangeCount = maximumTileChangeCount;
                ExtrusionFactor = extrusionFactor;
                TilemapPath = tilemapPath;
                Offset = offset;
                IsTrigger = isTrigger;
                UsedByEffector = usedByEffector;
                UsedByComposite = usedByComposite;
                Material = material;
            }

            public uint MaximumTileChangeCount { get; }

            public float ExtrusionFactor { get; }

            public string TilemapPath { get; }

            public Vector2 Offset { get; }

            public bool IsTrigger { get; }

            public bool UsedByEffector { get; }

            public bool UsedByComposite { get; }

            public PhysicsMaterial2D? Material { get; }
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
