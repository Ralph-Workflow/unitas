using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    [DefaultExecutionOrder(-100)]
    public sealed class LifecycleNegativeOrderScript : MonoBehaviour
    {
        public static List<string>? SharedTrace { get; set; }

        public string TagName { get; set; } = "Neg-100";

        public override int DefaultExecutionOrderValue => -100;

        public override void Awake() => SharedTrace?.Add(TagName + ":Awake");

        public override void OnEnable() => SharedTrace?.Add(TagName + ":OnEnable");

        public override void Start() => SharedTrace?.Add(TagName + ":Start");

        public override void FixedUpdate() => SharedTrace?.Add(TagName + ":FixedUpdate");

        public override void Update() => SharedTrace?.Add(TagName + ":Update");

        public override void LateUpdate() => SharedTrace?.Add(TagName + ":LateUpdate");
    }

    public sealed class LifecycleZeroOrderScript : MonoBehaviour
    {
        public static List<string>? SharedTrace { get; set; }

        public string TagName { get; set; } = "Zero0";

        public override int DefaultExecutionOrderValue => 0;

        public override void Awake() => SharedTrace?.Add(TagName + ":Awake");

        public override void OnEnable() => SharedTrace?.Add(TagName + ":OnEnable");

        public override void Start() => SharedTrace?.Add(TagName + ":Start");

        public override void FixedUpdate() => SharedTrace?.Add(TagName + ":FixedUpdate");

        public override void Update() => SharedTrace?.Add(TagName + ":Update");

        public override void LateUpdate() => SharedTrace?.Add(TagName + ":LateUpdate");
    }

    [DefaultExecutionOrder(100)]
    public sealed class LifecyclePositiveOrderScript : MonoBehaviour
    {
        public static List<string>? SharedTrace { get; set; }

        public string TagName { get; set; } = "Pos100";

        public override int DefaultExecutionOrderValue => 100;

        public override void Awake() => SharedTrace?.Add(TagName + ":Awake");

        public override void OnEnable() => SharedTrace?.Add(TagName + ":OnEnable");

        public override void Start() => SharedTrace?.Add(TagName + ":Start");

        public override void FixedUpdate() => SharedTrace?.Add(TagName + ":FixedUpdate");

        public override void Update() => SharedTrace?.Add(TagName + ":Update");

        public override void LateUpdate() => SharedTrace?.Add(TagName + ":LateUpdate");
    }

    public sealed class UnassignedFaultingRuntimeScript : MonoBehaviour
    {
        public GameObject? UnassignedTarget;

        public int AwakeCount;

        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public override void Awake()
        {
            AwakeCount++;
            UnassignedTarget = GameObject.CreateUnassignedReference(
                nameof(UnassignedTarget),
                nameof(UnassignedFaultingRuntimeScript));
        }

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
            _ = UnassignedTarget!.name;
        }
    }

    public sealed class SiblingHealthyRuntimeScript : MonoBehaviour
    {
        public int AwakeCount;

        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public int LateUpdateCount;

        public override void Awake()
        {
            AwakeCount++;
        }

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }

        public override void LateUpdate()
        {
            LateUpdateCount++;
        }
    }

    public sealed class EndOfFrameDestroyTargetScript : MonoBehaviour
    {
        public readonly List<string> Callbacks = new List<string>();

        public override void Awake() => Callbacks.Add("Awake");

        public override void OnEnable() => Callbacks.Add("OnEnable");

        public override void OnDisable() => Callbacks.Add("OnDisable");

        public override void OnDestroy() => Callbacks.Add("OnDestroy");
    }

    public static class LifecycleAcceptanceTests
    {
        private const string FirstScriptGuid = "11112222333344445555666677778888";
        private const string SecondScriptGuid = "22222222333344445555666677778888";
        private const string TwoScriptSceneFixture = "crates/unitas-runtime/tests/fixtures/scene_physics_query.unity";

        public static void Run()
        {
            TestStory11DefaultExecutionOrderNegativeZeroPositiveAndSettingOverride();
            TestStory39UnassignedReferenceExceptionNamesComponentAndFieldAndContainsFaultSoSiblingTicks();
            TestStory2DestroyedObjectNullTruthinessEndOfFrameVsDestroyImmediate();
        }

        private static void TestStory11DefaultExecutionOrderNegativeZeroPositiveAndSettingOverride()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            List<string> trace = new List<string>();
            LifecycleNegativeOrderScript.SharedTrace = trace;
            LifecycleZeroOrderScript.SharedTrace = trace;
            LifecyclePositiveOrderScript.SharedTrace = trace;
            MonoBehaviour.ClearExecutionOrderOverrides();
            MonoBehaviour.ClearRegisteredDefaultExecutionOrders();

            try
            {
                GameObject hostObject = Track(created, new GameObject("Story11_ExecutionOrderHost"));
                LifecyclePositiveOrderScript positive = hostObject.AddComponent<LifecyclePositiveOrderScript>();
                LifecycleZeroOrderScript zero = hostObject.AddComponent<LifecycleZeroOrderScript>();
                LifecycleNegativeOrderScript negative = hostObject.AddComponent<LifecycleNegativeOrderScript>();

                Check.Equal("Story 11 negative script default order is -100", -100, MonoBehaviour.GetEffectiveExecutionOrder(negative));
                Check.Equal("Story 11 zero script default order is 0", 0, MonoBehaviour.GetEffectiveExecutionOrder(zero));
                Check.Equal("Story 11 positive script default order is 100", 100, MonoBehaviour.GetEffectiveExecutionOrder(positive));

                MonoBehaviour[] batch = new MonoBehaviour[] { positive, zero, negative };
                MonoBehaviour.RunLifecycleBatch(batch, runFixedUpdate: true);

                Check.Equal(
                    "Story 11 [DefaultExecutionOrder] orders Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate across negative, zero, and positive orders",
                    "Neg-100:Awake, Zero0:Awake, Pos100:Awake, "
                    + "Neg-100:OnEnable, Zero0:OnEnable, Pos100:OnEnable, "
                    + "Neg-100:Start, Zero0:Start, Pos100:Start, "
                    + "Neg-100:FixedUpdate, Zero0:FixedUpdate, Pos100:FixedUpdate, "
                    + "Neg-100:Update, Zero0:Update, Pos100:Update, "
                    + "Neg-100:LateUpdate, Zero0:LateUpdate, Pos100:LateUpdate",
                    string.Join(", ", trace));

                trace.Clear();
                MonoBehaviour.SetExecutionOrderOverride<LifecyclePositiveOrderScript>(-250);
                MonoBehaviour.SetExecutionOrderOverride<LifecycleNegativeOrderScript>(250);

                Check.Equal("Story 11 setting overrides positive script order to -250", -250, MonoBehaviour.GetEffectiveExecutionOrder(positive));
                Check.Equal("Story 11 setting overrides negative script order to 250", 250, MonoBehaviour.GetEffectiveExecutionOrder(negative));

                MonoBehaviour.RunLifecycleBatch(batch, runFixedUpdate: true);

                Check.Equal(
                    "Story 11 Script Execution Order setting overrides [DefaultExecutionOrder] across all lifecycle phases",
                    "Pos100:Awake, Zero0:Awake, Neg-100:Awake, "
                    + "Pos100:OnEnable, Zero0:OnEnable, Neg-100:OnEnable, "
                    + "Pos100:Start, Zero0:Start, Neg-100:Start, "
                    + "Pos100:FixedUpdate, Zero0:FixedUpdate, Neg-100:FixedUpdate, "
                    + "Pos100:Update, Zero0:Update, Neg-100:Update, "
                    + "Pos100:LateUpdate, Zero0:LateUpdate, Neg-100:LateUpdate",
                    string.Join(", ", trace));
            }
            finally
            {
                LifecycleNegativeOrderScript.SharedTrace = null;
                LifecycleZeroOrderScript.SharedTrace = null;
                LifecyclePositiveOrderScript.SharedTrace = null;
                MonoBehaviour.ClearExecutionOrderOverrides();
                MonoBehaviour.ClearRegisteredDefaultExecutionOrders();
                DestroyAll(created);
            }
        }

        private static void TestStory39UnassignedReferenceExceptionNamesComponentAndFieldAndContainsFaultSoSiblingTicks()
        {
            string scenePath = ResolveFixture(TwoScriptSceneFixture);
            Check.That("Story 39 two-script scene fixture exists", File.Exists(scenePath));

            LogRecorder.Clear();
            ScriptGlueRegistry.Clear();
            UnityEngine.Object.ForcePlayerReferenceExceptions = false;

            UnassignedFaultingRuntimeScript faulting = new UnassignedFaultingRuntimeScript();
            SiblingHealthyRuntimeScript healthySibling = new SiblingHealthyRuntimeScript();

            ScriptGlueRegistry.Register(FirstScriptGuid, () => faulting);
            ScriptGlueRegistry.Register(SecondScriptGuid, () => healthySibling);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Tick(0.02);
                    host.Tick(0.02);

                    Check.Equal("Story 39 faulting script ran Awake once", 1, faulting.AwakeCount);
                    Check.Equal("Story 39 faulting script ran OnEnable once", 1, faulting.EnableCount);
                    Check.Equal("Story 39 faulting script ran Start once", 1, faulting.StartCount);
                    Check.Equal("Story 39 faulting script attempted Update on both update frames", 2, faulting.UpdateCount);

                    Check.Equal("Story 39 healthy sibling ran Awake once", 1, healthySibling.AwakeCount);
                    Check.Equal("Story 39 healthy sibling ran OnEnable once", 1, healthySibling.EnableCount);
                    Check.Equal("Story 39 healthy sibling ran Start once", 1, healthySibling.StartCount);
                    Check.Equal("Story 39 healthy sibling continued ticking Update despite sibling UnassignedReferenceException", 2, healthySibling.UpdateCount);
                    Check.Equal("Story 39 healthy sibling continued ticking LateUpdate despite sibling UnassignedReferenceException", 2, healthySibling.LateUpdateCount);

                    IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                    Check.Equal("Story 39 each faulting Update logged one error entry", 2, errors.Count);
                    for (int i = 0; i < errors.Count; i++)
                    {
                        LogEntry entry = errors[i];
                        Check.That(
                            "Story 39 logged UnassignedReferenceException names field UnassignedTarget",
                            entry.Message.Contains(nameof(UnassignedFaultingRuntimeScript.UnassignedTarget), StringComparison.Ordinal));
                        Check.That(
                            "Story 39 logged UnassignedReferenceException names component UnassignedFaultingRuntimeScript",
                            entry.Message.Contains(nameof(UnassignedFaultingRuntimeScript), StringComparison.Ordinal));
                        Check.That(
                            "Story 39 logged UnassignedReferenceException carries faulting script as Origin",
                            ReferenceEquals(entry.Origin, faulting));
                    }

                    bool directThrewUnassigned = false;
                    string directMessage = string.Empty;
                    try
                    {
                        _ = faulting.UnassignedTarget!.name;
                    }
                    catch (UnassignedReferenceException ex)
                    {
                        directThrewUnassigned = true;
                        directMessage = ex.Message;
                    }

                    Check.That("Story 39 direct access to unassigned reference in editor session throws UnassignedReferenceException", directThrewUnassigned);
                    Check.That(
                        "Story 39 UnassignedReferenceException message names field and component",
                        directMessage.Contains(nameof(UnassignedFaultingRuntimeScript.UnassignedTarget), StringComparison.Ordinal)
                        && directMessage.Contains(nameof(UnassignedFaultingRuntimeScript), StringComparison.Ordinal));

                    host.BuildKind = ApplicationBuildKind.ReleasePlayer;
                    bool playerThrewNullRef = false;
                    try
                    {
                        _ = faulting.UnassignedTarget!.name;
                    }
                    catch (NullReferenceException)
                    {
                        playerThrewNullRef = true;
                    }

                    Check.That("Story 39 accessing unassigned serialized reference in player session raises NullReferenceException", playerThrewNullRef);

                    host.Shutdown();
                }
            }
            finally
            {
                UnityEngine.Object.ForcePlayerReferenceExceptions = false;
                LogRecorder.Clear();
            }
        }

        private static void TestStory2DestroyedObjectNullTruthinessEndOfFrameVsDestroyImmediate()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject sourceGo = Track(created, new GameObject("Story2_Original"));
                EndOfFrameDestroyTargetScript sourceComp = sourceGo.AddComponent<EndOfFrameDestroyTargetScript>();
                sourceGo.transform.localPosition = new Vector3(3f, 6f, 9f);

                GameObject clonedGo = Track(created, UnityEngine.Object.Instantiate(sourceGo));
                EndOfFrameDestroyTargetScript? clonedComp = clonedGo.GetComponent<EndOfFrameDestroyTargetScript>();
                Check.That("Story 2 cloned GameObject has distinct instance ID", clonedGo.GetInstanceID() != sourceGo.GetInstanceID());
                Check.That("Story 2 cloned Component exists and has distinct instance ID", clonedComp != null && clonedComp.GetInstanceID() != sourceComp.GetInstanceID());

                GameObject immediateTarget = new GameObject("Story2_ImmediateTarget");
                EndOfFrameDestroyTargetScript immediateScript = immediateTarget.AddComponent<EndOfFrameDestroyTargetScript>();

                Check.That("Story 2 live object is not == null", !(immediateTarget == null));
                Check.That("Story 2 live object is != null", immediateTarget != null);
                Check.That("Story 2 live object is truthy in bool conversion", (bool)immediateTarget);
                Check.That("Story 2 !liveObj is false", !(!immediateTarget));

                UnityEngine.Object.DestroyImmediate(immediateTarget);

                Check.That("Story 2 DestroyImmediate makes object == null immediately", immediateTarget == null);
                Check.That("Story 2 DestroyImmediate makes !(obj != null) true", !(immediateTarget != null));
                Check.That("Story 2 DestroyImmediate makes !obj truthy (falsy object)", !immediateTarget);
                Check.That("Story 2 DestroyImmediate makes owned component == null", immediateScript == null);
                Check.That("Story 2 DestroyImmediate makes !component true", !immediateScript);
                Check.Equal(
                    "Story 2 DestroyImmediate runs OnDisable then OnDestroy",
                    "OnDisable, OnDestroy",
                    string.Join(", ", immediateScript.Callbacks));
                Check.That(
                    "Story 2 C# ReferenceEquals still observes non-null managed reference after DestroyImmediate",
                    !ReferenceEquals(immediateTarget, null));
                int? managedInstanceIdViaNullConditional = immediateTarget?.GetInstanceID();
                Check.That(
                    "Story 2 C# ?. operator still observes the managed reference while Unity == compares equal to null",
                    managedInstanceIdViaNullConditional.HasValue);

                GameObject frameDestroyTarget = new GameObject("Story2_EndOfFrameTarget");
                EndOfFrameDestroyTargetScript frameDestroyScript = frameDestroyTarget.AddComponent<EndOfFrameDestroyTargetScript>();

                UnityEngine.Object.Destroy(frameDestroyTarget);

                Check.That("Story 2 Destroy marks retained GameObject reference == null", frameDestroyTarget == null);
                Check.That("Story 2 Destroy marks retained GameObject reference falsy (!obj)", !frameDestroyTarget);
                Check.That("Story 2 Destroy marks retained Component reference == null", frameDestroyScript == null);
                Check.That("Story 2 Destroy marks retained Component reference falsy (!comp)", !frameDestroyScript);
                Check.Equal("Story 2 destroyed object ToString() is 'null'", "null", frameDestroyTarget.ToString());
                Check.Equal(
                    "Story 2 Destroy runs OnDisable then OnDestroy on component",
                    "OnDisable, OnDestroy",
                    string.Join(", ", frameDestroyScript.Callbacks));
                Check.That(
                    "Story 2 C# ?. operator observes managed reference on Destroyed object",
                    (frameDestroyTarget?.GetInstanceID()).HasValue);

                bool missingRefOnDestroyed = false;
                try
                {
                    _ = frameDestroyTarget.name;
                }
                catch (MissingReferenceException)
                {
                    missingRefOnDestroyed = true;
                }

                Check.That("Story 2 accessing member on destroyed object throws MissingReferenceException", missingRefOnDestroyed);
                Check.That("Story 2 destroyed object is excluded from FindObjectsOfType", UnityEngine.Object.FindObjectOfType<EndOfFrameDestroyTargetScript>(true) == clonedComp || UnityEngine.Object.FindObjectOfType<EndOfFrameDestroyTargetScript>(true) == sourceComp);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static GameObject Track(List<UnityEngine.Object> list, GameObject instance)
        {
            list.Add(instance);
            return instance;
        }

        private static void DestroyAll(List<UnityEngine.Object> list)
        {
            for (int i = list.Count - 1; i >= 0; i--)
            {
                UnityEngine.Object.Destroy(list[i]);
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
