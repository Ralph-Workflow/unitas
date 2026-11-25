using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Unitas.Bridge;

namespace Unitas.Scripting
{
    public sealed class ParsedPluginMeta
    {
        public string Guid { get; set; } = string.Empty;

        public ulong GuidHi { get; set; }

        public ulong GuidLo { get; set; }

        public string FullPath { get; set; } = string.Empty;

        public string RelativePath { get; set; } = string.Empty;

        public string LibraryName { get; set; } = string.Empty;

        public bool IsNative { get; set; }

        public bool AnyPlatformEnabled { get; set; }

        public bool EditorEnabled { get; set; } = true;

        public string? EditorCpu { get; set; }

        public string? EditorOs { get; set; }

        public bool AutoReferenced { get; set; } = true;

        public bool ValidateReferences { get; set; } = true;

        public List<string> DefineConstraints { get; } = new List<string>();

        public List<(string Platform, bool Enabled, string? Cpu, string? Os)> Platforms { get; } =
            new List<(string Platform, bool Enabled, string? Cpu, string? Os)>();
    }

    public static class ProjectMetadataIndex
    {
        private static readonly string[] PluginExtensions =
        {
            ".dll", ".dylib", ".so", ".bundle", ".a", ".framework", ".aar", ".jar", ".jslib", ".jspre"
        };

        private static readonly string[] NativeOnlyExtensions =
        {
            ".dylib", ".so", ".bundle", ".a", ".framework", ".aar", ".jslib", ".jspre"
        };

        public static void ScanProjectMeta(
            string projectRoot,
            Dictionary<string, string> guidToPath,
            Dictionary<string, string> pathToGuid,
            List<ParsedPluginMeta> plugins)
        {
            if (TryScanProjectMetaViaNativeAbi(projectRoot, guidToPath, pathToGuid, plugins))
            {
                return;
            }

            List<string> metaFiles = new List<string>();
            string assetsDir = Path.Combine(projectRoot, "Assets");
            string packagesDir = Path.Combine(projectRoot, "Packages");
            if (Directory.Exists(assetsDir))
            {
                CollectMetaFiles(assetsDir, metaFiles);
            }

            if (Directory.Exists(packagesDir))
            {
                CollectMetaFiles(packagesDir, metaFiles);
            }

            metaFiles.Sort(StringComparer.Ordinal);

            for (int i = 0; i < metaFiles.Count; i++)
            {
                string metaPath = Path.GetFullPath(metaFiles[i]);
                string assetPath = metaPath.Substring(0, metaPath.Length - 5);
                string guid = AssemblyDiscovery.TryReadGuidFromMetaFile(metaPath);
                if (string.IsNullOrEmpty(guid))
                {
                    continue;
                }

                guidToPath[guid] = assetPath;
                pathToGuid[assetPath] = guid;

                if (IsPluginPath(assetPath))
                {
                    ParsedPluginMeta? plugin = ParsePluginMeta(projectRoot, assetPath, metaPath, guid);
                    if (plugin != null)
                    {
                        plugins.Add(plugin);
                    }
                }
            }
        }

