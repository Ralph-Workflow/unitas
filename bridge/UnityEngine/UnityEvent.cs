using System;
using System.Collections.Generic;

namespace UnityEngine.Events
{
    public delegate void UnityAction();

    public delegate void UnityAction<T0>(T0 arg0);

    public enum UnityEventCallState
    {
        Off = 0,
        EditorAndRuntime = 1,
        RuntimeOnly = 2
    }

    public abstract class UnityEventBase
    {
        private enum PersistentArgumentMode
        {
            Void = 0,
            Dynamic = 1,
            String = 2,
            Int = 3,
            Float = 4,
            Bool = 5,
            Object = 6
        }

        private sealed class PersistentListenerEntry
        {
            public PersistentListenerEntry(
                string ownerName,
                string fieldName,
                Object? target,
                IUnitasMessageHandler? handler,
                string methodName,
                UnityEventCallState callState,
                PersistentArgumentMode argumentMode,
                UnityAction? directVoidAction,
                string? stringArgument,
                int intArgument,
                float floatArgument,
                bool boolArgument,
                Object? objectArgument)
            {
                OwnerName = ownerName;
                FieldName = fieldName;
                Target = target;
                Handler = handler;
                MethodName = methodName;
                CallState = callState;
                ArgumentMode = argumentMode;
                DirectVoidAction = directVoidAction;
                StringArgument = stringArgument;
                IntArgument = intArgument;
                FloatArgument = floatArgument;
                BoolArgument = boolArgument;
                ObjectArgument = objectArgument;
            }

            public string OwnerName { get; }

            public string FieldName { get; }

            public Object? Target { get; }

            public IUnitasMessageHandler? Handler { get; }

            public string MethodName { get; }

            public UnityEventCallState CallState { get; set; }

            public PersistentArgumentMode ArgumentMode { get; }

            public UnityAction? DirectVoidAction { get; }

            public string? StringArgument { get; }

            public int IntArgument { get; }

            public float FloatArgument { get; }

            public bool BoolArgument { get; }

            public Object? ObjectArgument { get; }
        }

        private readonly List<PersistentListenerEntry> _persistentListeners = new List<PersistentListenerEntry>();

        public string OwnerName { get; set; } = string.Empty;

        public string FieldName { get; set; } = string.Empty;

        public int GetPersistentEventCount() => _persistentListeners.Count;

        public Object? GetPersistentTarget(int index)
        {
            if (index < 0 || index >= _persistentListeners.Count)
            {
                return null;
            }

            return _persistentListeners[index].Target;
        }

        public string GetPersistentMethodName(int index)
        {
            if (index < 0 || index >= _persistentListeners.Count)
            {
                return string.Empty;
            }

            return _persistentListeners[index].MethodName;
        }

        public UnityEventCallState GetPersistentListenerState(int index)
        {
            if (index < 0 || index >= _persistentListeners.Count)
            {
                throw new IndexOutOfRangeException("Persistent listener index is out of range.");
            }

            return _persistentListeners[index].CallState;
        }

        public void SetPersistentListenerState(int index, UnityEventCallState state)
        {
            if (index < 0 || index >= _persistentListeners.Count)
            {
                throw new IndexOutOfRangeException("Persistent listener index is out of range.");
            }

            _persistentListeners[index].CallState = state;
        }

        public void AddPersistentListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            string resolvedOwner = ResolveOwnerName(owner);
            AddPersistentListener(resolvedOwner, fieldName, target, target as IUnitasMessageHandler, methodName, callState);
        }

