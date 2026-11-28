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

        /// Writes the filtered log entries as UTF-8 JSON into the caller buffer.
        /// A null or undersized buffer copies nothing: the call reports
        /// BufferTooSmall and the required byte count through written, so the
        /// caller can size the buffer and call again.
        public static int CollectLogs(
            int severity,
            long afterFrame,
            int limit,
            IntPtr buffer,
            int capacity,
            IntPtr written)
        {
            if (written == IntPtr.Zero || limit <= 0)
            {
                return (int)PlayerHostStage.InvalidArgument;
            }

            byte[] payload = PlayerHostCore.CollectLogsJson(severity, afterFrame, limit);
            Marshal.WriteInt32(written, payload.Length);
            if (buffer == IntPtr.Zero || capacity < payload.Length)
            {
                return (int)PlayerHostStage.BufferTooSmall;
            }

            Marshal.Copy(payload, 0, buffer, payload.Length);
            return (int)PlayerHostStage.Ok;
        }

        [UnmanagedCallersOnly]
        public static int UnitasPlayerCollectLogs(
            int severity,
            long afterFrame,
            int limit,
            IntPtr buffer,
            int capacity,
            IntPtr written)
        {
            return CollectLogs(severity, afterFrame, limit, buffer, capacity, written);
        }
    }
}
