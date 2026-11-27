using System.Collections.Generic;
using Unitas.Bridge;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class ScriptableObjectProbe : ScriptableObject
    {
        private readonly List<string> _events = new List<string>();

        public IReadOnlyList<string> Events => _events;

        public string ObservedOrder => string.Join(", ", _events);

        public override void Awake()
        {
            _events.Add("Awake");
        }

        public override void OnEnable()
        {
            _events.Add("OnEnable");
        }

        public override void OnValidate()
        {
            _events.Add("OnValidate");
        }

        public override void OnDisable()
        {
            _events.Add("OnDisable");
        }

        public override void OnDestroy()
        {
            _events.Add("OnDestroy");
        }
    }

    public static class ScriptableObjectTests
    {
        public static int Run()
        {
            int failuresBefore = Check.FailureCount;
            TestScriptableObjectCreateInstanceLifecycle();
            TestScriptableObjectDestroyFakeNullAndDoubleDestroy();
            TestScriptableObjectDestroyThroughObjectReference();
            return Check.FailureCount - failuresBefore;
        }

        private static void TestScriptableObjectCreateInstanceLifecycle()
        {
            using (BridgeHost host = new BridgeHost())
            {
                ScriptableObject.Provider = new ScriptableObjectProvider(host.Handle);
                ScriptableObjectProbe probe = ScriptableObject.CreateInstance<ScriptableObjectProbe>();

                Check.That("ScriptableObject create returns a live instance", probe != null);
                Check.Equal("ScriptableObject instance keeps its type name", "ScriptableObjectProbe", probe!.name);
                Check.Equal(
                    "ScriptableObject receives Awake, OnEnable, and OnValidate in order",
                    "Awake, OnEnable, OnValidate",
                    probe!.ObservedOrder);
                Check.That("ScriptableObject instance is held by the native world", probe!.NativeInstanceId != 0UL);

                ScriptableObject.Destroy(probe);
                ScriptableObject.Provider = null;
                host.Shutdown();
            }
        }

        private static void TestScriptableObjectDestroyThroughObjectReference()
        {
            using (BridgeHost host = new BridgeHost())
            {
                ScriptableObjectProvider provider = new ScriptableObjectProvider(host.Handle);
                ScriptableObject.Provider = provider;

                ScriptableObjectProbe probe = ScriptableObject.CreateInstance<ScriptableObjectProbe>();
                ulong native = probe!.NativeInstanceId;

                Object reference = probe;
                Object.Destroy(reference);

                Check.Equal(
                    "A ScriptableObject destroyed through Object.Destroy receives OnDisable and then OnDestroy",
                    "Awake, OnEnable, OnValidate, OnDisable, OnDestroy",
                    probe!.ObservedOrder);
                Check.That(
                    "A ScriptableObject destroyed through Object.Destroy compares equal to null",
                    probe == null);
                Check.That(
                    "Object.Destroy releases the native ScriptableObject instance",
                    !provider.IsAlive(native));

                ScriptableObject.Provider = null;
                host.Shutdown();
            }
        }

        private static void TestScriptableObjectDestroyFakeNullAndDoubleDestroy()
        {
            using (BridgeHost host = new BridgeHost())
            {
                ScriptableObjectProvider provider = new ScriptableObjectProvider(host.Handle);
                ScriptableObject.Provider = provider;

                ScriptableObjectProbe probe = ScriptableObject.CreateInstance<ScriptableObjectProbe>();
                ulong native = probe!.NativeInstanceId;
                Check.That("ScriptableObject instance is held by the native world", native != 0UL);
                Check.That("The native world holds the ScriptableObject instance", provider.IsAlive(native));

                ScriptableObject.Destroy(probe);

                Check.Equal(
                    "Destroyed ScriptableObject receives OnDisable and then OnDestroy",
                    "Awake, OnEnable, OnValidate, OnDisable, OnDestroy",
                    probe!.ObservedOrder);
                Check.That("Destroyed ScriptableObject compares equal to null", probe == null);
                Check.That("Destroyed ScriptableObject does not compare unequal to null", !(probe != null));
                Check.That("Destroyed ScriptableObject does not convert to true", !(bool)probe);
                Check.That("Destroyed ScriptableObject keeps its managed reference", probe is not null);
                Check.That("The native world drops the destroyed ScriptableObject instance", !provider.IsAlive(native));

                ScriptableObject.Destroy(probe);
                Check.Equal(
                    "A second ScriptableObject destroy does not repeat the destroy callbacks",
                    "Awake, OnEnable, OnValidate, OnDisable, OnDestroy",
                    probe!.ObservedOrder);

                ScriptableObject.Provider = null;
                host.Shutdown();
            }
        }
    }
}
