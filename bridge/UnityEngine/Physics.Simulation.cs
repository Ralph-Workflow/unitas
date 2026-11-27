using System;

namespace UnityEngine
{
    public enum SimulationMode
    {
        FixedUpdate = 0,
        Update = 1,
        Script = 2,
    }

    public interface IPhysicsSimulationProvider
    {
        SimulationMode SimulationMode3D { get; set; }

        SimulationMode2D SimulationMode2D { get; set; }

        bool Simulate3D(float step);

        bool Simulate2D(float step);
    }

    public static partial class Physics
    {
        private static SimulationMode s_simulationMode = SimulationMode.FixedUpdate;

        public static SimulationMode simulationMode
        {
            get
            {
                if (Provider is IPhysicsSimulationProvider simProvider)
                {
                    s_simulationMode = simProvider.SimulationMode3D;
                }
                return s_simulationMode;
            }
            set
            {
                s_simulationMode = value;
                if (Provider is IPhysicsSimulationProvider simProvider)
                {
                    simProvider.SimulationMode3D = value;
                }
            }
        }

        public static bool autoSimulation
        {
            get => simulationMode != SimulationMode.Script;
            set => simulationMode = value ? SimulationMode.FixedUpdate : SimulationMode.Script;
        }

        public static void Simulate(float step)
        {
            if (simulationMode != SimulationMode.Script)
            {
                Debug.LogError("Physics.Simulate(...) can only be called when Physics.simulationMode is set to SimulationMode.Script.");
                return;
            }
            if (!float.IsFinite(step) || step <= 0f)
            {
                return;
            }
            if (Provider is IPhysicsSimulationProvider simProvider)
            {
                simProvider.Simulate3D(step);
            }
        }
    }
}
