using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using UnityEngine;

[assembly: InternalsVisibleTo("Unitas.Bridge.Tests")]

namespace Unitas.Bridge
{
    /// <summary>
    /// Managed side of contract C2: one native call dispatches a whole script phase batch.
    /// The native export unitas_runtime_set_script_batch_dispatch may be absent on older
    /// runtimes; registration failure leaves the per-script callback path fully active.
    /// </summary>
    public sealed partial class BridgeHost
    {
        private const uint ScriptBatchKindFixedUpdate = 0;
        private const uint ScriptBatchKindUpdate = 1;
        private const uint ScriptBatchKindLateUpdate = 2;

        private static BridgeHost? _scriptBatchDispatchHost;

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        internal static unsafe void ScriptBatchDispatch(uint kind, ulong* handles, double* deltas, ulong len)
        {
            try
            {
                _scriptBatchDispatchHost?.DispatchScriptBatch(kind, handles, deltas, len);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex);
            }
        }

        private unsafe void RegisterScriptBatchDispatch()
        {
            _scriptBatchDispatchHost = this;
            try
            {
                RuntimeInterop.unitas_runtime_set_script_batch_dispatch(
                    _runtimeHandle,
                    (IntPtr)(delegate* unmanaged[Cdecl]<uint, ulong*, double*, ulong, void>)&ScriptBatchDispatch);
            }
            catch (EntryPointNotFoundException)
            {
            }
            catch (DllNotFoundException)
            {
            }
        }

        // Mirrors OnFixedUpdate/OnUpdate/OnLateUpdate: input sync once per batch, scripts run in
        // the order native supplied, per-script faults stay inside InvokeWithOrigin, and the phase
        // pump runs after the batch. deltas is the per-script wall-clock delta reserved for future
        // use; managed phase methods take no delta argument today.
        internal unsafe void DispatchScriptBatch(uint kind, ulong* handles, double* deltas, ulong len)
        {
            if (len == 0)
            {
                return;
            }

            EnsureInputSystemUpdatedForTick();
            SyncSimulatedTransforms();
            for (ulong index = 0; index < len; index++)
            {
                if (!_scripts.TryGetValue(handles[index], out MonoBehaviour? script) || !IsEligible(script))
                {
                    continue;
                }

                MonoBehaviour target = script!;
                switch (kind)
                {
                    case ScriptBatchKindFixedUpdate:
                        InvokeWithOrigin(target, target.FixedUpdate);
                        break;
                    case ScriptBatchKindUpdate:
                        InvokeWithOrigin(target, target.Update);
                        break;
                    case ScriptBatchKindLateUpdate:
                        InvokeWithOrigin(target, target.LateUpdate);
                        break;
                    default:
                        return;
                }
            }

            if (kind == ScriptBatchKindFixedUpdate)
            {
                PumpFixedSafely();
            }
            else if (kind == ScriptBatchKindLateUpdate)
            {
                PumpFrameSafely();
            }
        }

        // Managed-only construction for batch-dispatch unit tests; never boots the native
        // runtime, so the native-wired readonly fields stay unset and unused on this path.
#pragma warning disable CS8618
        private BridgeHost(bool managedOnly)
        {
            _ = managedOnly;
            _scriptBatchDispatchHost = this;
        }
#pragma warning restore CS8618

        internal static BridgeHost CreateManagedOnlyForBatchDispatch()
        {
            return new BridgeHost(managedOnly: true);
        }

        internal ulong RegisterScriptForManagedDispatch(MonoBehaviour script)
        {
            ulong handle = _nextScriptHandle++;
            _scripts[handle] = script;
            return handle;
        }
    }
}
