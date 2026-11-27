using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace Unitas.Bridge
{
    public static partial class RuntimeInterop
    {
        private const uint PlayerLoopTreeMagic = 0x314C5055;
        private const uint PlayerLoopTreeVersion = 1;

        private const uint CustomSystemBefore = 0;
        private const uint CustomSystemAfter = 1;

        // The managed player loop host. The runtime interop install puts it into the
        // UnityEngine player loop slot, so PlayerLoop.GetCurrentPlayerLoop() answers
        // with the native tree instead of the empty default.
        private static readonly PlayerLoopHostImpl PlayerLoopHostImpl = InstallPlayerLoopHost();

        private static PlayerLoopHostImpl InstallPlayerLoopHost()
        {
            PlayerLoopHostImpl host = new PlayerLoopHostImpl();
            UnityEngine.LowLevel.PlayerLoop.PlayerLoopHost = host;
            return host;
        }

        // The runtime the player loop surface answers for. The bridge attaches the
        // runtime it drives, the same way the time scale host is attached.
        private static ulong _playerLoopRuntime;

        /// <summary>Attaches the runtime the player loop surface talks to.</summary>
        public static void AttachPlayerLoopHost(ulong runtime)
        {
            _playerLoopRuntime = runtime;
        }

        /// <summary>Detaches the runtime the player loop surface talks to.</summary>
        public static void DetachPlayerLoopHost(ulong runtime)
        {
            if (_playerLoopRuntime == runtime)
            {
                _playerLoopRuntime = 0;
            }
        }

        /// <summary>The runtime the player loop surface talks to, zero when none is attached.</summary>
        public static ulong PlayerLoopRuntime => _playerLoopRuntime;

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern ulong unitas_runtime_player_loop_default_tree(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern ulong unitas_runtime_player_loop_tree(ulong runtime);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern ulong unitas_runtime_player_loop_buffer(ulong runtime, ulong capacity);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_player_loop_set_tree(ulong runtime, ulong address, ulong length);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_player_loop_register_custom_system(
            ulong runtime,
            ulong nameAddress,
            ulong nameLength,
            ulong typeAddress,
            ulong typeLength,
            ulong anchorAddress,
            ulong anchorLength,
            uint position,
            PlayerLoopCustomCallback callback,
            ulong userData);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int unitas_runtime_player_loop_unregister_custom_system(
            ulong runtime,
            ulong typeAddress,
            ulong typeLength);

        /// <summary>The native player loop callback. It marshals to the managed update function.</summary>
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void PlayerLoopCustomCallback(ulong userData, double deltaSeconds);

        private static readonly PlayerLoopCustomCallback CustomCallback = InvokeCustomSystem;

        // The managed custom systems the native loop calls back into. The native side
        // holds the function pointer, so the managed delegate stays rooted here.
        private static readonly Dictionary<ulong, PlayerLoopCustomSystem> CustomSystems =
            new Dictionary<ulong, PlayerLoopCustomSystem>();

        private static ulong _nextCustomSystemId = 1;

        private sealed class PlayerLoopCustomSystem
        {
            internal PlayerLoopCustomSystem(
                string systemType,
                UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction update)
            {
                SystemType = systemType;
                Update = update;
            }

            internal string SystemType { get; }

            internal UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction Update { get; }
        }

        /// <summary>
        /// Registers a custom player loop system with the attached runtime. The native
        /// loop calls the managed update function at the registered position every frame.
        /// </summary>
        public static bool RegisterPlayerLoopCustomSystem(
            string name,
            string systemType,
            string anchorType,
            bool afterAnchor,
            UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction update)
        {
            ulong runtime = _playerLoopRuntime;
            if (runtime == 0 || string.IsNullOrEmpty(systemType) || update == null)
            {
                return false;
            }

            if (FindCustomSystem(systemType) != null)
            {
                return false;
            }

            ulong id = _nextCustomSystemId++;
            CustomSystems[id] = new PlayerLoopCustomSystem(systemType, update);
            if (!RegisterCustomSystem(runtime, name, systemType, anchorType, afterAnchor, id))
            {
                CustomSystems.Remove(id);
                return false;
            }

            return true;
        }

        /// <summary>Removes a custom player loop system from the attached runtime.</summary>
        public static bool UnregisterPlayerLoopCustomSystem(string systemType)
        {
            ulong runtime = _playerLoopRuntime;
            if (runtime == 0 || string.IsNullOrEmpty(systemType))
            {
                return false;
            }

            ulong id = 0;
            bool found = false;
            foreach (KeyValuePair<ulong, PlayerLoopCustomSystem> entry in CustomSystems)
            {
                if (entry.Value.SystemType == systemType)
                {
                    id = entry.Key;
                    found = true;
                    break;
                }
            }

            if (!found)
            {
                return false;
            }

            CustomSystems.Remove(id);
            return UnregisterCustomSystem(runtime, systemType);
        }

        private static void InvokeCustomSystem(ulong userData, double deltaSeconds)
        {
            if (CustomSystems.TryGetValue(userData, out PlayerLoopCustomSystem system))
            {
                system.Update();
            }
        }

        private static UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction FindCustomSystem(
            string systemType)
        {
            foreach (KeyValuePair<ulong, PlayerLoopCustomSystem> entry in CustomSystems)
            {
                if (entry.Value.SystemType == systemType)
                {
                    return entry.Value.Update;
                }
            }

            return null;
        }

        private static bool RegisterCustomSystem(
            ulong runtime,
            string name,
            string systemType,
            string anchorType,
            bool afterAnchor,
            ulong userData)
        {
            byte[] nameBytes = Encoding.UTF8.GetBytes(name ?? ShortName(systemType));
            byte[] typeBytes = Encoding.UTF8.GetBytes(systemType);
            byte[] anchorBytes = Encoding.UTF8.GetBytes(anchorType ?? string.Empty);
            int total = nameBytes.Length + typeBytes.Length + anchorBytes.Length;
            IntPtr buffer = unitas_runtime_path_buffer(runtime, (ulong)Math.Max(total, 1));
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            long baseAddress = buffer.ToInt64();
            Marshal.Copy(nameBytes, 0, buffer, nameBytes.Length);
            Marshal.Copy(typeBytes, 0, (IntPtr)(baseAddress + nameBytes.Length), typeBytes.Length);
            Marshal.Copy(anchorBytes, 0, (IntPtr)(baseAddress + nameBytes.Length + typeBytes.Length), anchorBytes.Length);

            int code = unitas_runtime_player_loop_register_custom_system(
                runtime,
                (ulong)baseAddress,
                (ulong)nameBytes.Length,
                (ulong)(baseAddress + nameBytes.Length),
                (ulong)typeBytes.Length,
                (ulong)(baseAddress + nameBytes.Length + typeBytes.Length),
                (ulong)anchorBytes.Length,
                afterAnchor ? CustomSystemAfter : CustomSystemBefore,
                CustomCallback,
                userData);
            return code == 0;
        }

        private static bool UnregisterCustomSystem(ulong runtime, string systemType)
        {
            byte[] typeBytes = Encoding.UTF8.GetBytes(systemType);
            IntPtr buffer = unitas_runtime_path_buffer(runtime, (ulong)Math.Max(typeBytes.Length, 1));
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            Marshal.Copy(typeBytes, 0, buffer, typeBytes.Length);
            int code = unitas_runtime_player_loop_unregister_custom_system(
                runtime,
                (ulong)buffer.ToInt64(),
                (ulong)typeBytes.Length);
            return code == 0;
        }

        /// <summary>
        /// The managed player loop host. It serves the tree the native runtime holds.
        /// </summary>
        private sealed class PlayerLoopHostImpl : UnityEngine.LowLevel.IPlayerLoopHost
        {
            public UnityEngine.LowLevel.PlayerLoopSystem GetDefaultPlayerLoop()
            {
                ulong runtime = _playerLoopRuntime;
                if (runtime == 0)
                {
                    return default;
                }

                return ReadTree(runtime, unitas_runtime_player_loop_default_tree(runtime));
            }

            public UnityEngine.LowLevel.PlayerLoopSystem GetCurrentPlayerLoop()
            {
                ulong runtime = _playerLoopRuntime;
                if (runtime == 0)
                {
                    return default;
                }

                return ReadTree(runtime, unitas_runtime_player_loop_tree(runtime));
            }

            public void SetPlayerLoop(UnityEngine.LowLevel.PlayerLoopSystem loop)
            {
                ulong runtime = _playerLoopRuntime;
                if (runtime == 0)
                {
                    return;
                }

                SyncCustomSystems(runtime, loop);

                byte[] bytes = SerializeTree(loop);
                IntPtr buffer = unitas_runtime_path_buffer(runtime, (ulong)bytes.Length);
                if (buffer == IntPtr.Zero)
                {
                    return;
                }

                Marshal.Copy(bytes, 0, buffer, bytes.Length);
                if (unitas_runtime_player_loop_set_tree(runtime, (ulong)buffer.ToInt64(), (ulong)bytes.Length) != 0)
                {
                    UnityEngine.Debug.LogError(
                        "The player loop tree was rejected: " + unitas_runtime_error_message(runtime));
                }
            }
        }

        // Keeps the native custom systems in step with the managed tree: a system the
        // tree carries with an update function is registered with the native loop, and
        // a system the tree dropped is unregistered.
        private static void SyncCustomSystems(ulong runtime, UnityEngine.LowLevel.PlayerLoopSystem root)
        {
            Dictionary<string, UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction> wanted =
                new Dictionary<string, UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction>(StringComparer.Ordinal);
            CollectUpdateFunctions(root, wanted);

            List<string> stale = new List<string>();
            foreach (KeyValuePair<ulong, PlayerLoopCustomSystem> entry in CustomSystems)
            {
                if (!wanted.ContainsKey(entry.Value.SystemType))
                {
                    stale.Add(entry.Value.SystemType);
                }
            }

            foreach (string systemType in stale)
            {
                UnregisterPlayerLoopCustomSystem(systemType);
            }

            foreach (KeyValuePair<string, UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction> entry in wanted)
            {
                if (FindCustomSystem(entry.Key) != null)
                {
                    continue;
                }

                RegisterPlayerLoopCustomSystem(
                    ShortName(entry.Key),
                    entry.Key,
                    FindAnchor(runtime, entry.Key),
                    false,
                    entry.Value);
            }
        }

        private static void CollectUpdateFunctions(
            UnityEngine.LowLevel.PlayerLoopSystem node,
            Dictionary<string, UnityEngine.LowLevel.PlayerLoopSystem.PlayerLoopSystemUpdateFunction> wanted)
        {
            UnityEngine.LowLevel.PlayerLoopSystem[] children = node.subSystemList;
            if (children == null)
            {
                return;
            }

            foreach (UnityEngine.LowLevel.PlayerLoopSystem child in children)
            {
                if (child.updateDelegate != null && !string.IsNullOrEmpty(child.type))
                {
                    wanted[child.type] = child.updateDelegate;
                }

                CollectUpdateFunctions(child, wanted);
            }
        }

        // The native registration inserts the system next to the system it is anchored
        // to, so the anchor has to be a system the native tree holds. The position the
        // managed tree gives is the one that counts, because the tree is written after
        // the registration.
        private static string FindAnchor(ulong runtime, string systemType)
        {
            UnityEngine.LowLevel.PlayerLoopSystem native = ReadTree(runtime, unitas_runtime_player_loop_tree(runtime));
            List<string> types = new List<string>();
            CollectTypes(native, types);
            foreach (string type in types)
            {
                if (type != systemType)
                {
                    return type;
                }
            }

            return string.Empty;
        }

        private static void CollectTypes(UnityEngine.LowLevel.PlayerLoopSystem node, List<string> types)
        {
            UnityEngine.LowLevel.PlayerLoopSystem[] children = node.subSystemList;
            if (children == null)
            {
                return;
            }

            foreach (UnityEngine.LowLevel.PlayerLoopSystem child in children)
            {
                if (!string.IsNullOrEmpty(child.type))
                {
                    types.Add(child.type);
                }

                CollectTypes(child, types);
            }
        }

        private static UnityEngine.LowLevel.PlayerLoopSystem ReadTree(ulong runtime, ulong length)
        {
            if (length == 0)
            {
                return default;
            }

            IntPtr buffer = unitas_runtime_player_loop_buffer(runtime, length);
            if (buffer == IntPtr.Zero)
            {
                return default;
            }

            byte[] bytes = new byte[(int)length];
            Marshal.Copy(buffer, bytes, 0, (int)length);
            return DeserializeTree(bytes);
        }

        private static byte[] SerializeTree(UnityEngine.LowLevel.PlayerLoopSystem root)
        {
            using (MemoryStream stream = new MemoryStream())
            {
                using (BinaryWriter writer = new BinaryWriter(stream))
                {
                    writer.Write(PlayerLoopTreeMagic);
                    writer.Write(PlayerLoopTreeVersion);
                    writer.Write((uint)CountNodes(root));
                    WriteNode(writer, root);
                }

                return stream.ToArray();
            }
        }

        private static void WriteNode(BinaryWriter writer, UnityEngine.LowLevel.PlayerLoopSystem node)
        {
            string type = node.type ?? string.Empty;
            WriteString(writer, ShortName(type));
            WriteString(writer, type);
            UnityEngine.LowLevel.PlayerLoopSystem[] children =
                node.subSystemList ?? Array.Empty<UnityEngine.LowLevel.PlayerLoopSystem>();
            writer.Write((uint)children.Length);
            foreach (UnityEngine.LowLevel.PlayerLoopSystem child in children)
            {
                WriteNode(writer, child);
            }
        }

        private static void WriteString(BinaryWriter writer, string value)
        {
            byte[] bytes = Encoding.UTF8.GetBytes(value);
            writer.Write((uint)bytes.Length);
            writer.Write(bytes);
        }

        private static UnityEngine.LowLevel.PlayerLoopSystem DeserializeTree(byte[] bytes)
        {
            using (MemoryStream stream = new MemoryStream(bytes))
            using (BinaryReader reader = new BinaryReader(stream))
            {
                if (reader.ReadUInt32() != PlayerLoopTreeMagic)
                {
                    return default;
                }

                if (reader.ReadUInt32() != PlayerLoopTreeVersion)
                {
                    return default;
                }

                uint count = reader.ReadUInt32();
                UnityEngine.LowLevel.PlayerLoopSystem root = ReadNode(reader);
                if (CountNodes(root) != count)
                {
                    return default;
                }

                return root;
            }
        }

        private static UnityEngine.LowLevel.PlayerLoopSystem ReadNode(BinaryReader reader)
        {
            ReadString(reader);
            string type = ReadString(reader);
            uint childCount = reader.ReadUInt32();
            UnityEngine.LowLevel.PlayerLoopSystem[] children = new UnityEngine.LowLevel.PlayerLoopSystem[childCount];
            for (int index = 0; index < childCount; index++)
            {
                children[index] = ReadNode(reader);
            }

            return new UnityEngine.LowLevel.PlayerLoopSystem
            {
                type = type,
                subSystemList = children,
                updateDelegate = FindCustomSystem(type),
            };
        }

        private static string ReadString(BinaryReader reader)
        {
            uint length = reader.ReadUInt32();
            byte[] bytes = reader.ReadBytes((int)length);
            return Encoding.UTF8.GetString(bytes);
        }

        private static int CountNodes(UnityEngine.LowLevel.PlayerLoopSystem node)
        {
            int count = 1;
            UnityEngine.LowLevel.PlayerLoopSystem[] children = node.subSystemList;
            if (children != null)
            {
                foreach (UnityEngine.LowLevel.PlayerLoopSystem child in children)
                {
                    count += CountNodes(child);
                }
            }

            return count;
        }

        private static string ShortName(string type)
        {
            if (string.IsNullOrEmpty(type))
            {
                return string.Empty;
            }

            int index = type.LastIndexOf('.');
            return index < 0 ? type : type.Substring(index + 1);
        }
    }
}