        public static List<DiscoveredPluginRecord> EvaluatePlugins(
            List<ParsedPluginMeta> parsedPlugins,
            ScriptCompilationOptions options,
            HashSet<string> baseDefines,
            out Dictionary<string, DiscoveredPluginRecord> nativePluginsByName,
            out List<string> autoReferencedManagedDlls,
            out Dictionary<string, string> allCompatibleManagedDllsByFileName)
        {
            List<DiscoveredPluginRecord> records = new List<DiscoveredPluginRecord>();
            nativePluginsByName = new Dictionary<string, DiscoveredPluginRecord>(StringComparer.OrdinalIgnoreCase);
            autoReferencedManagedDlls = new List<string>();
            allCompatibleManagedDllsByFileName = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

            string targetPlatformToken = AssemblyDiscovery.GetUnityPlatformToken(options.TargetPlatform);
            string targetArch = options.TargetArchitecture;

            for (int i = 0; i < parsedPlugins.Count; i++)
            {
                ParsedPluginMeta p = parsedPlugins[i];
                bool constraintsPass = AssemblyDiscovery.EvaluateDefineConstraints(
                    p.DefineConstraints,
                    baseDefines,
                    out string _);

                bool isCompatible = true;
                bool isIncluded = constraintsPass;
                string? incompatibilityReason = null;

                if (!CheckPluginPlatformAndArch(
                    p,
                    targetPlatformToken,
                    targetArch,
                    options.IsEditor,
                    out bool enabledOnTarget,
                    out string? archDiagnostic))
                {
                    isCompatible = false;
                    isIncluded = false;
                    incompatibilityReason = archDiagnostic;
                }
                else if (!enabledOnTarget)
                {
                    isIncluded = false;
                }

                DiscoveredPluginRecord record = new DiscoveredPluginRecord(
                    p.Guid,
                    p.GuidHi,
                    p.GuidLo,
                    p.FullPath,
                    p.RelativePath,
                    p.LibraryName,
                    p.IsNative,
                    isCompatible,
                    isIncluded,
                    incompatibilityReason);

                records.Add(record);

                if (p.IsNative)
                {
                    nativePluginsByName[p.LibraryName] = record;
                    if (p.LibraryName.StartsWith("lib", StringComparison.OrdinalIgnoreCase) && p.LibraryName.Length > 3)
                    {
                        string withoutLib = p.LibraryName.Substring(3);
                        nativePluginsByName[withoutLib] = record;
                    }
                }
                else if (isCompatible && isIncluded && File.Exists(p.FullPath))
                {
                    string fileName = Path.GetFileName(p.FullPath);
                    allCompatibleManagedDllsByFileName[fileName] = p.FullPath;
                    allCompatibleManagedDllsByFileName[p.LibraryName] = p.FullPath;
                    if (p.AutoReferenced)
                    {
                        autoReferencedManagedDlls.Add(p.FullPath);
                    }
                }
            }

            return records;
        }

        private static bool CheckPluginPlatformAndArch(
            ParsedPluginMeta plugin,
            string targetPlatform,
            string targetArch,
            bool isEditor,
            out bool enabledOnTarget,
            out string? diagnostic)
        {
            enabledOnTarget = true;
            diagnostic = null;
            string activeTargetLabel = isEditor ? "Editor (" + targetPlatform + ")" : targetPlatform;

            if (isEditor)
            {
                if (!plugin.EditorEnabled && !plugin.AnyPlatformEnabled)
                {
                    enabledOnTarget = false;
                    diagnostic = "Plugin '" + plugin.LibraryName + "' is excluded for target '" + activeTargetLabel + "' (architecture '" + targetArch + "').";
                    return false;
                }

                if (!string.IsNullOrEmpty(plugin.EditorCpu) && !ArchMatches(plugin.EditorCpu, targetArch))
                {
                    enabledOnTarget = false;
                    diagnostic = "Plugin '" + plugin.LibraryName + "' targets architecture '" + plugin.EditorCpu + "' on target '" + activeTargetLabel + "', which is incompatible with running architecture '" + targetArch + "'.";
                    return false;
                }
            }

            for (int i = 0; i < plugin.Platforms.Count; i++)
            {
                (string plat, bool en, string? cpu, string? _) = plugin.Platforms[i];
                if (PlatformMatches(plat, targetPlatform))
                {
                    if (!en && !plugin.AnyPlatformEnabled)
                    {
                        enabledOnTarget = false;
                        diagnostic = "Plugin '" + plugin.LibraryName + "' is excluded on target '" + plat + "' (architecture '" + targetArch + "').";
                        return false;
                    }

                    if (!string.IsNullOrEmpty(cpu) && !ArchMatches(cpu, targetArch))
                    {
                        enabledOnTarget = false;
                        diagnostic = "Plugin '" + plugin.LibraryName + "' targets platform '" + plat + "' with architecture '" + cpu + "', which is incompatible with running architecture '" + targetArch + "'.";
                        return false;
                    }

                    return true;
                }
            }

            for (int i = 0; i < plugin.Platforms.Count; i++)
            {
                (string plat, bool en, string? cpu, string? _) = plugin.Platforms[i];
                if (en && !string.IsNullOrEmpty(cpu) && !ArchMatches(cpu, targetArch))
                {
                    enabledOnTarget = false;
                    diagnostic = "Plugin '" + plugin.LibraryName + "' targets platform '" + plat + "' with architecture '" + cpu + "', which is incompatible with target '" + activeTargetLabel + "' and architecture '" + targetArch + "'.";
                    return false;
                }
            }

            if (!isEditor && !plugin.AnyPlatformEnabled && plugin.Platforms.Count > 0)
            {
                enabledOnTarget = false;
                diagnostic = "Plugin '" + plugin.LibraryName + "' is not enabled for target '" + targetPlatform + "' (architecture '" + targetArch + "').";
                return false;
            }

            return true;
        }

