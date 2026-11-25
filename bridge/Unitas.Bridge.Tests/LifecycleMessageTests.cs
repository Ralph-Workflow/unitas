using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using Unitas.Bridge.Messaging;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class Story12RequiredDependencyComponent : MonoBehaviour
    {
    }

    public sealed class Story12DependentComponent : MonoBehaviour
    {
        public override IReadOnlyList<Type> RequiredComponents =>
            new[] { typeof(Story12RequiredDependencyComponent) };
    }

    public sealed class Story12SingleInstanceComponent : MonoBehaviour
    {
        public override bool DisallowMultiple => true;
    }

    public sealed class Story14MessageProbeBehaviour : MonoBehaviour
    {
        public readonly List<string> Trace = new List<string>();

        public override void OnBecameVisible() => Trace.Add("OnBecameVisible");

        public override void OnBecameInvisible() => Trace.Add("OnBecameInvisible");

        public override void OnMouseDown() => Trace.Add("OnMouseDown");
    }

    public sealed class Story32NamedCoroutineProbe : MonoBehaviour
    {
        public readonly List<string> Trace = new List<string>();
    }

    public static class LifecycleMessageTests
    {
        public static void Run()
        {
            TestStory12RequireComponentAndDisallowMultipleRefusalsAreIdenticalForAgentAndPersonPaths();
            TestStory14OnBecameVisibleAndOnMouseDownArriveThroughMessageDispatcher();
            TestStory16BrowserTargetThreadCreationReportsPlatformLimited();
            TestStory32StopCoroutineByNameStopsOnlyCoroutinesStartedWithThatName();
        }

        private static void TestStory12RequireComponentAndDisallowMultipleRefusalsAreIdenticalForAgentAndPersonPaths()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            LogRecorder.Clear();
            try
            {
                // [DisallowMultipleComponent]: the agent (programmatic) path.
                GameObject agentHost = Track(created, new GameObject("Story12_DisallowMultipleHost"));
                Story12SingleInstanceComponent? agentFirst = agentHost.AddComponent<Story12SingleInstanceComponent>();
                Check.That("Story 12 agent path attaches the first [DisallowMultipleComponent] instance", agentFirst != null);

                LogRecorder.Clear();
                GameObject.ComponentOperationResult agentResult = agentHost.TryAddComponentForAgent<Story12SingleInstanceComponent>();
                Check.That("Story 12 agent path refuses a second [DisallowMultipleComponent] instance", !agentResult.Succeeded);
                Check.That("Story 12 agent path reports a refusal reason", !string.IsNullOrEmpty(agentResult.RefusalReason));
                IReadOnlyList<LogEntry> agentErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 12 agent path logs one error for the refused add", 1, agentErrors.Count);
                string agentReason = agentErrors.Count == 1 ? agentErrors[0].Message : string.Empty;
                Check.Equal("Story 12 agent path refusal reason equals the logged error", agentReason, agentResult.RefusalReason ?? string.Empty);
                Check.Equal("Story 12 agent path leaves one [DisallowMultipleComponent] instance", 1, agentHost.GetComponents<Story12SingleInstanceComponent>().Length);

                // [DisallowMultipleComponent]: the person (editor) path.
                GameObject personHost = Track(created, new GameObject("Story12_DisallowMultipleHost"));
                Story12SingleInstanceComponent? personFirst = personHost.AddComponent<Story12SingleInstanceComponent>();
                Check.That("Story 12 person path attaches the first [DisallowMultipleComponent] instance", personFirst != null);

                LogRecorder.Clear();
                Story12SingleInstanceComponent? personSecond = personHost.AddComponent<Story12SingleInstanceComponent>();
                Check.That("Story 12 person path refuses a second [DisallowMultipleComponent] instance", personSecond == null);
                IReadOnlyList<LogEntry> personErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 12 person path logs one error for the refused add", 1, personErrors.Count);
                string personReason = personErrors.Count == 1 ? personErrors[0].Message : string.Empty;
                Check.Equal("Story 12 person path refusal reason equals the agent path refusal reason", agentReason, personReason);
                Check.Equal("Story 12 person path leaves one [DisallowMultipleComponent] instance", 1, personHost.GetComponents<Story12SingleInstanceComponent>().Length);

                // [DisallowMultipleComponent]: the serialized (editor-authored scene) path.
                GameObject serializedHost = Track(created, new GameObject("Story12_DisallowMultipleHost"));
                Story12SingleInstanceComponent? serializedFirst = serializedHost.AddComponent<Story12SingleInstanceComponent>();
                Check.That("Story 12 serialized path attaches the first [DisallowMultipleComponent] instance", serializedFirst != null);

                LogRecorder.Clear();
                Story12SingleInstanceComponent serializedDuplicate = new Story12SingleInstanceComponent();
                serializedHost.AttachComponent(serializedDuplicate, typeof(Story12SingleInstanceComponent));
                IReadOnlyList<LogEntry> serializedErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 12 serialized path logs one error for the refused attach", 1, serializedErrors.Count);
                string serializedReason = serializedErrors.Count == 1 ? serializedErrors[0].Message : string.Empty;
                Check.Equal("Story 12 serialized path refusal reason equals the agent path refusal reason", agentReason, serializedReason);
                Check.Equal("Story 12 serialized path leaves one [DisallowMultipleComponent] instance", 1, serializedHost.GetComponents<Story12SingleInstanceComponent>().Length);

                // [RequireComponent]: every path adds the required dependency.
                GameObject agentDependencyHost = Track(created, new GameObject("Story12_RequireComponentHost"));
                GameObject.ComponentOperationResult agentDependent = agentDependencyHost.TryAddComponentForAgent<Story12DependentComponent>();
                Check.That("Story 12 agent path attaches a [RequireComponent] component", agentDependent.Succeeded);
                Check.That("Story 12 agent path adds the required dependency", agentDependencyHost.GetComponent<Story12RequiredDependencyComponent>() != null);

                GameObject personDependencyHost = Track(created, new GameObject("Story12_RequireComponentHost"));
                Story12DependentComponent? personDependent = personDependencyHost.AddComponent<Story12DependentComponent>();
                Check.That("Story 12 person path attaches a [RequireComponent] component", personDependent != null);
                Check.That("Story 12 person path adds the required dependency", personDependencyHost.GetComponent<Story12RequiredDependencyComponent>() != null);

                GameObject serializedDependencyHost = Track(created, new GameObject("Story12_RequireComponentHost"));
                Story12DependentComponent serializedDependent = new Story12DependentComponent();
                serializedDependencyHost.AttachComponent(serializedDependent, typeof(Story12DependentComponent));
                Check.That("Story 12 serialized path attaches a [RequireComponent] component", serializedDependencyHost.GetComponent<Story12DependentComponent>() != null);
                Check.That("Story 12 serialized path adds the required dependency", serializedDependencyHost.GetComponent<Story12RequiredDependencyComponent>() != null);

                // [RequireComponent]: removing the dependency is refused with the same reason on both paths.
                GameObject agentRemovalHost = Track(created, new GameObject("Story12_RequireComponentHost"));
                agentRemovalHost.AddComponent<Story12DependentComponent>();
                Story12RequiredDependencyComponent? agentDependency = agentRemovalHost.GetComponent<Story12RequiredDependencyComponent>();
                Check.That("Story 12 agent path has the required dependency to remove", agentDependency != null);

                LogRecorder.Clear();
                GameObject.ComponentOperationResult agentRemoval = agentRemovalHost.TryRemoveComponentForAgent(agentDependency!);
                Check.That("Story 12 agent path refuses to remove a required dependency", !agentRemoval.Succeeded);
                IReadOnlyList<LogEntry> agentRemovalErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 12 agent path logs one error for the refused removal", 1, agentRemovalErrors.Count);
                string agentRemovalReason = agentRemovalErrors.Count == 1 ? agentRemovalErrors[0].Message : string.Empty;
                Check.That(
                    "Story 12 agent removal reason names the dependent component",
                    agentRemovalReason.Contains(nameof(Story12DependentComponent), StringComparison.Ordinal));
                Check.That(
                    "Story 12 agent removal reason names the required dependency",
                    agentRemovalReason.Contains(nameof(Story12RequiredDependencyComponent), StringComparison.Ordinal));
                Check.That("Story 12 agent path keeps the required dependency attached", !agentDependency!.IsDestroyed);

                GameObject personRemovalHost = Track(created, new GameObject("Story12_RequireComponentHost"));
                personRemovalHost.AddComponent<Story12DependentComponent>();
                Story12RequiredDependencyComponent? personDependency = personRemovalHost.GetComponent<Story12RequiredDependencyComponent>();
                Check.That("Story 12 person path has the required dependency to remove", personDependency != null);

                LogRecorder.Clear();
                UnityEngine.Object.Destroy(personDependency);
                IReadOnlyList<LogEntry> personRemovalErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 12 person path logs one error for the refused removal", 1, personRemovalErrors.Count);
                string personRemovalReason = personRemovalErrors.Count == 1 ? personRemovalErrors[0].Message : string.Empty;
                Check.Equal("Story 12 person and agent removal refusals report the identical reason", agentRemovalReason, personRemovalReason);
                Check.That("Story 12 person path keeps the required dependency attached", !personDependency!.IsDestroyed);
            }
            finally
            {
                LogRecorder.Clear();
                DestroyAll(created);
            }
        }

        private static void TestStory14OnBecameVisibleAndOnMouseDownArriveThroughMessageDispatcher()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            InputHandlingMode savedMode = MessageDispatcher.ActiveInputHandling;
            Story14MessageProbeBehaviour? probe = null;
            try
            {
                GameObject host = Track(created, new GameObject("Story14_MessageHost"));
                probe = host.AddComponent<Story14MessageProbeBehaviour>();
                BoxCollider? collider = host.AddComponent<BoxCollider>();
                Check.That("Story 14 the message host has the collider an OnMouseDown needs", collider != null);

                List<string> dispatched = new List<string>();
                MessageDispatcher.RegisterHandler(probe, (Story14MessageProbeBehaviour _) => dispatched.Add("OnBecameVisible"), "OnBecameVisible");
                MessageDispatcher.RegisterHandler(probe, (Story14MessageProbeBehaviour _) => dispatched.Add("OnMouseDown"), "OnMouseDown");

                // The real render and input triggers belong to another workstream, so the trigger is
                // injected at the MessageDispatcher seam here.
                Check.That(
                    "Story 14 the injected visibility trigger is delivered through the MessageDispatcher",
                    MessageDispatcher.NotifyBecameVisible(host));
                Check.Equal(
                    "Story 14 OnBecameVisible reaches the registered message handler",
                    "OnBecameVisible",
                    string.Join(", ", dispatched));

                dispatched.Clear();
                Check.That(
                    "Story 14 the injected mouse trigger is delivered through the MessageDispatcher",
                    MessageDispatcher.NotifyMouseDown(host));
                Check.Equal(
                    "Story 14 OnMouseDown reaches the registered message handler",
                    "OnMouseDown",
                    string.Join(", ", dispatched));

                GameObject builtinHost = Track(created, new GameObject("Story14_BuiltinMessageHost"));
                Story14MessageProbeBehaviour builtinProbe = builtinHost.AddComponent<Story14MessageProbeBehaviour>();
                BoxCollider? builtinCollider = builtinHost.AddComponent<BoxCollider>();
                Check.That("Story 14 the built-in message host has a collider", builtinCollider != null);

                MessageDispatcher.NotifyBecameVisible(builtinHost);
                MessageDispatcher.NotifyMouseDown(builtinHost);
                Check.Equal(
                    "Story 14 the built-in OnBecameVisible and OnMouseDown callbacks run through the MessageDispatcher",
                    "OnBecameVisible, OnMouseDown",
                    string.Join(", ", builtinProbe.Trace));

                MessageDispatcher.ActiveInputHandling = InputHandlingMode.InputSystemPackage;
                dispatched.Clear();
                builtinProbe.Trace.Clear();
                LogRecorder.Clear();

                Check.That(
                    "Story 14 OnMouseDown is not delivered when only the Input System is active",
                    !MessageDispatcher.NotifyMouseDown(host));
                Check.Equal(
                    "Story 14 no OnMouseDown message reaches the handler when only the Input System is active",
                    0,
                    dispatched.Count);
                Check.That(
                    "Story 14 the built-in OnMouseDown callback does not run when only the Input System is active",
                    !builtinProbe.Trace.Contains("OnMouseDown"));
                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                Check.Equal("Story 14 the suppressed OnMouseDown logs one warning", 1, warnings.Count);
                Check.That(
                    "Story 14 the warning names OnMouseDown",
                    warnings.Count == 1 && warnings[0].Message.Contains("OnMouseDown", StringComparison.Ordinal));

                // OnBecameVisible is a renderer message, not an input message, so the input handling
                // setting does not gate it (docs/product-specification/scripting-runtime.md).
                Check.That(
                    "Story 14 OnBecameVisible is still delivered when only the Input System is active",
                    MessageDispatcher.NotifyBecameVisible(host));
                Check.Equal(
                    "Story 14 OnBecameVisible still reaches the registered message handler when only the Input System is active",
                    "OnBecameVisible",
                    string.Join(", ", dispatched));

                MessageDispatcher.ActiveInputHandling = InputHandlingMode.Both;
                dispatched.Clear();
                Check.That(
                    "Story 14 OnMouseDown is delivered when both input handlers are active",
                    MessageDispatcher.NotifyMouseDown(host));
                Check.Equal(
                    "Story 14 OnMouseDown reaches the registered message handler when both input handlers are active",
                    "OnMouseDown",
                    string.Join(", ", dispatched));
            }
            finally
            {
                MessageDispatcher.ActiveInputHandling = savedMode;
                if (probe != null)
                {
                    MessageDispatcher.UnregisterHandler(probe);
                }

                LogRecorder.Clear();
                DestroyAll(created);
            }
        }

        private static void TestStory16BrowserTargetThreadCreationReportsPlatformLimited()
        {
            RuntimePlatform savedPlatform = MessageDispatcher.SimulatedTargetPlatform;
            try
            {
                MessageDispatcher.SimulatedTargetPlatform = RuntimePlatform.OSXEditor;
                MessageDispatcher.ClearPlatformLimitedDiagnostics();
                LogRecorder.Clear();

                bool desktopRan = false;
                bool desktopStarted = MessageDispatcher.TryStartThread(
                    () => desktopRan = true,
                    "Story16ProbeBehaviour.RunWorker",
                    out PlatformLimitedDiagnostic? desktopDiagnostic);
                Check.That("Story 16 a desktop target creates the thread", desktopStarted);
                Check.That("Story 16 a desktop target runs the thread body", desktopRan);
                Check.That("Story 16 a desktop target reports no Platform-limited diagnostic", desktopDiagnostic == null);

                MessageDispatcher.SimulatedTargetPlatform = RuntimePlatform.WebGLPlayer;
                MessageDispatcher.ClearPlatformLimitedDiagnostics();
                LogRecorder.Clear();

                bool browserRan = false;
                bool browserStarted = MessageDispatcher.TryStartThread(
                    () => browserRan = true,
                    "Story16ProbeBehaviour.RunWorker",
                    out PlatformLimitedDiagnostic? browserDiagnostic);

                Check.That("Story 16 a browser target does not create the thread", !browserStarted);
                Check.That("Story 16 a browser target reports a Platform-limited diagnostic", browserDiagnostic != null);
                Check.Equal(
                    "Story 16 the diagnostic compatibility state is Platform-limited",
                    "Platform-limited",
                    browserDiagnostic?.CompatibilityState ?? string.Empty);
                Check.Equal(
                    "Story 16 the diagnostic names the API",
                    "System.Threading.Thread.Start",
                    browserDiagnostic?.ApiName ?? string.Empty);
                Check.Equal(
                    "Story 16 the diagnostic names the calling member",
                    "Story16ProbeBehaviour.RunWorker",
                    browserDiagnostic?.CallingMember ?? string.Empty);
                Check.That(
                    "Story 16 the diagnostic targets the browser platform",
                    browserDiagnostic != null && browserDiagnostic.TargetPlatform == RuntimePlatform.WebGLPlayer);
                Check.That(
                    "Story 16 the diagnostic message names the API",
                    browserDiagnostic != null && browserDiagnostic.Message.Contains("System.Threading.Thread.Start", StringComparison.Ordinal));
                Check.That(
                    "Story 16 the diagnostic message names the calling member",
                    browserDiagnostic != null && browserDiagnostic.Message.Contains("Story16ProbeBehaviour.RunWorker", StringComparison.Ordinal));
                Check.That("Story 16 the thread body never runs on a browser target", !browserRan);
                Check.Equal(
                    "Story 16 the Platform-limited diagnostic is recorded",
                    1,
                    MessageDispatcher.PlatformLimitedDiagnostics.Count);
                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                Check.Equal("Story 16 the browser thread refusal logs one warning", 1, warnings.Count);
                Check.That(
                    "Story 16 the warning names the API and the calling member",
                    warnings.Count == 1
                    && warnings[0].Message.Contains("System.Threading.Thread.Start", StringComparison.Ordinal)
                    && warnings[0].Message.Contains("Story16ProbeBehaviour.RunWorker", StringComparison.Ordinal));
            }
            finally
            {
                MessageDispatcher.SimulatedTargetPlatform = savedPlatform;
                MessageDispatcher.ClearPlatformLimitedDiagnostics();
                LogRecorder.Clear();
            }
        }

        private static void TestStory32StopCoroutineByNameStopsOnlyCoroutinesStartedWithThatName()
        {
            GameObject host = new GameObject("Story32_NamedCoroutineHost");
            Story32NamedCoroutineProbe probe = host.AddComponent<Story32NamedCoroutineProbe>();
            float savedScale = Time.timeScale;
            try
            {
                Time.Reset();
                Time.timeScale = 1f;

                probe.RegisterNamedCoroutine("Alpha", () => Story32CounterRoutine(probe.Trace, "Alpha"));
                probe.RegisterNamedCoroutine("Beta", () => Story32CounterRoutine(probe.Trace, "Beta"));

                Coroutine? alpha = probe.StartCoroutine("Alpha");
                Coroutine? beta = probe.StartCoroutine("Beta");
                Coroutine? gamma = probe.StartCoroutine(Story32CounterRoutine(probe.Trace, "Gamma"));

                Check.That("Story 32 the named coroutine Alpha starts", alpha != null);
                Check.That("Story 32 the named coroutine Beta starts", beta != null);
                Check.That("Story 32 the enumerator coroutine Gamma starts", gamma != null);
                Check.Equal(
                    "Story 32 every coroutine runs to its first yield",
                    "Alpha:0, Beta:0, Gamma:0",
                    string.Join(", ", probe.Trace));

                PumpStory32Frame();
                Check.Equal(
                    "Story 32 every coroutine ticks once",
                    "Alpha:0, Beta:0, Gamma:0, Alpha:1, Beta:1, Gamma:1",
                    string.Join(", ", probe.Trace));

                probe.StopCoroutine("Alpha");
                PumpStory32Frame();
                PumpStory32Frame();

                Check.Equal(
                    "Story 32 StopCoroutine(string) stops only the coroutine started with that name",
                    "Alpha:0, Beta:0, Gamma:0, Alpha:1, Beta:1, Gamma:1, Beta:2, Gamma:2",
                    string.Join(", ", probe.Trace));
            }
            finally
            {
                Time.timeScale = savedScale;
                CoroutineScheduler.OwnerDestroyed(probe);
                UnityEngine.Object.Destroy(host);
            }
        }

        private static void PumpStory32Frame()
        {
            Time.Advance(0.02f);
            CoroutineScheduler.PumpFrame();
        }

        private static IEnumerator Story32CounterRoutine(List<string> trace, string label)
        {
            for (int i = 0; i < 3; i++)
            {
                trace.Add(label + ":" + i.ToString(CultureInfo.InvariantCulture));
                yield return null;
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
