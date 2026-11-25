using System;
using System.Runtime.InteropServices;

namespace Unitas.PlayerHost
{
    public static unsafe class PlayerHostEntry
    {
        [UnmanagedCallersOnly]
        public static int UnitasPlayerInit(ulong runtimeHandle, byte* bundleRoot, byte* persistRoot)
        {
            string? bundle = bundleRoot == null ? null : Marshal.PtrToStringUTF8((IntPtr)bundleRoot);
            string? persist = persistRoot == null ? null : Marshal.PtrToStringUTF8((IntPtr)persistRoot);
            return PlayerHostCore.Init(runtimeHandle, bundle, persist);
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerTick(double deltaSeconds)
        {
            return PlayerHostCore.Tick(deltaSeconds);
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerShutdown()
        {
            return PlayerHostCore.Shutdown();
        }
    }
}