        private static ParsedPluginMeta? ParsePluginMeta(
            string projectRoot,
            string assetPath,
            string metaPath,
            string guid)
        {
            string[] lines;
            try
            {
                lines = File.ReadAllLines(metaPath);
            }
            catch (IOException)
            {
                return null;
            }

            TrySplitGuid(guid, out ulong hi, out ulong lo);
            string ext = Path.GetExtension(assetPath);
            bool isNativeExt = false;
            for (int i = 0; i < NativeOnlyExtensions.Length; i++)
            {
                if (string.Equals(ext, NativeOnlyExtensions[i], StringComparison.OrdinalIgnoreCase))
                {
                    isNativeExt = true;
                    break;
                }
            }

            ParsedPluginMeta plugin = new ParsedPluginMeta
            {
                Guid = guid,
                GuidHi = hi,
                GuidLo = lo,
                FullPath = assetPath,
                RelativePath = GetRelativePathNormalized(projectRoot, assetPath),
                LibraryName = Path.GetFileNameWithoutExtension(assetPath),
                IsNative = isNativeExt
            };

            bool sawAnyPlatform = false;
            bool sawEditorEnabled = false;
            string? currentPlatform = null;
            bool? currentEnabled = null;
            string? currentCpu = null;
            string? currentOs = null;
            bool inSettings = false;
            bool inDefineConstraints = false;

            void FlushCurrent()
            {
                if (string.IsNullOrEmpty(currentPlatform))
                {
                    return;
                }

                string plat = currentPlatform;
                bool en = currentEnabled ?? false;
                string? cpu = currentCpu;
                string? os = currentOs;
                currentPlatform = null;
                currentEnabled = null;
                currentCpu = null;
                currentOs = null;

                if (string.Equals(plat, "Any", StringComparison.OrdinalIgnoreCase))
                {
                    plugin.AnyPlatformEnabled = en;
                    sawAnyPlatform = true;
                }
                else if (string.Equals(plat, "Editor", StringComparison.OrdinalIgnoreCase))
                {
                    plugin.EditorEnabled = en;
                    sawEditorEnabled = true;
                    if (!string.IsNullOrEmpty(cpu))
                    {
                        plugin.EditorCpu = cpu;
                    }

                    if (!string.IsNullOrEmpty(os))
                    {
                        plugin.EditorOs = os;
                    }

                    plugin.Platforms.Add((plat, en, cpu, os));
                }
                else
                {
                    plugin.Platforms.Add((plat, en, cpu, os));
                }
            }

            for (int i = 0; i < lines.Length; i++)
            {
                string rawLine = lines[i];
                string trimmed = rawLine.Trim();
                if (string.IsNullOrEmpty(trimmed))
                {
                    continue;
                }

                int indent = rawLine.Length - rawLine.TrimStart().Length;

                if (inDefineConstraints)
                {
                    if (trimmed.StartsWith("- ", StringComparison.Ordinal))
                    {
                        string c = Unquote(trimmed.Substring(2).Trim());
                        if (!string.IsNullOrEmpty(c))
                        {
                            plugin.DefineConstraints.Add(c);
                        }

                        continue;
                    }

                    if (indent <= 4)
                    {
                        inDefineConstraints = false;
                    }
                }

                if (string.Equals(trimmed, "defineConstraints:", StringComparison.Ordinal))
                {
                    inDefineConstraints = true;
                    continue;
                }

                if (trimmed.StartsWith("isPreloaded:", StringComparison.Ordinal))
                {
                    string val = trimmed.Substring("isPreloaded:".Length).Trim();
                    if (val == "1" || string.Equals(val, "true", StringComparison.OrdinalIgnoreCase))
                    {
                        plugin.IsNative = true;
                    }

                    continue;
                }

                if (trimmed.StartsWith("autoReferenced:", StringComparison.Ordinal))
                {
                    string val = trimmed.Substring("autoReferenced:".Length).Trim();
                    plugin.AutoReferenced = ParseBool(val, true);
                    continue;
                }

                if (trimmed.StartsWith("validateReferences:", StringComparison.Ordinal))
                {
                    string val = trimmed.Substring("validateReferences:".Length).Trim();
                    plugin.ValidateReferences = ParseBool(val, true);
                    continue;
                }

                if (trimmed.StartsWith("- first:", StringComparison.Ordinal))
                {
                    FlushCurrent();
                    inSettings = false;
                    string inline = trimmed.Substring("- first:".Length).Trim();
                    if (inline.StartsWith('{') && inline.EndsWith('}'))
                    {
                        string inner = inline.Substring(1, inline.Length - 2).Trim();
                        if (TrySplitKv(inner, out string k, out string v))
                        {
                            currentPlatform = string.Equals(k, "Any", StringComparison.OrdinalIgnoreCase)
                                ? "Any"
                                : (!string.IsNullOrEmpty(v) ? v : k);
                        }
                    }

                    continue;
                }

                if (string.Equals(trimmed, "first:", StringComparison.Ordinal))
                {
                    FlushCurrent();
                    inSettings = false;
                    continue;
                }

                if (currentPlatform == null && indent >= 4 && !inSettings)
                {
                    if (TrySplitKv(trimmed, out string k, out string v))
                    {
                        if (string.Equals(k, "Any", StringComparison.OrdinalIgnoreCase))
                        {
                            currentPlatform = "Any";
                            continue;
                        }

                        if (!IsMetaStructuralKey(k))
                        {
                            currentPlatform = !string.IsNullOrEmpty(v) ? v : k;
                            continue;
                        }
                    }
                }

                if (trimmed.StartsWith("- ", StringComparison.Ordinal))
                {
                    string afterDash = trimmed.Substring(2).Trim();
                    if (TrySplitKv(afterDash, out string k, out string v))
                    {
                        if (string.Equals(k, "Any", StringComparison.OrdinalIgnoreCase)
                            || k.StartsWith("Editor", StringComparison.OrdinalIgnoreCase)
                            || k.StartsWith("Standalone", StringComparison.OrdinalIgnoreCase))
                        {
                            FlushCurrent();
                            currentPlatform = !string.IsNullOrEmpty(v) ? v : k;
                            continue;
                        }
                    }
                }

                if (string.Equals(trimmed, "settings:", StringComparison.Ordinal))
                {
                    inSettings = true;
                    continue;
                }

                if (trimmed.StartsWith("settings:", StringComparison.Ordinal))
                {
                    inSettings = false;
                    continue;
                }

                if (currentPlatform != null)
                {
                    if (trimmed.StartsWith("enabled:", StringComparison.Ordinal))
                    {
                        string val = trimmed.Substring("enabled:".Length).Trim();
                        currentEnabled = ParseBool(val, false);
                        continue;
                    }

                    if (inSettings && TrySplitKv(trimmed, out string sk, out string sv))
                    {
                        if (string.Equals(sk, "CPU", StringComparison.OrdinalIgnoreCase)
                            && !string.IsNullOrEmpty(sv)
                            && !string.Equals(sv, "None", StringComparison.OrdinalIgnoreCase))
                        {
                            currentCpu = sv;
                        }
                        else if (string.Equals(sk, "OS", StringComparison.OrdinalIgnoreCase)
                            && !string.IsNullOrEmpty(sv)
                            && !string.Equals(sv, "None", StringComparison.OrdinalIgnoreCase))
                        {
                            currentOs = sv;
                        }
                    }
                }
            }

            FlushCurrent();

            if (!sawAnyPlatform && plugin.Platforms.Count == 0 && !sawEditorEnabled)
            {
                plugin.AnyPlatformEnabled = true;
                plugin.EditorEnabled = true;
            }
            else if (!sawEditorEnabled)
            {
                plugin.EditorEnabled = plugin.AnyPlatformEnabled;
            }

            for (int i = 0; i < plugin.Platforms.Count; i++)
            {
                string? cpu = plugin.Platforms[i].Cpu;
                if (!string.IsNullOrEmpty(cpu) && !string.Equals(cpu, "AnyCPU", StringComparison.OrdinalIgnoreCase))
                {
                    plugin.IsNative = true;
                }
            }

            if (!string.IsNullOrEmpty(plugin.EditorCpu)
                && !string.Equals(plugin.EditorCpu, "AnyCPU", StringComparison.OrdinalIgnoreCase))
            {
                plugin.IsNative = true;
            }

            return plugin;
        }

