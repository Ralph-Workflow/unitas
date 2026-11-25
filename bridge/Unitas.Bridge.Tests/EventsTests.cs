using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using Unitas.Bridge.Messaging;
using UnityEngine;
using UnityEngine.Events;

namespace Unitas.Bridge.Tests
{
    public sealed class EventReceiverBehaviour : MonoBehaviour, IUnitasMessageHandler
    {
        public Collection<string> Trace { get; } = new Collection<string>();

        public bool Dispatch(string message)
        {
            switch (message)
            {
                case "OnButtonPressed":
                    Trace.Add("persistent:void");
                    return true;
                case "OnPing":
                    Trace.Add("ping:" + gameObject.name);
                    return true;
                case "OnTick":
                    Trace.Add("tick:" + gameObject.name);
                    return true;
                case "OnRepeat":
                    Trace.Add("repeat:" + gameObject.name);
                    return true;
                default:
                    return false;
            }
        }

        public bool Dispatch<T>(string message, T argument)
        {
            switch (message)
            {
                case "OnButtonClickedWithLabel":
                    if (argument is string label)
                    {
                        Trace.Add("persistent:string:" + label);
                        return true;
                    }
                    return false;
                case "OnValueChanged":
                    Trace.Add("persistent:arg:" + (argument?.ToString() ?? "null"));
                    return true;
                case "OnDamage":
                    if (argument is int dmg)
                    {
                        Trace.Add("damage:" + gameObject.name + ":" + dmg);
                        return true;
                    }
                    return false;
                default:
                    return false;
            }
        }
    }

    public sealed class GlueRegisteredBehaviour : MonoBehaviour
    {
        public Collection<string> Trace { get; } = new Collection<string>();

        public void OnGluePing()
        {
            Trace.Add("glue:ping:" + gameObject.name);
        }

        public void OnGlueMessage(string payload)
        {
            Trace.Add("glue:msg:" + payload);
        }
    }

    public static class EventsTests
    {
        public static void Run()
        {
            TestUnityEventPersistentBeforeDynamicOrdering();
            TestUnityEventGenericPersistentAndDynamicListeners();
            TestUnityEventMissingOrRenamedPersistentMethodLogsWarningAndContinues();
            TestUnityEventCallStateAndRemoveAllListenersPreservesPersistent();
            TestSendMessageAndBroadcastMessageAndSendMessageUpwards();
            TestSendMessageWithGlueRegisteredMessageDispatcher();
            TestInvokeAndInvokeRepeatingScaledTimeAndPause();
            TestInvokeStopsPermanentlyWhenOwnerOrGameObjectDestroyed();
            TestCancelInvokeAndIsInvokingQueries();
        }

        private static void TestUnityEventPersistentBeforeDynamicOrdering()
        {
            GameObject buttonObject = new GameObject("StartButton");
            EventReceiverBehaviour receiver = buttonObject.AddComponent<EventReceiverBehaviour>();
            UnityEvent onClick = new UnityEvent("StartButton", "m_OnClick");

            onClick.AddListener(() => receiver.Trace.Add("runtimeListener:1"));
            onClick.AddPersistentStringListener(
                buttonObject,
                "m_OnClick",
                receiver,
                "OnButtonClickedWithLabel",
                "PlayGame");
            onClick.AddPersistentListener(
                buttonObject,
                "m_OnClick",
                receiver,
                "OnButtonPressed");
            onClick.AddListener(() => receiver.Trace.Add("runtimeListener:2"));

            Check.Equal("Persistent listener count is 2", 2, onClick.GetPersistentEventCount());
            Check.That("Persistent target 0 matches receiver", ReferenceEquals(onClick.GetPersistentTarget(0), receiver));
            Check.Equal("Persistent method name 0", "OnButtonClickedWithLabel", onClick.GetPersistentMethodName(0));
            Check.Equal("Persistent method name 1", "OnButtonPressed", onClick.GetPersistentMethodName(1));

            onClick.Invoke();

            Check.Equal(
                "UnityEvent runs persistent listeners in serialized order before runtime listeners",
                "persistent:string:PlayGame, persistent:void, runtimeListener:1, runtimeListener:2",
                string.Join(", ", receiver.Trace));

            UnityEngine.Object.Destroy(buttonObject);
        }

