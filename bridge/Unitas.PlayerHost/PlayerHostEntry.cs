using System;
using System.Runtime.InteropServices;

namespace Unitas.PlayerHost
{
    public static class PlayerHostEntry
    {
        // The managed entry points carry the native ABI; the unmanaged exports
        // forward to them so a managed host reaches the same code path without
        // reflection or function pointers.
        public static int Init(ulong runtimeHandle, IntPtr bundleRoot, IntPtr persistRoot)
        {
            string? bundle = bundleRoot == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(bundleRoot);
            string? persist = persistRoot == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(persistRoot);
            return PlayerHostCore.Init(runtimeHandle, bundle, persist);
        }

        public static int Tick(double deltaSeconds)
        {
            return PlayerHostCore.Tick(deltaSeconds);
        }

        public static int Shutdown()
        {
            return PlayerHostCore.Shutdown();
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerInit(ulong runtimeHandle, IntPtr bundleRoot, IntPtr persistRoot)
        {
            return Init(runtimeHandle, bundleRoot, persistRoot);
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerTick(double deltaSeconds)
        {
            return Tick(deltaSeconds);
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerShutdown()
        {
            return Shutdown();
        }
    }
}