        private static bool TryScanProjectMetaViaNativeAbi(
            string projectRoot,
            Dictionary<string, string> guidToPath,
            Dictionary<string, string> pathToGuid,
            List<ParsedPluginMeta> plugins)
        {
            ulong runtime = 0;
            try
            {
                runtime = RuntimeInterop.unitas_runtime_new();
                if (runtime == 0)
                {
                    return false;
                }

                byte[] rootBytes = Encoding.UTF8.GetBytes(projectRoot);
                ulong capacity = (ulong)Math.Max(rootBytes.Length + 1, 65536);
                IntPtr ptr = RuntimeInterop.unitas_runtime_path_buffer(runtime, capacity);
                if (ptr == IntPtr.Zero)
                {
                    return false;
                }

                Marshal.Copy(rootBytes, 0, ptr, rootBytes.Length);
                Marshal.WriteByte(ptr, rootBytes.Length, 0);

                int byteLen = RuntimeInterop.unitas_runtime_script_index_build(
                    runtime,
                    ptr,
                    (ulong)rootBytes.Length);
                if (byteLen < 0)
                {
                    return false;
                }

                if (byteLen == 0)
                {
                    return true;
                }

                IntPtr readBufferPtr = RuntimeInterop.unitas_runtime_path_buffer(runtime, (ulong)(byteLen + 1));
                if (readBufferPtr == IntPtr.Zero)
                {
                    return false;
                }

                byte[] manifestBytes = new byte[byteLen];
                Marshal.Copy(readBufferPtr, manifestBytes, 0, byteLen);
                string manifest = Encoding.UTF8.GetString(manifestBytes);
                ParseNativeScriptIndexManifest(projectRoot, manifest, guidToPath, pathToGuid, plugins);
                return true;
            }
            catch (DllNotFoundException)
            {
                return false;
            }
            catch (EntryPointNotFoundException)
            {
                return false;
            }
            finally
            {
                if (runtime != 0)
                {
                    _ = RuntimeInterop.unitas_runtime_destroy(runtime);
                }
            }
        }

