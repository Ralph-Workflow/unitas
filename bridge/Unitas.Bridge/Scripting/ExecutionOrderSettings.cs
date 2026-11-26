using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Scripting
{
    public static class ExecutionOrderSettings
    {
        public const string SettingsDirectoryName = "ProjectSettings";
        public const string AssetFileName = "ScriptExecutionOrder.asset";

        private const string ScriptKey = "m_Script:";
        private const string ExecutionOrderKey = "m_ExecutionOrder:";
        private const string GuidKey = "guid:";
        private const int GuidLength = 32;

        private static readonly List<Func<MonoBehaviour, bool>> s_registeredMatchers =
            new List<Func<MonoBehaviour, bool>>();

        public static string GetAssetPath(string projectRoot)
        {
            ArgumentNullException.ThrowIfNull(projectRoot);
            return Path.Combine(projectRoot, SettingsDirectoryName, AssetFileName);
        }

        public static int LoadProject(string? projectRoot)
        {
            Clear();
            if (string.IsNullOrEmpty(projectRoot))
            {
                return 0;
            }

            string assetPath = GetAssetPath(projectRoot);
            if (!File.Exists(assetPath) || !TryParse(assetPath, out Dictionary<string, int> orders))
            {
                return 0;
            }

            int registered = 0;
            foreach (KeyValuePair<string, int> entry in orders)
            {
                if (!TryParseGuid(entry.Key, out ulong guidHi, out ulong guidLo))
                {
                    continue;
                }

                Func<MonoBehaviour, bool> matcher = instance => MatchesGuid(instance, guidHi, guidLo);
                MonoBehaviour.SetExecutionOrderOverride(matcher, entry.Value);
                s_registeredMatchers.Add(matcher);
                registered++;
            }

            return registered;
        }

        public static void Clear()
        {
            for (int index = 0; index < s_registeredMatchers.Count; index++)
            {
                MonoBehaviour.RemoveExecutionOrderOverride(s_registeredMatchers[index]);
            }

            s_registeredMatchers.Clear();
        }

        public static bool TryParse(string assetPath, out Dictionary<string, int> orders)
        {
            orders = new Dictionary<string, int>(StringComparer.OrdinalIgnoreCase);
            if (string.IsNullOrEmpty(assetPath) || !File.Exists(assetPath))
            {
                return false;
            }

            string[] lines;
            try
            {
                lines = File.ReadAllLines(assetPath);
            }
            catch (IOException)
            {
                return false;
            }
            catch (UnauthorizedAccessException)
            {
                return false;
            }

            string? pendingGuid = null;
            int? pendingOrder = null;
            for (int index = 0; index < lines.Length; index++)
            {
                string trimmed = lines[index].Trim();
                if (trimmed.StartsWith("---", StringComparison.Ordinal))
                {
                    Flush(orders, ref pendingGuid, ref pendingOrder);
                    continue;
                }

                if (trimmed.StartsWith(ScriptKey, StringComparison.Ordinal))
                {
                    pendingGuid = ExtractGuid(trimmed);
                    continue;
                }

                if (trimmed.StartsWith(ExecutionOrderKey, StringComparison.Ordinal))
                {
                    pendingOrder = ExtractExecutionOrder(trimmed);
                }
            }

            Flush(orders, ref pendingGuid, ref pendingOrder);
            return true;
        }

        private static void Flush(Dictionary<string, int> orders, ref string? pendingGuid, ref int? pendingOrder)
        {
            if (!string.IsNullOrEmpty(pendingGuid) && pendingOrder.HasValue)
            {
                orders[pendingGuid] = pendingOrder.Value;
            }

            pendingGuid = null;
            pendingOrder = null;
        }

        private static string? ExtractGuid(string line)
        {
            int keyIndex = line.IndexOf(GuidKey, StringComparison.Ordinal);
            if (keyIndex < 0)
            {
                return null;
            }

            int start = keyIndex + GuidKey.Length;
            while (start < line.Length && char.IsWhiteSpace(line[start]))
            {
                start++;
            }

            if (start + GuidLength > line.Length)
            {
                return null;
            }

            ReadOnlySpan<char> candidate = line.AsSpan(start, GuidLength);
            for (int index = 0; index < candidate.Length; index++)
            {
                if (!IsHexDigit(candidate[index]))
                {
                    return null;
                }
            }

            return new string(candidate).ToLowerInvariant();
        }

        private static int? ExtractExecutionOrder(string line)
        {
            int separator = line.IndexOf(':', StringComparison.Ordinal);
            if (separator < 0)
            {
                return null;
            }

            ReadOnlySpan<char> value = line.AsSpan(separator + 1).Trim();
            return int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int order) ? order : null;
        }

        private static bool IsHexDigit(char value)
        {
            return (value >= '0' && value <= '9')
                || (value >= 'a' && value <= 'f')
                || (value >= 'A' && value <= 'F');
        }

        private static bool TryParseGuid(string guidHex, out ulong guidHi, out ulong guidLo)
        {
            guidHi = 0;
            guidLo = 0;
            if (guidHex.Length != GuidLength)
            {
                return false;
            }

            if (!ulong.TryParse(guidHex.AsSpan(0, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out guidHi))
            {
                return false;
            }

            return ulong.TryParse(guidHex.AsSpan(16, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out guidLo);
        }

        private static bool MatchesGuid(MonoBehaviour instance, ulong guidHi, ulong guidLo)
        {
            if (instance is null)
            {
                return false;
            }

            return ScriptGlueRegistry.TryGetRegisteredGuid(instance, out ulong instanceHi, out ulong instanceLo)
                && instanceHi == guidHi
                && instanceLo == guidLo;
        }
    }
}
