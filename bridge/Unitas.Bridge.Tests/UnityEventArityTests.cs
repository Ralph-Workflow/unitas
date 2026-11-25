using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using UnityEngine;
using UnityEngine.Events;

namespace Unitas.Bridge.Tests
{
    public sealed class ArityEventReceiverBehaviour : MonoBehaviour, IUnitasMessageHandler
    {
        public Collection<string> Trace { get; } = new Collection<string>();

        public bool Dispatch(string message)
        {
            switch (message)
            {
                case "OnPersistentVoid":
                    Trace.Add("persistent:void");
                    return true;
                case "OnHeartbeat":
                    Trace.Add("heartbeat:" + gameObject.name);
                    return true;
                case "OnDelayedShot":
                    Trace.Add("delayed:" + gameObject.name);
                    return true;
                default:
                    return false;
            }
        }

        public bool Dispatch<T>(string message, T argument)
        {
            switch (message)
            {
                case "OnPersistentString":
                    if (argument is string label)
                    {
                        Trace.Add("persistent:string:" + label);
                        return true;
                    }
                    return false;
                case "OnPersistentInt":
                    if (argument is int number)
                    {
                        Trace.Add("persistent:int:" + number);
                        return true;
                    }
                    return false;
                case "OnArity2Dynamic":
                    if (argument is ValueTuple<string, int> pair)
                    {
                        Trace.Add("persistent:arity2:" + pair.Item1 + ":" + pair.Item2);
                        return true;
                    }
                    return false;
                case "OnArity3Dynamic":
                    if (argument is ValueTuple<string, int, float> triple)
                    {
                        Trace.Add("persistent:arity3:" + triple.Item1 + ":" + triple.Item2 + ":" + triple.Item3.ToString(CultureInfo.InvariantCulture));
                        return true;
                    }
                    return false;
                case "OnArity4Dynamic":
                    if (argument is ValueTuple<string, int, float, bool> quad)
                    {
                        Trace.Add("persistent:arity4:" + quad.Item1 + ":" + quad.Item2 + ":" + quad.Item3.ToString(CultureInfo.InvariantCulture) + ":" + quad.Item4);
                        return true;
                    }
                    return false;
                default:
                    return false;
            }
        }
    }

    public static class UnityEventArityTests
    {
        public static void Run()
        {
            TestTwoArityUnityEventDynamicAndPersistentListeners();
            TestThreeArityUnityEventDynamicAndPersistentListeners();
            TestFourArityUnityEventDynamicAndPersistentListeners();
            TestRenamedPersistentMethodReportsOwnerFieldTargetMethodAndContinuesAcrossArities();
            TestSendMessageRequireReceiverVsDontRequireReceiver();
            TestInvokeRepeatingTimeScaleZeroPauseAndPermanentCancellationOnDestroy();
            TestDecimalLocaleDoesNotCorruptFloatEventTraces();
        }