        private static void ParseNativeScriptIndexManifest(
            string projectRoot,
            string manifest,
            Dictionary<string, string> guidToPath,
            Dictionary<string, string> pathToGuid,
            List<ParsedPluginMeta> plugins)
        {
            string[] lines = manifest.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < lines.Length; i++)
            {
                string[] cols = lines[i].Split('\t');
                if (cols.Length >= 3 && string.Equals(cols[0], "GUID", StringComparison.Ordinal))
                {
                    string guid = cols[1];
                    string relPath = cols[2].Replace('/', Path.DirectorySeparatorChar);
                    string fullPath = Path.GetFullPath(Path.Combine(projectRoot, relPath));
                    guidToPath[guid] = fullPath;
                    pathToGuid[fullPath] = guid;
                }
                else if (cols.Length >= 12 && string.Equals(cols[0], "PLUGIN", StringComparison.Ordinal))
                {
                    string guid = cols[1];
                    TrySplitGuid(guid, out ulong hi, out ulong lo);
                    string libName = cols[4];
                    string relPath = cols[5];
                    string fullPath = Path.GetFullPath(Path.Combine(projectRoot, relPath.Replace('/', Path.DirectorySeparatorChar)));
                    bool isNative = string.Equals(cols[6], "native", StringComparison.OrdinalIgnoreCase);
                    bool anyPlat = cols[7] == "1";
                    bool editorEn = cols[8] == "1";
                    string[] editorSpec = cols[9].Split(':');
                    string? editorCpu = editorSpec.Length > 0 && !string.Equals(editorSpec[0], "AnyCPU", StringComparison.OrdinalIgnoreCase)
                        ? editorSpec[0]
                        : null;
                    string? editorOs = editorSpec.Length > 1 && !string.Equals(editorSpec[1], "AnyOS", StringComparison.OrdinalIgnoreCase)
                        ? editorSpec[1]
                        : null;
                    bool autoRef = cols[10] == "1";

                    ParsedPluginMeta plugin = new ParsedPluginMeta
                    {
                        Guid = guid,
                        GuidHi = hi,
                        GuidLo = lo,
                        FullPath = fullPath,
                        RelativePath = relPath,
                        LibraryName = libName,
                        IsNative = isNative,
                        AnyPlatformEnabled = anyPlat,
                        EditorEnabled = editorEn,
                        EditorCpu = editorCpu,
                        EditorOs = editorOs,
                        AutoReferenced = autoRef
                    };

                    if (!string.IsNullOrEmpty(cols[11]))
                    {
                        string[] platItems = cols[11].Split(',', StringSplitOptions.RemoveEmptyEntries);
                        for (int p = 0; p < platItems.Length; p++)
                        {
                            string[] parts = platItems[p].Split(':');
                            if (parts.Length >= 4)
                            {
                                string platName = parts[0];
                                bool platEn = parts[1] == "1";
                                string? cpu = !string.Equals(parts[2], "AnyCPU", StringComparison.OrdinalIgnoreCase) ? parts[2] : null;
                                string? os = !string.Equals(parts[3], "AnyOS", StringComparison.OrdinalIgnoreCase) ? parts[3] : null;
                                plugin.Platforms.Add((platName, platEn, cpu, os));
                            }
                        }
                    }

                    if (cols.Length >= 13 && !string.IsNullOrEmpty(cols[12]))
                    {
                        string[] dcs = cols[12].Split(',', StringSplitOptions.RemoveEmptyEntries);
                        for (int d = 0; d < dcs.Length; d++)
                        {
                            plugin.DefineConstraints.Add(dcs[d]);
                        }
                    }

                    plugins.Add(plugin);
                }
            }
        }

