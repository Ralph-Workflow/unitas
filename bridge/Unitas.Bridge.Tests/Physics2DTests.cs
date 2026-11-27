using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class Physics2DRecorderScript : MonoBehaviour
    {
        private readonly List<string> _events = new List<string>();
        private int _frame;

        public IReadOnlyList<string> Events => _events;

        public string ObservedOrder => string.Join(", ", _events);

        public int CollisionContactCount { get; private set; }

        public string CollisionOtherName { get; private set; } = string.Empty;

        public ContactPoint2D CollisionContact { get; private set; }

        public override void Update()
        {
            _frame++;
            _events.Add("Update@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        public override void OnCollisionEnter(Collision collision)
        {
            _events.Add("OnCollisionEnter@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        public override void OnCollisionEnter2D(Collision2D collision)
        {
            _events.Add("OnCollisionEnter2D@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
            CollisionContactCount = collision.contactCount;
            CollisionOtherName = collision.gameObject?.name ?? string.Empty;
            if (collision.contactCount > 0)
            {
                CollisionContact = collision.GetContact(0);
            }
        }

        public override void OnTriggerEnter(Collider? other)
        {
            _events.Add("OnTriggerEnter@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        public override void OnTriggerEnter2D(Collider2D? other)
        {
            _events.Add("OnTriggerEnter2D@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }
    }

    public static class Physics2DTests
    {
        public static void Run()
        {
            TestTwoDimensionalContactsReachTheTwoDimensionalHandlers();
            TestBoxCastReachesTheCollidersItSweeps();
            TestRigidbodyConstraintsReachTheNativeBody();
            TestCapsuleColliderCreatesANativeCapsuleShape();
        }

        private static void TestTwoDimensionalContactsReachTheTwoDimensionalHandlers()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");
            Check.That("Physics2D scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            Physics2DRecorderScript recorded = new Physics2DRecorderScript();
            ScriptGlueRegistry.Register(0x2d2d2d2d2d2d2d2dUL, 0x2d2d2d2d2d2d2d2dUL, () => recorded);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            IReadOnlyList<string> events = recorded.Events;
            Console.WriteLine("Observed 2D order: " + recorded.ObservedOrder);

            int triggerIndex = IndexOfEvent(events, "OnTriggerEnter2D");
            int collisionIndex = IndexOfEvent(events, "OnCollisionEnter2D");

            Check.That("2D trigger reaches the OnTriggerEnter2D handler", triggerIndex >= 0);
            Check.That("2D collision reaches the OnCollisionEnter2D handler", collisionIndex >= 0);
            Check.That("2D trigger never reaches the 3D OnTriggerEnter handler", IndexOfEvent(events, "OnTriggerEnter") < 0);
            Check.That("2D collision never reaches the 3D OnCollisionEnter handler", IndexOfEvent(events, "OnCollisionEnter") < 0);
            Check.That(
                "2D trigger reaches the handler before the 2D collision",
                triggerIndex >= 0 && collisionIndex > triggerIndex);
            Check.That(
                "2D trigger is dispatched after the physics step and before the frame update",
                IsDispatchedBeforeTheNextUpdate(events, triggerIndex));
            Check.That(
                "2D collision is dispatched after the physics step and before the frame update",
                IsDispatchedBeforeTheNextUpdate(events, collisionIndex));
            Check.That(
                "2D collision carries the contact points of the 2D world",
                recorded.CollisionContactCount > 0);
            Check.That(
                "2D collision resolves the other game object",
                recorded.CollisionOtherName == "Crate2D");
            Check.That(
                "2D collision contact point sits on the crate surface",
                Math.Abs(recorded.CollisionContact.point.y - 0.5f) < 0.25f);
        }

        private static void TestBoxCastReachesTheCollidersItSweeps()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);

                RaycastHit2D[] crateHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D());
                Check.Equal("the box cast reports the crate it reaches", 1, crateHits.Length);
                Check.Equal("the box cast hit resolves the crate collider", "Crate2D", HitName(crateHits, 0));
                Check.That(
                    "the box cast stops when the box reaches the crate face",
                    crateHits.Length > 0 && Math.Abs(crateHits[0].distance - 2.75f) < 0.05f);
                Check.That(
                    "the box cast normal points back at the caster",
                    crateHits.Length > 0 && crateHits[0].normal.x < -0.99f && Math.Abs(crateHits[0].normal.y) < 0.01f);

                ContactFilter2D sideFilter = new ContactFilter2D
                {
                    useOutsideNormalAngle = true,
                    minNormalAngle = 90f,
                    maxNormalAngle = 270f,
                };
                RaycastHit2D[] sideHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    sideFilter);
                Check.Equal("a normal-angle filter that holds the hit normal keeps the hit", 1, sideHits.Length);

                ContactFilter2D upwardFilter = new ContactFilter2D
                {
                    useOutsideNormalAngle = true,
                    minNormalAngle = 0f,
                    maxNormalAngle = 90f,
                };
                RaycastHit2D[] upwardHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    upwardFilter);
                Check.Equal("a normal-angle filter that excludes the hit normal drops the hit", 0, upwardHits.Length);

                RaycastHit2D[] unfilteredHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D());
                Check.Equal("a filter without a normal angle keeps the hit", 1, unfilteredHits.Length);

                RaycastHit2D[] maskedHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D(),
                    0);
                Check.Equal("a layer mask that excludes the crate layer drops the hit", 0, maskedHits.Length);

                ContactFilter2D triggerFilter = new ContactFilter2D
                {
                    useTriggers = true,
                };
                RaycastHit2D[] triggerHits = Physics2D.BoxCast(
                    new Vector2(-5f, 1.5f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    triggerFilter);
                Check.Equal("a trigger-inclusive filter reports the trigger collider", 1, triggerHits.Length);
                Check.Equal("the trigger hit resolves the trigger collider", "Trigger2D", HitName(triggerHits, 0));

                RaycastHit2D[] solidHits = Physics2D.BoxCast(
                    new Vector2(-5f, 1.5f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D());
                Check.Equal("a trigger-exclusive filter drops the trigger collider", 0, solidHits.Length);
            }
        }

        private static void TestRigidbodyConstraintsReachTheNativeBody()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                Rigidbody2D? body = FindSceneRigidbody2D();
                Check.That("the scene rigidbody is materialized", body != null);
                if (body == null)
                {
                    return;
                }

                body.freezeRotation = true;
                Check.That("freezeRotation reads back as frozen", body.freezeRotation);
                Check.Equal(
                    "the freeze-rotation constraint carries the Unity bit",
                    4,
                    (int)(body.constraints & RigidbodyConstraints2D.FreezeRotation));
                Check.That(
                    "the native body accepts the freeze-rotation constraint",
                    Physics2D.Provider != null
                    && Physics2D.Provider.TrySetBodyConstraints2D(
                        body.gameObject.NativeEntityId,
                        RigidbodyConstraints2D.FreezeRotation));

                body.constraints = RigidbodyConstraints2D.None;
                Check.That("clearing the constraints clears freezeRotation", !body.freezeRotation);

                body.constraints = RigidbodyConstraints2D.FreezeAll;
                Check.That("the freeze-all constraints freeze the rotation", body.freezeRotation);
                Check.That(
                    "the freeze-all constraints keep the position bits",
                    (body.constraints & RigidbodyConstraints2D.FreezePosition) == RigidbodyConstraints2D.FreezePosition);
                Check.That(
                    "the freeze-all constraints reach the native body",
                    Physics2D.Provider != null
                    && Physics2D.Provider.TrySetBodyConstraints2D(
                        body.gameObject.NativeEntityId,
                        RigidbodyConstraints2D.FreezeAll));
            }
        }

        private static void TestCapsuleColliderCreatesANativeCapsuleShape()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                Rigidbody2D? body = FindSceneRigidbody2D();
                Check.That("the scene rigidbody is materialized", body != null);
                if (body == null)
                {
                    return;
                }

                Check.Equal(
                    "the box collider of the ball does not reach the capsule cap",
                    0,
                    Physics2D.OverlapPoint(new Vector2(0f, 3.8f)).Length);

                CapsuleCollider2D? capsule = body.gameObject.AddComponent<CapsuleCollider2D>();
                Check.That("the capsule collider is attached to the ball", capsule != null);
                if (capsule == null)
                {
                    return;
                }
                capsule.size = new Vector2(0.5f, 2f);

                Check.Equal(
                    "the native capsule collider reaches the capsule cap",
                    1,
                    Physics2D.OverlapPoint(new Vector2(0f, 3.8f)).Length);
                Check.Equal(
                    "the native capsule collider does not widen the ball",
                    0,
                    Physics2D.OverlapPoint(new Vector2(0.4f, 3.8f)).Length);
            }
        }

        private static Rigidbody2D? FindSceneRigidbody2D()
        {
            Collider2D[] colliders = Physics2D.OverlapPoint(new Vector2(0f, 3f));
            for (int index = 0; index < colliders.Length; index++)
            {
                Rigidbody2D? body = colliders[index].gameObject.GetComponent<Rigidbody2D>();
                if (body != null)
                {
                    return body;
                }
            }
            return null;
        }

        private static string HitName(RaycastHit2D[] hits, int index)
        {
            if (index >= hits.Length)
            {
                return string.Empty;
            }
            Collider2D? collider = hits[index].collider;
            return collider == null ? string.Empty : collider.gameObject.name;
        }

        private static int IndexOfEvent(IReadOnlyList<string> events, string name)
        {
            for (int index = 0; index < events.Count; index++)
            {
                if (events[index].StartsWith(name + "@", StringComparison.Ordinal))
                {
                    return index;
                }
            }
            return -1;
        }

        private static bool IsDispatchedBeforeTheNextUpdate(IReadOnlyList<string> events, int eventIndex)
        {
            if (eventIndex < 0 || eventIndex + 1 >= events.Count)
            {
                return false;
            }
            int frame = FrameOf(events[eventIndex]);
            if (frame < 0)
            {
                return false;
            }
            string next = events[eventIndex + 1];
            if (!next.StartsWith("Update@", StringComparison.Ordinal))
            {
                return false;
            }
            return FrameOf(next) == frame + 1;
        }

        private static int FrameOf(string entry)
        {
            int separator = entry.LastIndexOf('@');
            if (separator < 0 || separator + 1 >= entry.Length)
            {
                return -1;
            }
            return int.TryParse(
                entry.AsSpan(separator + 1),
                System.Globalization.NumberStyles.Integer,
                System.Globalization.CultureInfo.InvariantCulture,
                out int frame)
                ? frame
                : -1;
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
