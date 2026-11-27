namespace UnityEngine.LowLevel
{
    /// <summary>
    /// One node of the player loop tree: the callback that runs it, the condition that
    /// gates it, the child systems it owns, and the name of the system it stands for.
    /// </summary>
    public struct PlayerLoopSystem
    {
        /// <summary>The callback the player loop invokes when it reaches this node.</summary>
        public delegate void PlayerLoopSystemUpdateFunction();

        /// <summary>The callback that decides whether this node runs in the current frame.</summary>
        public delegate bool PlayerLoopSystemLoopConditionFunction();

        /// <summary>The callback the player loop invokes when it reaches this node.</summary>
        public PlayerLoopSystemUpdateFunction updateDelegate { get; set; }

        /// <summary>The condition that decides whether this node runs in the current frame.</summary>
        public PlayerLoopSystemLoopConditionFunction loopConditionFunction { get; set; }

        /// <summary>The child systems this node owns.</summary>
        public PlayerLoopSystem[] subSystemList { get; set; }

        /// <summary>The name of the system this node stands for.</summary>
        public string type { get; set; }
    }
}