        private static void TestTwoArityUnityEventDynamicAndPersistentListeners()
        {
            GameObject owner = new GameObject("ScoreBoard");
            ArityEventReceiverBehaviour receiver = owner.AddComponent<ArityEventReceiverBehaviour>();
            UnityEvent<string, int> onScoreUpdated = new UnityEvent<string, int>("ScoreBoard", "onScoreUpdated");

            UnityAction<string, int> runtimeFirst = (player, pts) => receiver.Trace.Add("runtime:1:" + player + ":" + pts);
            UnityAction<string, int> runtimeSecond = (player, pts) => receiver.Trace.Add("runtime:2:" + player + ":" + pts);

            onScoreUpdated.AddListener(runtimeFirst);
            onScoreUpdated.AddPersistentStringListener(owner, "onScoreUpdated", receiver, "OnPersistentString", "ScoreBanner");
            onScoreUpdated.AddPersistentListener(owner, "onScoreUpdated", receiver, "OnArity2Dynamic");
            onScoreUpdated.AddListener(runtimeSecond);

            onScoreUpdated.Invoke("Alice", 120);
            Check.Equal(
                "UnityEvent<T0, T1> invokes persistent listeners in order before runtime listeners",
                "persistent:string:ScoreBanner, persistent:arity2:Alice:120, runtime:1:Alice:120, runtime:2:Alice:120",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onScoreUpdated.RemoveListener(runtimeFirst);
            onScoreUpdated.SetPersistentListenerState(0, UnityEventCallState.Off);
            onScoreUpdated.Invoke("Bob", 85);
            Check.Equal(
                "UnityEvent<T0, T1> honors RemoveListener and SetPersistentListenerState(Off)",
                "persistent:arity2:Bob:85, runtime:2:Bob:85",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onScoreUpdated.SetPersistentListenerState(0, UnityEventCallState.RuntimeOnly);
            onScoreUpdated.RemoveAllListeners();
            onScoreUpdated.Invoke("Charlie", 200);
            Check.Equal(
                "UnityEvent<T0, T1> RemoveAllListeners clears runtime listeners while preserving persistent listeners",
                "persistent:string:ScoreBanner, persistent:arity2:Charlie:200",
                string.Join(", ", receiver.Trace));

            UnityEngine.Object.Destroy(owner);
        }

        private static void TestThreeArityUnityEventDynamicAndPersistentListeners()
        {
            GameObject owner = new GameObject("CombatEmitter");
            ArityEventReceiverBehaviour receiver = owner.AddComponent<ArityEventReceiverBehaviour>();
            UnityEvent<string, int, float> onHit = new UnityEvent<string, int, float>("CombatEmitter", "onHit");

            UnityAction<string, int, float> firstListener = (targetName, dmg, crit) =>
                receiver.Trace.Add("runtime:1:" + targetName + ":" + dmg + ":" + crit.ToString(CultureInfo.InvariantCulture));
            UnityAction<string, int, float> secondListener = (targetName, dmg, crit) =>
                receiver.Trace.Add("runtime:2:" + targetName + ":" + dmg + ":" + crit.ToString(CultureInfo.InvariantCulture));

            onHit.AddListener(firstListener);
            onHit.AddPersistentIntListener(owner, "onHit", receiver, "OnPersistentInt", 42);
            onHit.AddPersistentListener(owner, "onHit", receiver, "OnArity3Dynamic");
            onHit.AddListener(secondListener);

            onHit.Invoke("Boss", 50, 1.5f);
            Check.Equal(
                "UnityEvent<T0, T1, T2> invokes persistent listeners before runtime listeners",
                "persistent:int:42, persistent:arity3:Boss:50:1.5, runtime:1:Boss:50:1.5, runtime:2:Boss:50:1.5",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onHit.RemoveListener(firstListener);
            onHit.SetPersistentListenerState(1, UnityEventCallState.Off);
            onHit.Invoke("Minion", 15, 1f);
            Check.Equal(
                "UnityEvent<T0, T1, T2> skips disabled persistent listener and removed runtime listener",
                "persistent:int:42, runtime:2:Minion:15:1",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onHit.SetPersistentListenerState(1, UnityEventCallState.EditorAndRuntime);
            onHit.RemoveAllListeners();
            onHit.Invoke("Tower", 90, 2f);
            Check.Equal(
                "UnityEvent<T0, T1, T2> RemoveAllListeners keeps persistent listeners intact",
                "persistent:int:42, persistent:arity3:Tower:90:2",
                string.Join(", ", receiver.Trace));

            UnityEngine.Object.Destroy(owner);
        }

        private static void TestFourArityUnityEventDynamicAndPersistentListeners()
        {
            GameObject owner = new GameObject("TelemetryBus");
            ArityEventReceiverBehaviour receiver = owner.AddComponent<ArityEventReceiverBehaviour>();
            UnityEvent<string, int, float, bool> onTelemetry =
                new UnityEvent<string, int, float, bool>("TelemetryBus", "onTelemetry");

            UnityAction<string, int, float, bool> firstListener = (evtName, code, elapsed, ok) =>
                receiver.Trace.Add("runtime:1:" + evtName + ":" + code + ":" + elapsed.ToString(CultureInfo.InvariantCulture) + ":" + ok);
            UnityAction<string, int, float, bool> secondListener = (evtName, code, elapsed, ok) =>
                receiver.Trace.Add("runtime:2:" + evtName + ":" + code + ":" + elapsed.ToString(CultureInfo.InvariantCulture) + ":" + ok);

            onTelemetry.AddListener(firstListener);
            onTelemetry.AddPersistentListener(owner, "onTelemetry", receiver, "OnPersistentVoid");
            onTelemetry.AddPersistentListener(owner, "onTelemetry", receiver, "OnArity4Dynamic");
            onTelemetry.AddListener(secondListener);

            onTelemetry.Invoke("Spawn", 7, 0.25f, true);
            Check.Equal(
                "UnityEvent<T0, T1, T2, T3> invokes persistent listeners before runtime listeners",
                "persistent:void, persistent:arity4:Spawn:7:0.25:True, runtime:1:Spawn:7:0.25:True, runtime:2:Spawn:7:0.25:True",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onTelemetry.RemoveListener(secondListener);
            onTelemetry.SetPersistentListenerState(0, UnityEventCallState.Off);
            onTelemetry.Invoke("Despawn", 8, 0.5f, false);
            Check.Equal(
                "UnityEvent<T0, T1, T2, T3> respects listener removal and persistent call state Off",
                "persistent:arity4:Despawn:8:0.5:False, runtime:1:Despawn:8:0.5:False",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onTelemetry.SetPersistentListenerState(0, UnityEventCallState.RuntimeOnly);
            onTelemetry.RemoveAllListeners();
            onTelemetry.Invoke("Reset", 9, 1.25f, true);
            Check.Equal(
                "UnityEvent<T0, T1, T2, T3> RemoveAllListeners removes runtime listeners only",
                "persistent:void, persistent:arity4:Reset:9:1.25:True",
                string.Join(", ", receiver.Trace));

            UnityEngine.Object.Destroy(owner);
        }

        private static void TestRenamedPersistentMethodReportsOwnerFieldTargetMethodAndContinuesAcrossArities()
        {
            GameObject buttonOwner = new GameObject("ConfirmButton");
            GameObject targetHost = new GameObject("DialogController");
            ArityEventReceiverBehaviour receiver = targetHost.AddComponent<ArityEventReceiverBehaviour>();
            UnityEvent<string, int, float, bool> onConfirm =
                new UnityEvent<string, int, float, bool>("ConfirmButton", "m_OnConfirm");

            onConfirm.AddPersistentStringListener(
                buttonOwner,
                "m_OnConfirm",
                receiver,
                "OnOldRenamedConfirmHandler",
                "ConfirmPayload");
            onConfirm.AddPersistentStringListener(
                buttonOwner,
                "m_OnConfirm",
                receiver,
                "OnPersistentString",
                "ConfirmPayload");
            onConfirm.AddListener((label, count, weight, active) =>
                receiver.Trace.Add("runtime:confirm:" + label + ":" + count + ":" + weight.ToString(CultureInfo.InvariantCulture) + ":" + active));

            LogRecorder.Clear();
            try
            {
                onConfirm.Invoke("Purchase", 3, 2.5f, true);

                Check.Equal(
                    "Remaining persistent and runtime listeners execute after renamed persistent method",
                    "persistent:string:ConfirmPayload, runtime:confirm:Purchase:3:2.5:True",
                    string.Join(", ", receiver.Trace));

                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                Check.Equal("Renamed persistent method logs 1 warning", 1, warnings.Count);
                if (warnings.Count == 1)
                {
                    string warningMessage = warnings[0].Message;
                    Check.That("Warning names owner ConfirmButton", warningMessage.Contains("ConfirmButton", StringComparison.Ordinal));
                    Check.That("Warning names field m_OnConfirm", warningMessage.Contains("m_OnConfirm", StringComparison.Ordinal));
                    Check.That("Warning names target DialogController", warningMessage.Contains("DialogController", StringComparison.Ordinal));
                    Check.That("Warning names missing method OnOldRenamedConfirmHandler", warningMessage.Contains("OnOldRenamedConfirmHandler", StringComparison.Ordinal));
                }
            }
            finally
            {
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(buttonOwner);
                UnityEngine.Object.Destroy(targetHost);
            }
        }

        private static void TestSendMessageRequireReceiverVsDontRequireReceiver()
        {
            GameObject target = new GameObject("MessengerNode");
            ArityEventReceiverBehaviour receiver = target.AddComponent<ArityEventReceiverBehaviour>();
            LogRecorder.Clear();
            try
            {
                target.SendMessage("OnMissingAction", SendMessageOptions.DontRequireReceiver);
                Check.Equal(
                    "SendMessage with DontRequireReceiver logs no error when no receiver handles message",
                    0,
                    LogRecorder.OfSeverity(LogSeverity.Error).Count);

                target.SendMessage("OnMissingAction", SendMessageOptions.RequireReceiver);
                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("SendMessage with RequireReceiver logs 1 error when no receiver handles message", 1, errors.Count);
                if (errors.Count == 1)
                {
                    Check.That("SendMessage error names method OnMissingAction", errors[0].Message.Contains("OnMissingAction", StringComparison.Ordinal));
                    Check.That("SendMessage error names target MessengerNode", errors[0].Message.Contains("MessengerNode", StringComparison.Ordinal));
                }

                LogRecorder.Clear();
                target.SendMessage("OnHeartbeat", SendMessageOptions.RequireReceiver);
                Check.Equal("SendMessage with matching receiver dispatches method", "heartbeat:MessengerNode", string.Join(", ", receiver.Trace));
                Check.Equal("SendMessage with matching receiver logs 0 errors", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);
            }
            finally
            {
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(target);
            }
        }

        private static void TestInvokeRepeatingTimeScaleZeroPauseAndPermanentCancellationOnDestroy()
        {
            GameObject componentDestroyHost = new GameObject("ComponentDestroyHost");
            ArityEventReceiverBehaviour componentTarget = componentDestroyHost.AddComponent<ArityEventReceiverBehaviour>();
            GameObject objectDestroyHost = new GameObject("ObjectDestroyHost");
            ArityEventReceiverBehaviour objectTarget = objectDestroyHost.AddComponent<ArityEventReceiverBehaviour>();

            float savedTimeScale = Time.timeScale;
            InvokeScheduler.StopAll();
            try
            {
                Time.timeScale = 1f;
                componentTarget.InvokeRepeating("OnHeartbeat", 0.1f, 0.2f);
                objectTarget.InvokeRepeating("OnHeartbeat", 0.1f, 0.2f);

                Time.Advance(0.1);
                InvokeScheduler.PumpFrame();
                Check.Equal("componentTarget fires initial InvokeRepeating at t=0.1s", 1, componentTarget.Trace.Count);
                Check.Equal("objectTarget fires initial InvokeRepeating at t=0.1s", 1, objectTarget.Trace.Count);

                Time.timeScale = 0f;
                for (int step = 0; step < 8; step++)
                {
                    Time.Advance(0.1);
                    InvokeScheduler.PumpFrame();
                }

                Check.Equal("componentTarget does not fire while Time.timeScale == 0", 1, componentTarget.Trace.Count);
                Check.Equal("objectTarget does not fire while Time.timeScale == 0", 1, objectTarget.Trace.Count);

                Time.timeScale = 2f;
                Time.Advance(0.1);
                InvokeScheduler.PumpFrame();
                Check.Equal("componentTarget fires second invocation after scaled 0.2s under timeScale=2", 2, componentTarget.Trace.Count);
                Check.Equal("objectTarget fires second invocation after scaled 0.2s under timeScale=2", 2, objectTarget.Trace.Count);

                UnityEngine.Object.Destroy(componentTarget);
                UnityEngine.Object.Destroy(objectDestroyHost);

                Time.timeScale = 1f;
                for (int step = 0; step < 6; step++)
                {
                    Time.Advance(0.1);
                    InvokeScheduler.PumpFrame();
                }

                Check.Equal("Destroyed component permanently stops InvokeRepeating", 2, componentTarget.Trace.Count);
                Check.Equal("Destroyed GameObject permanently stops InvokeRepeating", 2, objectTarget.Trace.Count);
                Check.Equal("InvokeScheduler has zero active entries after destruction", 0, InvokeScheduler.ActiveCount);
            }
            finally
            {
                InvokeScheduler.StopAll();
                Time.timeScale = savedTimeScale;
                UnityEngine.Object.Destroy(componentDestroyHost);
                UnityEngine.Object.Destroy(objectDestroyHost);
            }
        }

        private static void TestDecimalLocaleDoesNotCorruptFloatEventTraces()
        {
            CultureInfo savedCulture = CultureInfo.CurrentCulture;
            GameObject owner = new GameObject("LocaleHost");
            ArityEventReceiverBehaviour receiver = owner.AddComponent<ArityEventReceiverBehaviour>();
            try
            {
                CultureInfo.CurrentCulture = new CultureInfo("de-DE");

                UnityEvent<string, int, float> onArity3 = new UnityEvent<string, int, float>("LocaleHost", "onArity3");
                onArity3.AddPersistentListener(owner, "onArity3", receiver, "OnArity3Dynamic");
                onArity3.AddListener((targetName, dmg, crit) =>
                    receiver.Trace.Add("runtime:1:" + targetName + ":" + dmg + ":" + crit.ToString(CultureInfo.InvariantCulture)));
                onArity3.Invoke("Boss", 50, 1.5f);

                Check.Equal(
                    "UnityEvent<T0, T1, T2> formats float event trace with invariant decimal dot under de-DE locale",
                    "persistent:arity3:Boss:50:1.5, runtime:1:Boss:50:1.5",
                    string.Join(", ", receiver.Trace));

                receiver.Trace.Clear();
                UnityEvent<string, int, float, bool> onArity4 =
                    new UnityEvent<string, int, float, bool>("LocaleHost", "onArity4");
                onArity4.AddPersistentListener(owner, "onArity4", receiver, "OnArity4Dynamic");
                onArity4.AddListener((evtName, code, elapsed, ok) =>
                    receiver.Trace.Add("runtime:1:" + evtName + ":" + code + ":" + elapsed.ToString(CultureInfo.InvariantCulture) + ":" + ok));
                onArity4.Invoke("Spawn", 7, 0.25f, true);

                Check.Equal(
                    "UnityEvent<T0, T1, T2, T3> formats float event trace with invariant decimal dot under de-DE locale",
                    "persistent:arity4:Spawn:7:0.25:True, runtime:1:Spawn:7:0.25:True",
                    string.Join(", ", receiver.Trace));
            }
            finally
            {
                CultureInfo.CurrentCulture = savedCulture;
                UnityEngine.Object.Destroy(owner);
            }
        }
    }
}