        public void AddPersistentListener(
            string ownerName,
            string fieldName,
            Object? target,
            IUnitasMessageHandler? handler,
            string methodName,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                string.IsNullOrEmpty(ownerName) ? OwnerName : ownerName,
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                handler ?? (target as IUnitasMessageHandler),
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.Dynamic,
                null,
                null,
                0,
                0f,
                false,
                null));
        }

        public void AddPersistentListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            UnityAction? callback,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                ResolveOwnerName(owner),
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                target as IUnitasMessageHandler,
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.Void,
                callback,
                null,
                0,
                0f,
                false,
                null));
        }

        public void AddPersistentStringListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            string? argument,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            AddPersistentStringListener(ResolveOwnerName(owner), fieldName, target, target as IUnitasMessageHandler, methodName, argument, callState);
        }

        public void AddPersistentStringListener(
            string ownerName,
            string fieldName,
            Object? target,
            IUnitasMessageHandler? handler,
            string methodName,
            string? argument,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                string.IsNullOrEmpty(ownerName) ? OwnerName : ownerName,
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                handler ?? (target as IUnitasMessageHandler),
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.String,
                null,
                argument,
                0,
                0f,
                false,
                null));
        }

        public void AddPersistentIntListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            int argument,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                ResolveOwnerName(owner),
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                target as IUnitasMessageHandler,
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.Int,
                null,
                null,
                argument,
                0f,
                false,
                null));
        }

        public void AddPersistentFloatListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            float argument,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                ResolveOwnerName(owner),
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                target as IUnitasMessageHandler,
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.Float,
                null,
                null,
                0,
                argument,
                false,
                null));
        }

        public void AddPersistentBoolListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            bool argument,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                ResolveOwnerName(owner),
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                target as IUnitasMessageHandler,
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.Bool,
                null,
                null,
                0,
                0f,
                argument,
                null));
        }

        public void AddPersistentObjectListener(
            Object? owner,
            string fieldName,
            Object? target,
            string methodName,
            Object? argument,
            UnityEventCallState callState = UnityEventCallState.RuntimeOnly)
        {
            _persistentListeners.Add(new PersistentListenerEntry(
                ResolveOwnerName(owner),
                string.IsNullOrEmpty(fieldName) ? FieldName : fieldName,
                target,
                target as IUnitasMessageHandler,
                methodName ?? string.Empty,
                callState,
                PersistentArgumentMode.Object,
                null,
                null,
                0,
                0f,
                false,
                argument));
        }

        public abstract void RemoveAllListeners();

        protected void InvokePersistentVoid()
        {
            List<PersistentListenerEntry> snapshot = new List<PersistentListenerEntry>(_persistentListeners);
            for (int index = 0; index < snapshot.Count; index++)
            {
                PersistentListenerEntry entry = snapshot[index];
                if (entry.CallState == UnityEventCallState.Off)
                {
                    continue;
                }

                InvokeSinglePersistentVoid(entry);
            }
        }

        protected void InvokePersistentDynamic<T0>(T0 arg0)
        {
            List<PersistentListenerEntry> snapshot = new List<PersistentListenerEntry>(_persistentListeners);
            for (int index = 0; index < snapshot.Count; index++)
            {
                PersistentListenerEntry entry = snapshot[index];
                if (entry.CallState == UnityEventCallState.Off)
                {
                    continue;
                }

                if (entry.ArgumentMode == PersistentArgumentMode.Dynamic)
                {
                    InvokeSinglePersistentDynamic(entry, arg0);
                }
                else
                {
                    InvokeSinglePersistentVoid(entry);
                }
            }
        }

        private void InvokeSinglePersistentVoid(PersistentListenerEntry entry)
        {
            if (entry.DirectVoidAction != null)
            {
                if (entry.Target is not null && entry.Target == null)
                {
                    ReportMissingPersistentListener(entry);
                    return;
                }

                try
                {
                    entry.DirectVoidAction();
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex, entry.Target);
                }

                return;
            }

            IUnitasMessageHandler? handler = ResolveHandler(entry);
            if (handler == null || string.IsNullOrEmpty(entry.MethodName))
            {
                ReportMissingPersistentListener(entry);
                return;
            }

            bool dispatched;
            try
            {
                switch (entry.ArgumentMode)
                {
                    case PersistentArgumentMode.String:
                        dispatched = handler.Dispatch(entry.MethodName, entry.StringArgument);
                        break;
                    case PersistentArgumentMode.Int:
                        dispatched = handler.Dispatch(entry.MethodName, entry.IntArgument);
                        break;
                    case PersistentArgumentMode.Float:
                        dispatched = handler.Dispatch(entry.MethodName, entry.FloatArgument);
                        break;
                    case PersistentArgumentMode.Bool:
                        dispatched = handler.Dispatch(entry.MethodName, entry.BoolArgument);
                        break;
                    case PersistentArgumentMode.Object:
                        dispatched = handler.Dispatch(entry.MethodName, entry.ObjectArgument);
                        break;
                    case PersistentArgumentMode.Void:
                    case PersistentArgumentMode.Dynamic:
                    default:
                        dispatched = handler.Dispatch(entry.MethodName);
                        break;
                }
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, entry.Target);
                return;
            }

            if (!dispatched)
            {
                ReportMissingPersistentListener(entry);
            }
        }

        private void InvokeSinglePersistentDynamic<T0>(PersistentListenerEntry entry, T0 arg0)
        {
            IUnitasMessageHandler? handler = ResolveHandler(entry);
            if (handler == null || string.IsNullOrEmpty(entry.MethodName))
            {
                ReportMissingPersistentListener(entry);
                return;
            }

            bool dispatched;
            try
            {
                dispatched = handler.Dispatch(entry.MethodName, arg0) || handler.Dispatch(entry.MethodName);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, entry.Target);
                return;
            }

            if (!dispatched)
            {
                ReportMissingPersistentListener(entry);
            }
        }

        private static IUnitasMessageHandler? ResolveHandler(PersistentListenerEntry entry)
        {
            if (entry.Target is not null && entry.Target == null)
            {
                return null;
            }

            if (entry.Handler != null)
            {
                return entry.Handler;
            }

            if (entry.Target is MonoBehaviour behaviour)
            {
                return GameObject.ResolveMessageHandler(behaviour);
            }

            if (entry.Target is GameObject gameObject)
            {
                IReadOnlyList<Component> components = gameObject.Components;
                for (int i = 0; i < components.Count; i++)
                {
                    if (components[i] is MonoBehaviour mb && !mb.IsDestroyed)
                    {
                        IUnitasMessageHandler? resolved = GameObject.ResolveMessageHandler(mb);
                        if (resolved != null)
                        {
                            return resolved;
                        }
                    }
                }
            }

            return entry.Target as IUnitasMessageHandler;
        }

        private string ResolveOwnerName(Object? owner)
        {
            if (owner != null && !string.IsNullOrEmpty(owner.name))
            {
                return owner.name;
            }

            return string.IsNullOrEmpty(OwnerName) ? "UnknownOwner" : OwnerName;
        }

        private void ReportMissingPersistentListener(PersistentListenerEntry entry)
        {
            string ownerLabel = string.IsNullOrEmpty(entry.OwnerName)
                ? (string.IsNullOrEmpty(OwnerName) ? "UnknownOwner" : OwnerName)
                : entry.OwnerName;
            string fieldLabel = string.IsNullOrEmpty(entry.FieldName)
                ? (string.IsNullOrEmpty(FieldName) ? "UnknownEvent" : FieldName)
                : entry.FieldName;
            string targetLabel = entry.Target != null && !string.IsNullOrEmpty(entry.Target.name)
                ? entry.Target.name
                : "MissingTarget";
            string methodLabel = string.IsNullOrEmpty(entry.MethodName) ? "MissingMethod" : entry.MethodName;

            Debug.LogWarning(
                "UnityEvent persistent listener could not be invoked: owner='"
                + ownerLabel
                + "', field='"
                + fieldLabel
                + "', target='"
                + targetLabel
                + "', method='"
                + methodLabel
                + "'.",
                entry.Target);
        }
    }

    public class UnityEvent : UnityEventBase
    {
        private readonly List<UnityAction> _dynamicListeners = new List<UnityAction>();

        public UnityEvent()
        {
        }

        public UnityEvent(string ownerName, string fieldName)
        {
            OwnerName = ownerName ?? string.Empty;
            FieldName = fieldName ?? string.Empty;
        }

        public void AddListener(UnityAction call)
        {
            if (call == null)
            {
                throw new ArgumentNullException(nameof(call));
            }

            _dynamicListeners.Add(call);
        }

        public void RemoveListener(UnityAction call)
        {
            if (call == null)
            {
                return;
            }

            _dynamicListeners.Remove(call);
        }

        public override void RemoveAllListeners()
        {
            _dynamicListeners.Clear();
        }

        public void Invoke()
        {
            InvokePersistentVoid();
            List<UnityAction> snapshot = new List<UnityAction>(_dynamicListeners);
            for (int index = 0; index < snapshot.Count; index++)
            {
                UnityAction call = snapshot[index];
                if (!_dynamicListeners.Contains(call))
                {
                    continue;
                }

                try
                {
                    call();
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex);
                }
            }
        }
    }

    public class UnityEvent<T0> : UnityEventBase
    {
        private readonly List<UnityAction<T0>> _dynamicListeners = new List<UnityAction<T0>>();

        public UnityEvent()
        {
        }

        public UnityEvent(string ownerName, string fieldName)
        {
            OwnerName = ownerName ?? string.Empty;
            FieldName = fieldName ?? string.Empty;
        }

        public void AddListener(UnityAction<T0> call)
        {
            if (call == null)
            {
                throw new ArgumentNullException(nameof(call));
            }

            _dynamicListeners.Add(call);
        }

        public void RemoveListener(UnityAction<T0> call)
        {
            if (call == null)
            {
                return;
            }

            _dynamicListeners.Remove(call);
        }

        public override void RemoveAllListeners()
        {
            _dynamicListeners.Clear();
        }

        public void Invoke(T0 arg0)
        {
            InvokePersistentDynamic(arg0);
            List<UnityAction<T0>> snapshot = new List<UnityAction<T0>>(_dynamicListeners);
            for (int index = 0; index < snapshot.Count; index++)
            {
                UnityAction<T0> call = snapshot[index];
                if (!_dynamicListeners.Contains(call))
                {
                    continue;
                }

                try
                {
                    call(arg0);
                }
                catch (Exception ex) when (!(ex is OutOfMemoryException))
                {
                    Debug.LogException(ex);
                }
            }
        }
    }
}
