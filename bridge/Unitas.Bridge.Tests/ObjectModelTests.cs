using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class UnassignedRefProbeBehaviour : MonoBehaviour
    {
        public GameObject? TargetObject;

        public Transform? TargetTransform;
    }

    public sealed class RequiredDependencyComponent : MonoBehaviour
    {
        public int Marker { get; set; } = 10;
    }

    public sealed class SecondaryRequiredDependencyComponent : MonoBehaviour
    {
        public int Marker { get; set; } = 20;
    }

    [RequireComponent(typeof(RequiredDependencyComponent), typeof(SecondaryRequiredDependencyComponent))]
    public sealed class DependentProbeComponent : MonoBehaviour
    {
        public override IReadOnlyList<Type> RequiredComponents =>
            new[] { typeof(RequiredDependencyComponent), typeof(SecondaryRequiredDependencyComponent) };
    }

    [DisallowMultipleComponent]
    public sealed class SingleInstanceProbeComponent : MonoBehaviour
    {
        public override bool DisallowMultiple => true;
    }

    [DefaultExecutionOrder(50)]
    public sealed class LateOrderedProbeScript : MonoBehaviour
    {
        public static List<string>? SharedLog { get; set; }

        public string Label { get; set; } = "Late50";

        public override int DefaultExecutionOrderValue => 50;

        public override void Awake() => SharedLog?.Add(Label + ":Awake");

        public override void OnEnable() => SharedLog?.Add(Label + ":OnEnable");

        public override void Start() => SharedLog?.Add(Label + ":Start");

        public override void FixedUpdate() => SharedLog?.Add(Label + ":FixedUpdate");

        public override void Update() => SharedLog?.Add(Label + ":Update");

        public override void LateUpdate() => SharedLog?.Add(Label + ":LateUpdate");
    }

    [DefaultExecutionOrder(-50)]
    public sealed class EarlyOrderedProbeScript : MonoBehaviour
    {
        public static List<string>? SharedLog { get; set; }

        public string Label { get; set; } = "Early-50";

        public override int DefaultExecutionOrderValue => -50;

        public override void Awake() => SharedLog?.Add(Label + ":Awake");

        public override void OnEnable() => SharedLog?.Add(Label + ":OnEnable");

        public override void Start() => SharedLog?.Add(Label + ":Start");

        public override void FixedUpdate() => SharedLog?.Add(Label + ":FixedUpdate");

        public override void Update() => SharedLog?.Add(Label + ":Update");

        public override void LateUpdate() => SharedLog?.Add(Label + ":LateUpdate");
    }

    public sealed class DefaultOrderedProbeScript : MonoBehaviour
    {
        public static List<string>? SharedLog { get; set; }

        public string Label { get; set; } = "Default0";

        public override void Awake() => SharedLog?.Add(Label + ":Awake");

        public override void OnEnable() => SharedLog?.Add(Label + ":OnEnable");

        public override void Start() => SharedLog?.Add(Label + ":Start");

        public override void FixedUpdate() => SharedLog?.Add(Label + ":FixedUpdate");

        public override void Update() => SharedLog?.Add(Label + ":Update");

        public override void LateUpdate() => SharedLog?.Add(Label + ":LateUpdate");
    }

    public static class ObjectModelTests
    {
        public static void Run() => RunAll();

        public static void RunAll()
        {
            TestStory39CompareTagUndefinedTagRaisesUnityException();
            TestStory39UnassignedSerializedReferenceRaisesUnassignedOrNullException();
            TestStory12RequireComponentAutoAddsAndRefusesDependencyRemoval();
            TestStory12DisallowMultipleComponentRefusesDuplicate();
            TestStory11DefaultExecutionOrderAndSettingOverride();
        }

        private static void TestStory39CompareTagUndefinedTagRaisesUnityException()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            GameObject.ResetDefinedTags();
            try
            {
                GameObject go = Track(created, new GameObject("TaggedActor"));
                Check.That("CompareTag('Untagged') returns true on default GameObject", go.CompareTag("Untagged"));
                Check.That("CompareTag('Player') returns false for built-in Player tag when Untagged", !go.CompareTag("Player"));
                Check.That("Component.CompareTag('Untagged') returns true", go.transform.CompareTag("Untagged"));

                bool compareUndefinedThrew = false;
                string compareUndefinedMessage = string.Empty;
                try
                {
                    go.CompareTag("CompletelyUndefinedTag_Story39");
                }
                catch (UnityException ex)
                {
                    compareUndefinedThrew = true;
                    compareUndefinedMessage = ex.Message;
                }

                Check.That("Story 39: GameObject.CompareTag with undefined tag raises UnityException", compareUndefinedThrew);
                Check.That(
                    "Story 39: CompareTag error names the undefined tag",
                    compareUndefinedMessage.Contains("CompletelyUndefinedTag_Story39", StringComparison.Ordinal));

                bool componentCompareThrew = false;
                try
                {
                    go.transform.CompareTag("CompletelyUndefinedTag_Story39");
                }
                catch (UnityException)
                {
                    componentCompareThrew = true;
                }

                Check.That("Story 39: Component.CompareTag with undefined tag raises UnityException", componentCompareThrew);

                GameObject.RegisterTag("CustomQuestTarget");
                Check.That("Registered custom tag is accepted by CompareTag", !go.CompareTag("CustomQuestTarget"));
                go.tag = "CustomQuestTarget";
                Check.That("CompareTag matches after assigning registered custom tag", go.CompareTag("CustomQuestTarget"));
            }
            finally
            {
                GameObject.ResetDefinedTags();
                DestroyAll(created);
            }
        }

        private static void TestStory39UnassignedSerializedReferenceRaisesUnassignedOrNullException()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject hostGo = Track(created, new GameObject("InspectorHost"));
                UnassignedRefProbeBehaviour probe = hostGo.AddComponent<UnassignedRefProbeBehaviour>();
                probe.TargetObject = GameObject.CreateUnassignedReference(nameof(UnassignedRefProbeBehaviour.TargetObject), hostGo.name);
                probe.TargetTransform = Component.CreateUnassignedReference<Transform>(nameof(UnassignedRefProbeBehaviour.TargetTransform), hostGo.name);

                Check.That("Unassigned GameObject placeholder compares equal to null via Unity ==", probe.TargetObject == null);
                Check.That("Unassigned Transform placeholder compares equal to null via Unity ==", probe.TargetTransform == null);
                Check.That("Unassigned reference placeholder is falsy in bool conversion", !probe.TargetObject);

                bool editorThrewUnassigned = false;
                string editorExMessage = string.Empty;
                try
                {
                    _ = probe.TargetObject!.name;
                }
                catch (UnassignedReferenceException ex)
                {
                    editorThrewUnassigned = true;
                    editorExMessage = ex.Message;
                }

                Check.That("Story 39: Accessing unassigned serialized GameObject reference raises UnassignedReferenceException in editor mode", editorThrewUnassigned);
                Check.That("Story 39: UnassignedReferenceException names the field TargetObject", editorExMessage.Contains("TargetObject", StringComparison.Ordinal));

                bool transformThrewUnassigned = false;
                try
                {
                    _ = probe.TargetTransform!.name;
                }
                catch (UnassignedReferenceException)
                {
                    transformThrewUnassigned = true;
                }

                Check.That("Story 39: Accessing unassigned serialized Component reference raises UnassignedReferenceException in editor mode", transformThrewUnassigned);

                UnityEngine.Object.ForcePlayerReferenceExceptions = true;
                try
                {
                    bool playerThrewNullRef = false;
                    try
                    {
                        _ = probe.TargetObject!.name;
                    }
                    catch (NullReferenceException)
                    {
                        playerThrewNullRef = true;
                    }

                    Check.That("Story 39: Accessing unassigned serialized reference in player mode raises NullReferenceException", playerThrewNullRef);
                }
                finally
                {
                    UnityEngine.Object.ForcePlayerReferenceExceptions = false;
                }
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestStory12RequireComponentAutoAddsAndRefusesDependencyRemoval()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            LogRecorder.Clear();
            try
            {
                GameObject go = Track(created, new GameObject("RequireComponentHost"));
                DependentProbeComponent dependent = go.AddComponent<DependentProbeComponent>();
                Check.That("DependentProbeComponent is attached", dependent != null);

                RequiredDependencyComponent? dep1 = go.GetComponent<RequiredDependencyComponent>();
                SecondaryRequiredDependencyComponent? dep2 = go.GetComponent<SecondaryRequiredDependencyComponent>();
                Check.That("Story 12: [RequireComponent] auto-adds first required dependency", dep1 != null);
                Check.That("Story 12: [RequireComponent] auto-adds second required dependency", dep2 != null);

                if (dep1 != null)
                {
                    LogRecorder.Clear();
                    UnityEngine.Object.Destroy(dep1);
                    Check.That("Story 12: Removing required dependency via Destroy is refused while dependent is attached", !dep1.IsDestroyed && go.GetComponent<RequiredDependencyComponent>() != null);

                    IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                    Check.Equal("Story 12: Refusing required component removal logs an error", 1, errors.Count);
                    if (errors.Count == 1)
                    {
                        Check.That(
                            "Story 12: Removal refusal reason names dependent component DependentProbeComponent",
                            errors[0].Message.Contains(nameof(DependentProbeComponent), StringComparison.Ordinal));
                        Check.That(
                            "Story 12: Removal refusal reason names required component RequiredDependencyComponent",
                            errors[0].Message.Contains(nameof(RequiredDependencyComponent), StringComparison.Ordinal));
                    }

                    LogRecorder.Clear();
                    UnityEngine.Object.DestroyImmediate(dep1);
                    Check.That("Story 12: Removing required dependency via DestroyImmediate is also refused", !dep1.IsDestroyed);
                    Check.Equal("Story 12: DestroyImmediate refusal logs an error", 1, LogRecorder.OfSeverity(LogSeverity.Error).Count);

                    if (dependent != null)
                    {
                        UnityEngine.Object.Destroy(dependent);
                        Check.That("Dependent component can be destroyed", dependent.IsDestroyed);
                    }

                    UnityEngine.Object.Destroy(dep1);
                    Check.That("Required dependency can be destroyed after dependent component is removed", dep1.IsDestroyed);
                }
            }
            finally
            {
                LogRecorder.Clear();
                DestroyAll(created);
            }
        }

        private static void TestStory12DisallowMultipleComponentRefusesDuplicate()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            LogRecorder.Clear();
            try
            {
                GameObject go = Track(created, new GameObject("DisallowMultipleHost"));
                SingleInstanceProbeComponent? first = go.AddComponent<SingleInstanceProbeComponent>();
                Check.That("First [DisallowMultipleComponent] instance attaches", first != null);

                LogRecorder.Clear();
                SingleInstanceProbeComponent? secondGeneric = go.AddComponent<SingleInstanceProbeComponent>();
                Check.That("Story 12: Second generic AddComponent<T> of [DisallowMultipleComponent] is refused (returns null)", secondGeneric == null);
                Check.Equal("Only one SingleInstanceProbeComponent exists on GameObject", 1, go.GetComponents<SingleInstanceProbeComponent>().Length);

                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 12: Refusing duplicate [DisallowMultipleComponent] logs an error", 1, errors.Count);
                if (errors.Count == 1)
                {
                    Check.That(
                        "Story 12: Duplicate refusal error names SingleInstanceProbeComponent",
                        errors[0].Message.Contains(nameof(SingleInstanceProbeComponent), StringComparison.Ordinal));
                }

                LogRecorder.Clear();
                Component? secondByType = go.AddComponent(typeof(SingleInstanceProbeComponent));
                Check.That("Story 12: Second non-generic AddComponent(Type) of [DisallowMultipleComponent] is refused (returns null)", secondByType == null);
                Check.Equal("Non-generic duplicate refusal also logs an error", 1, LogRecorder.OfSeverity(LogSeverity.Error).Count);
            }
            finally
            {
                LogRecorder.Clear();
                DestroyAll(created);
            }
        }

        private static void TestStory11DefaultExecutionOrderAndSettingOverride()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            List<string> trace = new List<string>();
            LateOrderedProbeScript.SharedLog = trace;
            EarlyOrderedProbeScript.SharedLog = trace;
            DefaultOrderedProbeScript.SharedLog = trace;
            MonoBehaviour.ClearExecutionOrderOverrides();
            try
            {
                GameObject go = Track(created, new GameObject("ExecutionOrderHost"));
                LateOrderedProbeScript late = go.AddComponent<LateOrderedProbeScript>();
                DefaultOrderedProbeScript def = go.AddComponent<DefaultOrderedProbeScript>();
                EarlyOrderedProbeScript early = go.AddComponent<EarlyOrderedProbeScript>();

                Check.Equal("LateOrderedProbeScript effective order from attribute is 50", 50, MonoBehaviour.GetEffectiveExecutionOrder(late));
                Check.Equal("DefaultOrderedProbeScript effective order is 0", 0, MonoBehaviour.GetEffectiveExecutionOrder(def));
                Check.Equal("EarlyOrderedProbeScript effective order from attribute is -50", -50, MonoBehaviour.GetEffectiveExecutionOrder(early));

                MonoBehaviour[] behaviors = new MonoBehaviour[] { late, def, early };
                MonoBehaviour.RunLifecycleBatch(behaviors, runFixedUpdate: true);

                Check.Equal(
                    "Story 11: [DefaultExecutionOrder] orders Awake, OnEnable, Start, FixedUpdate, Update, and LateUpdate across script types",
                    "Early-50:Awake, Default0:Awake, Late50:Awake, "
                    + "Early-50:OnEnable, Default0:OnEnable, Late50:OnEnable, "
                    + "Early-50:Start, Default0:Start, Late50:Start, "
                    + "Early-50:FixedUpdate, Default0:FixedUpdate, Late50:FixedUpdate, "
                    + "Early-50:Update, Default0:Update, Late50:Update, "
                    + "Early-50:LateUpdate, Default0:LateUpdate, Late50:LateUpdate",
                    string.Join(", ", trace));

                trace.Clear();
                MonoBehaviour.SetExecutionOrderOverride<LateOrderedProbeScript>(-100);
                Check.Equal("Setting overrides [DefaultExecutionOrder] attribute value", -100, MonoBehaviour.GetEffectiveExecutionOrder(late));

                MonoBehaviour.RunLifecycleBatch(behaviors, runFixedUpdate: false);
                Check.Equal(
                    "Story 11: Script Execution Order setting overrides [DefaultExecutionOrder] attribute for Awake, OnEnable, Start, Update, LateUpdate",
                    "Late50:Awake, Early-50:Awake, Default0:Awake, "
                    + "Late50:OnEnable, Early-50:OnEnable, Default0:OnEnable, "
                    + "Late50:Start, Early-50:Start, Default0:Start, "
                    + "Late50:Update, Early-50:Update, Default0:Update, "
                    + "Late50:LateUpdate, Early-50:LateUpdate, Default0:LateUpdate",
                    string.Join(", ", trace));
            }
            finally
            {
                LateOrderedProbeScript.SharedLog = null;
                EarlyOrderedProbeScript.SharedLog = null;
                DefaultOrderedProbeScript.SharedLog = null;
                MonoBehaviour.ClearExecutionOrderOverrides();
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
    }
}
