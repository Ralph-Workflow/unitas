using System;
using System.Runtime.InteropServices;
using Unitas.Bridge;

namespace Unitas.PlayerHost
{
    public static class PlayerHostEntry
    {
        public static int Init(
            ulong runtimeHandle,
            IntPtr bundleRoot,
            IntPtr persistRoot,
            IntPtr startupScenePath)
        {
            string? bundle = bundleRoot == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(bundleRoot);
            string? persist = persistRoot == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(persistRoot);
            string? scene = startupScenePath == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(startupScenePath);
            return PlayerHostCore.Init(runtimeHandle, bundle, persist, scene);
        }

        public static int Tick(double deltaSeconds)
        {
            return PlayerHostCore.Tick(deltaSeconds);
        }

        public static int Shutdown()
        {
            return PlayerHostCore.Shutdown();
        }

        public static int InjectInput(ulong kind, ulong code, byte pressed, double x, double y)
        {
            BridgeHost? host = PlayerHostCore.Host;
            if (host == null)
            {
                return (int)PlayerHostStage.NotInitialized;
            }

            switch (kind)
            {
                case 0:
                    host.InjectKey((uint)code, pressed != 0);
                    return (int)PlayerHostStage.Ok;
                case 1:
                    host.InjectMouseButton((int)code, pressed != 0);
                    return (int)PlayerHostStage.Ok;
                case 2:
                    host.InjectPointerPosition(x, y);
                    return (int)PlayerHostStage.Ok;
                default:
                    return (int)PlayerHostStage.InvalidArgument;
            }
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerInit(
            ulong runtimeHandle,
            IntPtr bundleRoot,
            IntPtr persistRoot,
            IntPtr startupScenePath)
        {
            return Init(runtimeHandle, bundleRoot, persistRoot, startupScenePath);
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

        [UnmanagedCallersOnly]
        public static int UnitasPlayerInjectInput(ulong kind, ulong code, byte pressed, double x, double y)
        {
            return InjectInput(kind, code, pressed, x, y);
        }
    }
}
