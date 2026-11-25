using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class LifetimeProbeScript : MonoBehaviour
    {
        private readonly System.Collections.Generic.List<string> _events = new System.Collections.Generic.List<string>();

        public string ObservedOrder => string.Join(", ", _events);

        public override void OnDisable()
        {
            _events.Add("OnDisable");
        }

        public override void OnDestroy()
        {
            _events.Add("OnDestroy");
        }
    }

    public sealed class DestroyOriginProbeScript : MonoBehaviour
    {
        public override void OnDisable()
        {
            UnityEngine.Debug.Log("destroy origin probe disable");
        }

        public override void OnDestroy()
        {
            UnityEngine.Debug.Log("destroy origin probe destroy");
        }
    }

    public static class LifetimeTests
    {
        public static void Run()
        {
            GameObject live = new GameObject("Live");
            Check.That("live object is not null", live != null);
            Check.That("live object is truthy", (bool)live);
            Check.That("live object is not equal to null", !(live == null));
            Check.That("live object equals itself", live!.Equals(live));
            Check.That("live object does not equal null", !live.Equals(null));
            Check.That("two live objects are different", !new GameObject("Other").Equals(live));

            Object? realNull = null;
            Check.That("null equals null", realNull == null);
            Check.That("null differs from a live object", realNull != live);

            GameObject doomed = new GameObject("Doomed");
            Object.Destroy(doomed);
            Check.That("destroyed object equals null", doomed == null);
            Check.That("destroyed object does not differ from null", !(doomed != null));
            Check.That("destroyed object is falsy", !(bool)doomed);
            Check.That("destroyed object equals null through Equals", doomed!.Equals(null));
            Check.Equal("destroyed object stringifies as null", "null", doomed!.ToString());
            Check.That("live object does not equal destroyed object", !(live == doomed));
            Check.That("destroyed object does not equal live object", !(doomed == live));
            Check.That("live object differs from destroyed object", live != doomed);
            Check.That("destroyed object differs from live object", doomed != live);
            Check.That("live object does not equal destroyed object through Equals", !live!.Equals(doomed));
            Check.That("destroyed object does not equal live object through Equals", !doomed.Equals(live));

            GameObject secondDoomed = new GameObject("SecondDoomed");
            Object.Destroy(secondDoomed);
            Check.That("two destroyed objects compare equal", doomed == secondDoomed);
            Check.That("two destroyed objects do not differ", !(doomed != secondDoomed));
            Check.That("two destroyed objects compare equal through Equals", doomed.Equals(secondDoomed));

            bool threw = false;
            try
            {
                string inaccessible = doomed.name;
            }
            catch (MissingReferenceException)
            {
                threw = true;
            }

            Check.That("destroyed object throws MissingReferenceException on name access", threw);

            GameObject probeOwner = new GameObject("ProbeOwner");
            LifetimeProbeScript probe = probeOwner.AddComponent<LifetimeProbeScript>();
            Object.Destroy(probe);
            Check.That("destroyed script equals null", probe == null);
            Check.That("destroyed script is falsy", !(bool)probe);
            Check.Equal("destroyed script fires OnDisable then OnDestroy", "OnDisable, OnDestroy", probe!.ObservedOrder);
            Object.Destroy(probe);
            Check.Equal("double destroy is a no-op", "OnDisable, OnDestroy", probe.ObservedOrder);
            Object.Destroy(probeOwner);

            GameObject owner = new GameObject("Owner");
            LifetimeProbeScript owned = owner.AddComponent<LifetimeProbeScript>();
            Object.Destroy(owner);
            Check.That("destroyed owner equals null", owner == null);
            Check.Equal("destroyed owner fires component OnDisable then OnDestroy", "OnDisable, OnDestroy", owned.ObservedOrder);
            Check.That("destroyed component equals null", owned == null);

            DestroyOriginIsRecorded();
        }

        private static void DestroyOriginIsRecorded()
        {
            LogRecorder.Clear();
            GameObject sentinel = new GameObject("DestroyOriginSentinel");
            GameObject probeOwner = new GameObject("DestroyOriginProbeOwner");
            DestroyOriginProbeScript probe = probeOwner.AddComponent<DestroyOriginProbeScript>();
            LogRecorder.CurrentOrigin = sentinel;
            Object.Destroy(probe);
            Check.That("destroy restores the previous log origin", ReferenceEquals(LogRecorder.CurrentOrigin, sentinel));
            LogRecorder.CurrentOrigin = null;

            System.Collections.Generic.IReadOnlyList<LogEntry> entries = LogRecorder.Search("destroy origin probe");
            Check.Equal("destroy probe recorded two entries", 2, entries.Count);
            Check.That(
                "destroy callbacks log with the destroyed script as origin",
                entries.Count == 2
                && ReferenceEquals(entries[0].Origin, probe)
                && ReferenceEquals(entries[1].Origin, probe));
            Object.Destroy(probeOwner);
            Object.Destroy(sentinel);
        }
    }
}