        private static void TestUnityEventGenericPersistentAndDynamicListeners()
        {
            GameObject sliderObject = new GameObject("VolumeSlider");
            EventReceiverBehaviour receiver = sliderObject.AddComponent<EventReceiverBehaviour>();
            UnityEvent<int> onValueChanged = new UnityEvent<int>("VolumeSlider", "onValueChanged");

            UnityAction<int> firstCallback = value => receiver.Trace.Add("runtimeListener:first:" + value);
            UnityAction<int> secondCallback = value => receiver.Trace.Add("runtimeListener:second:" + value);

            onValueChanged.AddListener(firstCallback);
            onValueChanged.AddListener(secondCallback);
            onValueChanged.AddPersistentListener(
                sliderObject,
                "onValueChanged",
                receiver,
                "OnValueChanged");

            onValueChanged.Invoke(75);
            Check.Equal(
                "UnityEvent<int> invokes persistent listener then runtime listeners",
                "persistent:arg:75, runtimeListener:first:75, runtimeListener:second:75",
                string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            onValueChanged.RemoveListener(firstCallback);
            onValueChanged.Invoke(50);
            Check.Equal(
                "RemoveListener removes only the specified runtime listener",
                "persistent:arg:50, runtimeListener:second:50",
                string.Join(", ", receiver.Trace));

            UnityEngine.Object.Destroy(sliderObject);
        }

        private static void TestUnityEventMissingOrRenamedPersistentMethodLogsWarningAndContinues()
        {
            GameObject buttonObject = new GameObject("SubmitButton");
            GameObject targetObject = new GameObject("MenuController");
            EventReceiverBehaviour receiver = targetObject.AddComponent<EventReceiverBehaviour>();
            UnityEvent onClick = new UnityEvent("SubmitButton", "m_OnClick");

            onClick.AddPersistentStringListener(
                buttonObject,
                "m_OnClick",
                receiver,
                "OnRenamedMethodThatNoLongerExists",
                "arg1");
            onClick.AddPersistentStringListener(
                buttonObject,
                "m_OnClick",
                receiver,
                "OnButtonClickedWithLabel",
                "Recovered");
            onClick.AddListener(() => receiver.Trace.Add("runtimeListener:afterMissing"));

            LogRecorder.Clear();
            try
            {
                onClick.Invoke();

                Check.Equal(
                    "Remaining persistent and runtime listeners still run after a missing persistent method",
                    "persistent:string:Recovered, runtimeListener:afterMissing",
                    string.Join(", ", receiver.Trace));

                IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
                Check.Equal("Missing persistent listener logs a warning", 1, warnings.Count);
                if (warnings.Count == 1)
                {
                    string msg = warnings[0].Message;
                    Check.That("Diagnostic names owning object SubmitButton", msg.Contains("SubmitButton", StringComparison.Ordinal));
                    Check.That("Diagnostic names event field m_OnClick", msg.Contains("m_OnClick", StringComparison.Ordinal));
                    Check.That("Diagnostic names target object MenuController", msg.Contains("MenuController", StringComparison.Ordinal));
                    Check.That("Diagnostic names missing method OnRenamedMethodThatNoLongerExists", msg.Contains("OnRenamedMethodThatNoLongerExists", StringComparison.Ordinal));
                }
            }
            finally
            {
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(buttonObject);
                UnityEngine.Object.Destroy(targetObject);
            }
        }

        private static void TestUnityEventCallStateAndRemoveAllListenersPreservesPersistent()
        {
            GameObject owner = new GameObject("ToggleOwner");
            EventReceiverBehaviour receiver = owner.AddComponent<EventReceiverBehaviour>();
            UnityEvent evt = new UnityEvent("ToggleOwner", "onToggle");

            evt.AddPersistentListener(owner, "onToggle", receiver, "OnButtonPressed", UnityEventCallState.RuntimeOnly);
            evt.AddListener(() => receiver.Trace.Add("runtimeListener:1"));

            evt.SetPersistentListenerState(0, UnityEventCallState.Off);
            evt.Invoke();
            Check.Equal("UnityEventCallState.Off skips persistent listener", "runtimeListener:1", string.Join(", ", receiver.Trace));

            receiver.Trace.Clear();
            evt.SetPersistentListenerState(0, UnityEventCallState.EditorAndRuntime);
            evt.RemoveAllListeners();
            evt.Invoke();
            Check.Equal(
                "RemoveAllListeners clears runtime listeners while keeping persistent listeners",
                "persistent:void",
                string.Join(", ", receiver.Trace));

            UnityEngine.Object.Destroy(owner);
        }

        private static void TestSendMessageAndBroadcastMessageAndSendMessageUpwards()
        {
            GameObject root = new GameObject("RootNode");
            GameObject childActive = new GameObject("ChildActive");
            GameObject childInactive = new GameObject("ChildInactive");
            childActive.transform.parent = root.transform;
            childInactive.transform.parent = root.transform;
            childInactive.SetActive(false);

            EventReceiverBehaviour rootReceiver = root.AddComponent<EventReceiverBehaviour>();
            EventReceiverBehaviour childActiveReceiver = childActive.AddComponent<EventReceiverBehaviour>();
            EventReceiverBehaviour childInactiveReceiver = childInactive.AddComponent<EventReceiverBehaviour>();

            LogRecorder.Clear();
            try
            {
                root.SendMessage("OnPing");
                Check.Equal("SendMessage dispatches to components on target GameObject", "ping:RootNode", string.Join(", ", rootReceiver.Trace));
                Check.Equal("SendMessage does not dispatch to child GameObject", 0, childActiveReceiver.Trace.Count);
                Check.Equal("SendMessage with matching receiver logs zero errors", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);

                rootReceiver.Trace.Clear();
                root.BroadcastMessage("OnPing");
                Check.Equal("BroadcastMessage reaches root", "ping:RootNode", string.Join(", ", rootReceiver.Trace));
                Check.Equal("BroadcastMessage reaches active child", "ping:ChildActive", string.Join(", ", childActiveReceiver.Trace));
                Check.Equal("BroadcastMessage skips inactive child", 0, childInactiveReceiver.Trace.Count);

                rootReceiver.Trace.Clear();
                childActiveReceiver.Trace.Clear();
                root.BroadcastMessage("OnDamage", 25);
                Check.Equal("BroadcastMessage<int> passes argument to root", "damage:RootNode:25", string.Join(", ", rootReceiver.Trace));
                Check.Equal("BroadcastMessage<int> passes argument to active child", "damage:ChildActive:25", string.Join(", ", childActiveReceiver.Trace));

                rootReceiver.Trace.Clear();
                childActiveReceiver.Trace.Clear();
                childActiveReceiver.SendMessageUpwards("OnPing");
                Check.Equal("SendMessageUpwards dispatches on child", "ping:ChildActive", string.Join(", ", childActiveReceiver.Trace));
                Check.Equal("SendMessageUpwards dispatches on parent root", "ping:RootNode", string.Join(", ", rootReceiver.Trace));

                LogRecorder.Clear();
                root.SendMessage("OnNonExistentMessage", SendMessageOptions.DontRequireReceiver);
                Check.Equal(
                    "SendMessage with DontRequireReceiver logs no error when unhandled",
                    0,
                    LogRecorder.OfSeverity(LogSeverity.Error).Count);

                root.SendMessage("OnNonExistentMessage", SendMessageOptions.RequireReceiver);
                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("SendMessage with RequireReceiver and no receiver logs 1 error", 1, errors.Count);
                if (errors.Count == 1)
                {
                    Check.That("SendMessage error names method OnNonExistentMessage", errors[0].Message.Contains("OnNonExistentMessage", StringComparison.Ordinal));
                    Check.That("SendMessage error names target RootNode", errors[0].Message.Contains("RootNode", StringComparison.Ordinal));
                }
            }
            finally
            {
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(root);
                UnityEngine.Object.Destroy(childActive);
                UnityEngine.Object.Destroy(childInactive);
            }
        }

        private static void TestSendMessageWithGlueRegisteredMessageDispatcher()
        {
            GameObject go = new GameObject("GlueObject");
            GlueRegisteredBehaviour script = go.AddComponent<GlueRegisteredBehaviour>();
            MessageDispatcher.Clear();
            LogRecorder.Clear();
            try
            {
                MessageDispatcher.RegisterHandler<GlueRegisteredBehaviour>(script, s => s.OnGluePing(), "OnGluePing");
                MessageDispatcher.RegisterHandler<GlueRegisteredBehaviour, string>(script, (s, arg) => s.OnGlueMessage(arg), "OnGlueMessage");

                go.SendMessage("OnGluePing");
                go.SendMessage("OnGlueMessage", "HelloGlue");

                Check.Equal(
                    "MessageDispatcher dispatches void and typed messages to glue-registered MonoBehaviour",
                    "glue:ping:GlueObject, glue:msg:HelloGlue",
                    string.Join(", ", script.Trace));
                Check.Equal("Glue-dispatched messages log zero errors", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);
            }
            finally
            {
                MessageDispatcher.Clear();
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(go);
            }
        }

        private static void TestInvokeAndInvokeRepeatingScaledTimeAndPause()
        {
            GameObject go = new GameObject("TimerObject");
            EventReceiverBehaviour script = go.AddComponent<EventReceiverBehaviour>();
            float savedScale = Time.timeScale;
            InvokeScheduler.StopAll();
            try
            {
                Time.timeScale = 1f;
                script.Invoke("OnTick", 0.2f);
                script.InvokeRepeating("OnRepeat", 0.1f, 0.15f);

                Check.Equal("IsInvoking() is true after scheduling", true, script.IsInvoking());
                Check.Equal("IsInvoking('OnTick') is true", true, script.IsInvoking("OnTick"));
                Check.Equal("IsInvoking('OnRepeat') is true", true, script.IsInvoking("OnRepeat"));

                Time.Advance(0.1);
                InvokeScheduler.PumpFrame();
                Check.Equal("At t=0.1s OnRepeat fires initial invocation", "repeat:TimerObject", string.Join(", ", script.Trace));

                Time.timeScale = 0f;
                for (int i = 0; i < 10; i++)
                {
                    Time.Advance(0.1);
                    InvokeScheduler.PumpFrame();
                }

                Check.Equal(
                    "While Time.timeScale == 0 neither Invoke nor InvokeRepeating advances or fires",
                    "repeat:TimerObject",
                    string.Join(", ", script.Trace));

                Time.timeScale = 1f;
                Time.Advance(0.1);
                InvokeScheduler.PumpFrame();
                Check.Equal(
                    "After restoring timeScale=1 and advancing 0.1s (total 0.2s scaled), OnTick fires",
                    "repeat:TimerObject, tick:TimerObject",
                    string.Join(", ", script.Trace));
                Check.Equal("One-shot OnTick is no longer invoking after firing", false, script.IsInvoking("OnTick"));
                Check.Equal("Repeating OnRepeat is still invoking", true, script.IsInvoking("OnRepeat"));

                Time.Advance(0.05);
                InvokeScheduler.PumpFrame();
                Check.Equal(
                    "At total 0.25s scaled (0.1 + 0.15), OnRepeat fires its second invocation",
                    "repeat:TimerObject, tick:TimerObject, repeat:TimerObject",
                    string.Join(", ", script.Trace));
            }
            finally
            {
                InvokeScheduler.StopAll();
                Time.timeScale = savedScale;
                UnityEngine.Object.Destroy(go);
            }
        }

        private static void TestInvokeStopsPermanentlyWhenOwnerOrGameObjectDestroyed()
        {
            GameObject goA = new GameObject("DestroyComponentTarget");
            EventReceiverBehaviour scriptA = goA.AddComponent<EventReceiverBehaviour>();
            GameObject goB = new GameObject("DestroyGameObjectTarget");
            EventReceiverBehaviour scriptB = goB.AddComponent<EventReceiverBehaviour>();
            float savedScale = Time.timeScale;
            InvokeScheduler.StopAll();
            try
            {
                Time.timeScale = 1f;
                scriptA.InvokeRepeating("OnRepeat", 0.05f, 0.05f);
                scriptB.InvokeRepeating("OnRepeat", 0.05f, 0.05f);

                Time.Advance(0.05);
                InvokeScheduler.PumpFrame();
                Check.Equal("scriptA fired once before destroy", 1, scriptA.Trace.Count);
                Check.Equal("scriptB fired once before destroy", 1, scriptB.Trace.Count);

                UnityEngine.Object.Destroy(scriptA);
                UnityEngine.Object.Destroy(goB);

                for (int i = 0; i < 5; i++)
                {
                    Time.Advance(0.05);
                    InvokeScheduler.PumpFrame();
                }

                Check.Equal("Destroyed MonoBehaviour never fires InvokeRepeating again", 1, scriptA.Trace.Count);
                Check.Equal("Destroyed GameObject owner never fires InvokeRepeating again", 1, scriptB.Trace.Count);
                Check.Equal("ActiveCount is 0 after owners are destroyed", 0, InvokeScheduler.ActiveCount);
            }
            finally
            {
                InvokeScheduler.StopAll();
                Time.timeScale = savedScale;
                UnityEngine.Object.Destroy(goA);
                UnityEngine.Object.Destroy(goB);
            }
        }

        private static void TestCancelInvokeAndIsInvokingQueries()
        {
            GameObject go = new GameObject("CancelTarget");
            EventReceiverBehaviour script = go.AddComponent<EventReceiverBehaviour>();
            float savedScale = Time.timeScale;
            InvokeScheduler.StopAll();
            try
            {
                Time.timeScale = 1f;
                Check.Equal("IsInvoking false when nothing scheduled", false, script.IsInvoking());

                script.InvokeRepeating("OnRepeat", 0.1f, 0.1f);
                script.Invoke("OnTick", 0.1f);
                Check.Equal("Both OnRepeat and OnTick are invoking", true, script.IsInvoking("OnRepeat") && script.IsInvoking("OnTick"));

                script.CancelInvoke("OnRepeat");
                Check.Equal("CancelInvoke('OnRepeat') stops OnRepeat", false, script.IsInvoking("OnRepeat"));
                Check.Equal("CancelInvoke('OnRepeat') leaves OnTick pending", true, script.IsInvoking("OnTick"));

                script.InvokeRepeating("OnRepeat", 0.1f, 0.1f);
                script.CancelInvoke();
                Check.Equal("CancelInvoke() cancels all pending invocations on script", false, script.IsInvoking());

                Time.Advance(0.2);
                InvokeScheduler.PumpFrame();
                Check.Equal("Cancelled invocations do not fire", 0, script.Trace.Count);
            }
            finally
            {
                InvokeScheduler.StopAll();
                Time.timeScale = savedScale;
                UnityEngine.Object.Destroy(go);
            }
        }
    }
}
