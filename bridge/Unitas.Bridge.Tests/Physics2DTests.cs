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
