using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    /// <summary>
    /// Owns the arena's ghost file: the record of the ball's real per-fixed-step
    /// trajectory, written under the persistent data path through the engine's
    /// atomic save path.
    /// </summary>
    public static class SavedState
    {
        private const string GhostArena = "arena1";
        private const string GhostFolder = "ghosts";
        private const int GhostFileVersion = 2;
        private const int CurrentGhostVersion = 2;
        private const float ReplayToleranceMetres = 0.01f;

        private static readonly IReadOnlyList<SaveMigrationStep> GhostMigrationSteps = new SaveMigrationStep[]
        {
            new SaveMigrationStep(1, 2, MigrateGhostV1ToV2),
        };

        /// <summary>The path of the arena's ghost file under the persistent data path.</summary>
        public static string GhostPath
        {
            get { return Path.Combine(Application.persistentDataPath, GhostFolder, GhostArena + ".ghost"); }
        }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        public static void RestoreAndSave()
        {
            if (!RecordsStore.IsArmed)
            {
                return;
            }

            RecordsStore.Restore();
            RestoreGhost(null);
        }

        /// <summary>
        /// Ends a finished run: it awards the run's records and writes the
        /// trajectory the run recorded as the arena's ghost. The run's update
        /// path calls this once the arena is cleared.
        /// </summary>
        public static void FinishRun(IReadOnlyList<Vector3> route, float seconds, int score)
        {
            if (!RecordsStore.IsArmed)
            {
                return;
            }

            if (route.Count == 0)
            {
                Debug.LogWarning("Ghost not written the run recorded no trajectory");
                return;
            }

            if (!RecordsStore.SaveRun(seconds, score) && File.Exists(GhostPath))
            {
                Debug.Log($"Ghost not written the run did not beat the recorded best {FormatSeconds(RecordsStore.BestTime)} s");
                return;
            }

            WriteGhost(route);
        }

        private static void WriteGhost(IReadOnlyList<Vector3> route)
        {
            byte[] record = EncodeRoute(route);
            string envelope = BuildEnvelope(GhostFileVersion, route.Count, Convert.ToBase64String(record));
            byte[] bytes = new UTF8Encoding(false).GetBytes(envelope);
            SaveMigration.WriteSaveFileAtomically(GhostPath, bytes);
            Vector3[] written = DecodeRoute(record);
            if (written == null)
            {
                Debug.LogWarning("Ghost written the encoded route is malformed");
                return;
            }

            Debug.Log($"Ghost written {Describe(GhostFileVersion, written, bytes.Length)} at {GhostPath}");
            RestoreGhost(route);
        }

        /// <summary>
        /// Reads the route the arena's ghost file holds. The file is migrated to
        /// the current version first, so a ghost an older build wrote still
        /// replays.
        /// </summary>
        public static Vector3[]? LoadGhostRoute()
        {
            return ReadPersistedRoute();
        }

        /// <summary>
        /// Reads the persisted ghost back and reports what the file holds. When
        /// a recorded route is given, the restored route is compared with it.
        /// </summary>
        private static void RestoreGhost(IReadOnlyList<Vector3>? recorded)
        {
            Vector3[]? restored = ReadPersistedRoute();
            if (restored == null || recorded == null)
            {
                return;
            }

            int steps = Math.Min(restored.Length, recorded.Count);
            float maxDeviation = 0f;
            for (int step = 0; step < steps; step++)
            {
                float dx = restored[step].x - recorded[step].x;
                float dy = restored[step].y - recorded[step].y;
                float dz = restored[step].z - recorded[step].z;
                float deviation = (float)Math.Sqrt((dx * dx) + (dy * dy) + (dz * dz));
                if (deviation > maxDeviation)
                {
                    maxDeviation = deviation;
                }
            }

            Debug.Log($"Ghost replay max deviation {FormatSeconds(maxDeviation)} m over {steps} steps");
            if (maxDeviation <= ReplayToleranceMetres)
            {
                Debug.Log("Ghost replay within 0.01 m");
            }
        }

        private static Vector3[]? ReadPersistedRoute()
        {
            string ghostPath = GhostPath;
            if (!File.Exists(ghostPath))
            {
                Debug.Log($"Ghost restored outcome {SaveMigrationOutcome.Absent} at {ghostPath}");
                return null;
            }

            SaveMigrationResult result = SaveMigration.MigrateFile(ghostPath, CurrentGhostVersion, GhostMigrationSteps);
            if (result.Outcome != SaveMigrationOutcome.Migrated && result.Outcome != SaveMigrationOutcome.AlreadyCurrent)
            {
                Debug.LogWarning($"Ghost replay failed outcome {result.Outcome} diagnostic {result.DiagnosticMessage}");
                return null;
            }

            string routeBase64 = ExtractJsonString(result.Payload, "route");
            if (string.IsNullOrEmpty(routeBase64))
            {
                Debug.LogWarning("Ghost replay failed the migrated ghost carries no route");
                return null;
            }

            Vector3[] restored = DecodeRoute(Convert.FromBase64String(routeBase64));
            if (restored == null)
            {
                Debug.LogWarning("Ghost replay failed the migrated ghost route is malformed");
                return null;
            }

            long size = new FileInfo(ghostPath).Length;
            Debug.Log($"Ghost restored {Describe(ExtractJsonInt(result.Payload, "version"), restored, size)} at {ghostPath} outcome {result.Outcome}");
            return restored;
        }

        private static bool MigrateGhostV1ToV2(
            string inputPayload,
            out string migratedPayload,
            IList<string> unretainedDataNotes,
            out string failureReason)
        {
            migratedPayload = string.Empty;
            failureReason = string.Empty;

            string route = ExtractJsonString(inputPayload, "route");
            int steps = ExtractJsonInt(inputPayload, "steps");
            if (string.IsNullOrEmpty(route) || steps <= 0)
            {
                failureReason = "the v1 ghost envelope carries no route";
                return false;
            }

            migratedPayload = BuildEnvelope(CurrentGhostVersion, steps, route);
            return true;
        }

        private static string BuildEnvelope(int version, int steps, string routeBase64)
        {
            StringBuilder builder = new StringBuilder();
            builder.Append("{\"version\":");
            builder.Append(version.ToString(CultureInfo.InvariantCulture));
            builder.Append(",\"steps\":");
            builder.Append(steps.ToString(CultureInfo.InvariantCulture));
            builder.Append(",\"route\":\"");
            builder.Append(routeBase64);
            builder.Append("\"}");
            return builder.ToString();
        }

        private static string Describe(int version, Vector3[] route, long size)
        {
            StringBuilder builder = new StringBuilder();
            builder.Append("version ");
            builder.Append(version.ToString(CultureInfo.InvariantCulture));
            builder.Append(" steps ");
            builder.Append(route.Length.ToString(CultureInfo.InvariantCulture));
            builder.Append(" bytes ");
            builder.Append(size.ToString(CultureInfo.InvariantCulture));
            builder.Append(" first ");
            builder.Append(FormatPoint(route[0]));
            builder.Append(" last ");
            builder.Append(FormatPoint(route[route.Length - 1]));
            return builder.ToString();
        }

        private static string FormatPoint(Vector3 point)
        {
            return string.Concat(
                point.x.ToString("F3", CultureInfo.InvariantCulture),
                " ",
                point.y.ToString("F3", CultureInfo.InvariantCulture),
                " ",
                point.z.ToString("F3", CultureInfo.InvariantCulture));
        }

        private static byte[] EncodeRoute(IReadOnlyList<Vector3> route)
        {
            byte[] bytes = new byte[route.Count * 6];
            int offset = 0;
            for (int step = 0; step < route.Count; step++)
            {
                offset = WriteInt16(bytes, offset, route[step].x);
                offset = WriteInt16(bytes, offset, route[step].y);
                offset = WriteInt16(bytes, offset, route[step].z);
            }

            return bytes;
        }

        private static Vector3[]? DecodeRoute(byte[] record)
        {
            if (record.Length % 6 != 0)
            {
                return null;
            }

            Vector3[] route = new Vector3[record.Length / 6];
            int offset = 0;
            for (int step = 0; step < route.Length; step++)
            {
                float x = ReadInt16(record, offset);
                float y = ReadInt16(record, offset + 2);
                float z = ReadInt16(record, offset + 4);
                offset += 6;
                route[step] = new Vector3(x / 1000f, y / 1000f, z / 1000f);
            }

            return route;
        }

        private static int WriteInt16(byte[] bytes, int offset, float metres)
        {
            int millimetres = (int)Math.Round(metres * 1000f, MidpointRounding.AwayFromZero);
            if (millimetres > short.MaxValue)
            {
                millimetres = short.MaxValue;
            }
            else if (millimetres < short.MinValue)
            {
                millimetres = short.MinValue;
            }

            bytes[offset] = (byte)(millimetres & 0xFF);
            bytes[offset + 1] = (byte)((millimetres >> 8) & 0xFF);
            return offset + 2;
        }

        private static float ReadInt16(byte[] bytes, int offset)
        {
            int value = bytes[offset] | (bytes[offset + 1] << 8);
            if (value > short.MaxValue)
            {
                value -= 0x10000;
            }

            return value;
        }

        private static string ExtractJsonString(string json, string key)
        {
            string quoted = "\"" + key + "\"";
            int keyPosition = json.IndexOf(quoted, StringComparison.Ordinal);
            if (keyPosition < 0)
            {
                return string.Empty;
            }

            int cursor = keyPosition + quoted.Length;
            while (cursor < json.Length && json[cursor] != ':')
            {
                cursor++;
            }

            while (cursor < json.Length && json[cursor] != '"')
            {
                cursor++;
            }

            if (cursor >= json.Length)
            {
                return string.Empty;
            }

            cursor++;
            int end = json.IndexOf('"', cursor);
            return end < 0 ? string.Empty : json.Substring(cursor, end - cursor);
        }

        private static int ExtractJsonInt(string json, string key)
        {
            string quoted = "\"" + key + "\"";
            int keyPosition = json.IndexOf(quoted, StringComparison.Ordinal);
            if (keyPosition < 0)
            {
                return 0;
            }

            int cursor = keyPosition + quoted.Length;
            while (cursor < json.Length && (json[cursor] == ':' || json[cursor] == ' '))
            {
                cursor++;
            }

            int start = cursor;
            while (cursor < json.Length && (json[cursor] == '-' || (json[cursor] >= '0' && json[cursor] <= '9')))
            {
                cursor++;
            }

            return int.TryParse(json.Substring(start, cursor - start), NumberStyles.Integer, CultureInfo.InvariantCulture, out int value) ? value : 0;
        }

        private static string FormatSeconds(float seconds)
        {
            return seconds.ToString("F3", CultureInfo.InvariantCulture);
        }
    }
}
