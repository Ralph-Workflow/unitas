using System;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IPhysicsSimulationProvider
    {
        private SimulationMode _simulationMode3D = SimulationMode.FixedUpdate;
        private SimulationMode2D _simulationMode2D = SimulationMode2D.FixedUpdate;

        public SimulationMode SimulationMode3D
        {
            get
            {
                if (_isDisposed || _runtimeHandle == 0)
                {
                    return _simulationMode3D;
                }
                try
                {
                    int nativeMode = RuntimeInterop.unitas_runtime_physics_get_simulation_mode(_runtimeHandle);
                    if (nativeMode >= 0 && nativeMode <= 2)
                    {
                        _simulationMode3D = (SimulationMode)nativeMode;
                    }
                }
                catch (EntryPointNotFoundException)
                {
                }
                catch (DllNotFoundException)
                {
                }
                return _simulationMode3D;
            }
            set
            {
                _simulationMode3D = value;
                if (_isDisposed || _runtimeHandle == 0)
                {
                    return;
                }
                try
                {
                    RuntimeInterop.unitas_runtime_physics_set_simulation_mode(_runtimeHandle, (uint)value);
                }
                catch (EntryPointNotFoundException)
                {
                }
                catch (DllNotFoundException)
                {
                }
            }
        }

        public SimulationMode2D SimulationMode2D
        {
            get
            {
                if (_isDisposed || _runtimeHandle == 0)
                {
                    return _simulationMode2D;
                }
                try
                {
                    int nativeMode = RuntimeInterop.unitas_runtime_physics2d_get_simulation_mode(_runtimeHandle);
                    if (nativeMode >= 0 && nativeMode <= 2)
                    {
                        _simulationMode2D = (SimulationMode2D)nativeMode;
                    }
                }
                catch (EntryPointNotFoundException)
                {
                }
                catch (DllNotFoundException)
                {
                }
                return _simulationMode2D;
            }
            set
            {
                _simulationMode2D = value;
                if (_isDisposed || _runtimeHandle == 0)
                {
                    return;
                }
                try
                {
                    RuntimeInterop.unitas_runtime_physics2d_set_simulation_mode(_runtimeHandle, (uint)value);
                }
                catch (EntryPointNotFoundException)
                {
                }
                catch (DllNotFoundException)
                {
                }
            }
        }

        public bool Simulate3D(float step)
        {
            if (_isDisposed || _runtimeHandle == 0 || !float.IsFinite(step) || step <= 0f)
            {
                return false;
            }
            if (SimulationMode3D != SimulationMode.Script)
            {
                return false;
            }
            SyncRuntimeObjectsBeforeTick();
            int result = RuntimeInterop.unitas_runtime_physics_simulate(_runtimeHandle, step);
            if (result != 0)
            {
                return false;
            }
            SyncSimulatedTransforms();
            return true;
        }

        public bool Simulate2D(float step)
        {
            if (_isDisposed || _runtimeHandle == 0 || !float.IsFinite(step) || step <= 0f)
            {
                return false;
            }
            if (SimulationMode2D != SimulationMode2D.Script)
            {
                return false;
            }
            SyncRuntimeObjectsBeforeTick();
            int result = RuntimeInterop.unitas_runtime_physics2d_simulate(_runtimeHandle, step);
            if (result != 0)
            {
                return false;
            }
            SyncSimulatedTransforms();
            return true;
        }
    }
}