        public static bool TrySplitGuid(string guid, out ulong hi, out ulong lo)
        {
            hi = 0;
            lo = 0;
            string trimmed = guid.Trim();
            if (trimmed.Length != 32)
            {
                return false;
            }

            return ulong.TryParse(trimmed.Substring(0, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out hi)
                && ulong.TryParse(trimmed.Substring(16, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out lo);
        }

        private static bool IsMetaStructuralKey(string key)
        {
            return string.Equals(key, "second", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "enabled", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "settings", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "platformData", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "PluginImporter", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "externalObjects", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "serializedVersion", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "iconMap", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "executionOrder", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "userData", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "assetBundleName", StringComparison.OrdinalIgnoreCase)
                || string.Equals(key, "assetBundleVariant", StringComparison.OrdinalIgnoreCase);
        }

        private static bool ArchMatches(string configured, string target)
        {
            string conf = NormalizeArch(configured);
            string targ = NormalizeArch(target);
            if (conf == "anycpu" || conf == "none" || string.IsNullOrEmpty(conf))
            {
                return true;
            }

            return string.Equals(conf, targ, StringComparison.Ordinal);
        }

        private static string NormalizeArch(string arch)
        {
            string lower = arch.Trim().ToLowerInvariant();
            return lower switch
            {
                "x64" or "x86_64" or "amd64" => "x86_64",
                "arm64" or "aarch64" => "arm64",
                "x86" or "i386" or "i686" => "x86",
                "anycpu" or "any" => "anycpu",
                _ => lower
            };
        }

        private static bool PlatformMatches(string configured, string target)
        {
            if (string.Equals(configured, target, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            string conf = configured.ToLowerInvariant();
            string targ = target.ToLowerInvariant();
            if ((conf == "standaloneosx" || conf == "standaloneosxuniversal" || conf == "osx")
                && (targ == "standaloneosx" || targ == "osx" || targ == "macos"))
            {
                return true;
            }

            if ((conf == "standalonewindows64" || conf == "standalonewindows" || conf == "windows")
                && (targ == "standalonewindows64" || targ == "standalonewindows" || targ == "windows"))
            {
                return true;
            }

            if ((conf == "standalonelinux64" || conf == "linux")
                && (targ == "standalonelinux64" || targ == "linux"))
            {
                return true;
            }

            return false;
        }

        private static bool TrySplitKv(string line, out string key, out string val)
        {
            key = string.Empty;
            val = string.Empty;
            int colon = line.IndexOf(':');
            if (colon < 0)
            {
                return false;
            }

            key = Unquote(line.Substring(0, colon).Trim());
            val = Unquote(line.Substring(colon + 1).Trim());
            return true;
        }

        private static string Unquote(string text)
        {
            string trimmed = text.Trim();
            if (trimmed.Length >= 2
                && ((trimmed[0] == '"' && trimmed[trimmed.Length - 1] == '"')
                    || (trimmed[0] == '\'' && trimmed[trimmed.Length - 1] == '\'')))
            {
                return trimmed.Substring(1, trimmed.Length - 2);
            }

            return trimmed;
        }

        private static bool ParseBool(string text, bool defaultVal)
        {
            if (text == "1" || string.Equals(text, "true", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            if (text == "0" || string.Equals(text, "false", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            return defaultVal;
        }

        private static bool IsPluginPath(string path)
        {
            for (int i = 0; i < PluginExtensions.Length; i++)
            {
                if (path.EndsWith(PluginExtensions[i], StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            return false;
        }

        private static void CollectMetaFiles(string dir, List<string> metaFiles)
        {
            try
            {
                string[] files = Directory.GetFiles(dir, "*.meta", SearchOption.AllDirectories);
                for (int i = 0; i < files.Length; i++)
                {
                    metaFiles.Add(Path.GetFullPath(files[i]));
                }
            }
            catch (IOException)
            {
            }
        }

        public static string GetRelativePathNormalized(string root, string fullPath)
        {
            try
            {
                return Path.GetRelativePath(root, fullPath).Replace('\\', '/');
            }
            catch (ArgumentException)
            {
                return fullPath.Replace('\\', '/');
            }
        }
    }
}
