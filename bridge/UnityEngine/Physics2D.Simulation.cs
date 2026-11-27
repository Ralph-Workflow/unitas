using System;

namespace UnityEngine
{
    public enum SimulationMode2D
    {
        FixedUpdate = 0,
        Update = 1,
        Script = 2,
    }

    public static partial class Physics2D
    {
        private static SimulationMode2D s_simulationMode = SimulationMode2D.FixedUpdate;

        public static SimulationMode2D simulationMode
        {
            get
            {
                if (Provider is IPhysicsSimulationProvider simProvider)
                {
                    s_simulationMode = simProvider.SimulationMode2D;
                }
                return s_simulationMode;
            }
            set
            {
                s_simulationMode = value;
                if (Provider is IPhysicsSimulationProvider simProvider)
                {
                    simProvider.SimulationMode2D = value;
                }
            }
        }

        public static bool autoSimulation
        {
            get => simulationMode != SimulationMode2D.Script;
            set => simulationMode = value ? SimulationMode2D.FixedUpdate : SimulationMode2D.Script;
        }

        public static bool Simulate(float step)
        {
            if (simulationMode != SimulationMode2D.Script)
            {
                Debug.LogError("Physics2D.Simulate(...) can only be called when Physics2D.simulationMode is set to SimulationMode2D.Script.");
                return false;
            }
            if (!float.IsFinite(step) || step <= 0f)
            {
                return false;
            }
            if (Provider is IPhysicsSimulationProvider simProvider)
            {
                return simProvider.Simulate2D(step);
            }
            return true;
        }
    }
}
