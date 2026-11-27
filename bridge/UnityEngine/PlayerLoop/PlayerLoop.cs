namespace UnityEngine.LowLevel
{
    /// <summary>
    /// The player loop surface the managed slice reads and writes. The bridge implements it
    /// against the runtime ABI; tests implement it with a fake.
    /// </summary>
    public interface IPlayerLoopHost
    {
        /// <summary>The player loop the runtime starts with.</summary>
        PlayerLoopSystem GetDefaultPlayerLoop();

        /// <summary>The player loop the runtime runs right now.</summary>
        PlayerLoopSystem GetCurrentPlayerLoop();

        /// <summary>Replaces the player loop the runtime runs.</summary>
        void SetPlayerLoop(PlayerLoopSystem loop);
    }

    /// <summary>Holds the host the managed player loop talks to.</summary>
    public static class PlayerLoop
    {
        /// <summary>
        /// The host that serves the player loop. It is null when no runtime is attached, and
        /// the getters then report an empty player loop.
        /// </summary>
        public static IPlayerLoopHost? PlayerLoopHost { get; set; }

        /// <summary>The player loop the host starts with, empty when no host is installed.</summary>
        public static PlayerLoopSystem GetDefaultPlayerLoop()
        {
            IPlayerLoopHost? host = PlayerLoopHost;
            return host != null ? host.GetDefaultPlayerLoop() : default;
        }

        /// <summary>The player loop the host runs right now, empty when no host is installed.</summary>
        public static PlayerLoopSystem GetCurrentPlayerLoop()
        {
            IPlayerLoopHost? host = PlayerLoopHost;
            return host != null ? host.GetCurrentPlayerLoop() : default;
        }

        /// <summary>Replaces the player loop the host runs; does nothing when no host is installed.</summary>
        public static void SetPlayerLoop(PlayerLoopSystem loop)
        {
            PlayerLoopHost?.SetPlayerLoop(loop);
        }
    }
}
