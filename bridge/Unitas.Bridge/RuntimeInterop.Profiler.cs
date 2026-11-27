using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine.Profiling;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        private const uint ProfilerCaptureMagic = 0x50524643;
        private const uint ProfilerCaptureVersion = 1;

        private static readonly ProfilerHostImpl ProfilerHostInstance = new ProfilerHostImpl();
        private static bool _profilerRecording;

        public static void InstallProfilerHost()
        {
            Profiler.ProfilerHost = ProfilerHostInstance;
        }

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_profiler_start_capture();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_profiler_stop_capture();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern ulong unitas_runtime_profiler_capture_size();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr unitas_runtime_profiler_capture_buffer(ulong capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr unitas_runtime_profiler_scratch_buffer(ulong capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_profiler_begin_sample(ulong address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_profiler_end_sample();

        private sealed class ProfilerHostImpl : IProfilerHost
        {
            public bool IsRecording => _profilerRecording;

            public void StartCapture()
            {
                _profilerRecording = true;
                unitas_runtime_profiler_start_capture();
            }

            public void StopCapture()
            {
                _profilerRecording = false;
                unitas_runtime_profiler_stop_capture();
            }

            public void BeginSample(string name)
            {
                if (!_profilerRecording || string.IsNullOrEmpty(name))
                {
                    return;
                }

                byte[] bytes = Encoding.UTF8.GetBytes(name);
                IntPtr buffer = unitas_runtime_profiler_scratch_buffer((ulong)Math.Max(bytes.Length, 1));
                if (buffer == IntPtr.Zero)
                {
                    return;
                }

                Marshal.Copy(bytes, 0, buffer, bytes.Length);
                unitas_runtime_profiler_begin_sample((ulong)buffer.ToInt64(), (ulong)bytes.Length);
            }

            public void EndSample()
            {
                if (!_profilerRecording)
                {
                    return;
                }

                unitas_runtime_profiler_end_sample();
            }

            public ProfilerCapture GetCapture()
            {
                ulong size = unitas_runtime_profiler_capture_size();
                if (size == 0)
                {
                    return ProfilerCapture.Empty;
                }

                IntPtr buffer = unitas_runtime_profiler_capture_buffer(size);
                if (buffer == IntPtr.Zero)
                {
                    return ProfilerCapture.Empty;
                }

                byte[] bytes = new byte[(int)size];
                Marshal.Copy(buffer, bytes, 0, (int)size);
                return DeserializeProfilerCapture(bytes);
            }
        }

        private static ProfilerCapture DeserializeProfilerCapture(byte[] bytes)
        {
            using (MemoryStream stream = new MemoryStream(bytes))
            using (BinaryReader reader = new BinaryReader(stream))
            {
                if (stream.Length < 12)
                {
                    return ProfilerCapture.Empty;
                }

                if (reader.ReadUInt32() != ProfilerCaptureMagic)
                {
                    return ProfilerCapture.Empty;
                }

                if (reader.ReadUInt32() != ProfilerCaptureVersion)
                {
                    return ProfilerCapture.Empty;
                }

                _ = reader.ReadUInt32();
                List<ProfilerMarkerNode> allNodes = new List<ProfilerMarkerNode>();
                List<ProfilerMarkerNode> stack = new List<ProfilerMarkerNode>();
                ProfilerMarkerNode? root = null;

                while (stream.Position < stream.Length)
                {
                    int depth = (int)reader.ReadUInt32();
                    ulong durationNs = reader.ReadUInt64();
                    uint nameLength = reader.ReadUInt32();
                    byte[] nameBytes = reader.ReadBytes((int)nameLength);
                    string name = Encoding.UTF8.GetString(nameBytes);
                    ProfilerMarkerNode node = new ProfilerMarkerNode(name, durationNs, depth);
                    allNodes.Add(node);

                    if (depth == 0 && root == null)
                    {
                        root = node;
                        stack.Clear();
                        stack.Add(node);
                        continue;
                    }

                    while (stack.Count > depth && stack.Count > 1)
                    {
                        stack.RemoveAt(stack.Count - 1);
                    }

                    if (stack.Count > 0)
                    {
                        stack[stack.Count - 1].AddChild(node);
                    }

                    stack.Add(node);
                }

                return root != null
                    ? new ProfilerCapture(root, allNodes)
                    : ProfilerCapture.Empty;
            }
        }
    }
}
