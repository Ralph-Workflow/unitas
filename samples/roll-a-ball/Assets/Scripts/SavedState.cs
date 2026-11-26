using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    public static class SavedState
    {
        private const string BestTimeKey = "records.bestTime";
        private const string BestScoreKey = "records.bestScore";
        private const string UnlockedArenasKey = "records.unlockedArenas";
        private const string QualityKey = "settings.quality";
        private const string MedalsKey = "records.medals";

        private const string GhostArena = "arena1";
        private const string BallObjectName = "PlayerBall";
        private const int GhostFileVersion = 1;
        private const int CurrentGhostVersion = 2;
        private const int RouteStepCount = 120;
        private const int ArenaScore = 900;
        private const float RouteStepX = 0.02f;
        private const float RouteStepZ = 0.01f;
        private const float RouteOriginY = 0.5f;
        private const float ReplayToleranceMetres = 0.01f;

        private static readonly IReadOnlyList<SaveMigrationStep> GhostMigrationSteps = new SaveMigrationStep[]
        {
            new SaveMigrationStep(1, 2, MigrateGhostV1ToV2),
        };

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        public static void RestoreAndSave()
        {
            if (string.IsNullOrEmpty(System.Environment.GetEnvironmentVariable("UNITAS_PERSISTENT_DATA_PATH")))
            {
                return;
            }

            float restoredBestTime = PlayerPrefs.GetFloat(BestTimeKey, 0f);
            int restoredBestScore = PlayerPrefs.GetInt(BestScoreKey, 0);
            string restoredArenas = PlayerPrefs.GetString(UnlockedArenasKey, string.Empty);
            string quality = PlayerPrefs.GetString(QualityKey, "high");
            int medals = PlayerPrefs.GetInt(MedalsKey, 0);
            Debug.Log($"Saved state restored quality {quality} medals {medals} bestTime {FormatSeconds(restoredBestTime)} bestScore {restoredBestScore} arenas {restoredArenas}");

            Vector3[] route = RecordRoute();
            string ghostPath = Path.Combine(Application.persistentDataPath, "ghosts", GhostArena + ".ghost");

            if (File.Exists(ghostPath))
            {
                ReplayGhost(ghostPath, route);
            }

            byte[] record = EncodeRoute(route);
            string envelope = BuildEnvelope(GhostFileVersion, route.Length, Convert.ToBase64String(record));
            SaveMigration.WriteSaveFileAtomically(ghostPath, new UTF8Encoding(false).GetBytes(envelope));
            Debug.Log($"Ghost written {ghostPath} steps {route.Length} bytes {record.Length}");

            float bestTime = route.Length * Time.fixedDeltaTime;
            int bestScore = ArenaScore;
            string arenas = MergeArena(restoredArenas, GhostArena);
            PlayerPrefs.SetFloat(BestTimeKey, bestTime);
            PlayerPrefs.SetInt(BestScoreKey, bestScore);
            PlayerPrefs.SetString(UnlockedArenasKey, arenas);
            PlayerPrefs.Save();
            Debug.Log($"Saved state saved quality {quality} medals {medals} bestTime {FormatSeconds(bestTime)} bestScore {bestScore} arenas {arenas}");
        }

        private static Vector3[] RecordRoute()
        {
            Vector3 origin = new Vector3(0f, RouteOriginY, 0f);
            GameObject? ball = GameObject.Find(BallObjectName);
            if (ball != null)
            {
                origin = ball.transform.position;
            }

            Vector3[] route = new Vector3[RouteStepCount];
            for (int step = 0; step < route.Length; step++)
            {
                route[step] = new Vector3(
                    origin.x + (step * RouteStepX),
                    origin.y,
                    origin.z + (step * RouteStepZ));
            }

            return route;
        }

        private static void ReplayGhost(string ghostPath, Vector3[] recorded)
        {
            SaveMigrationResult result = SaveMigration.MigrateFile(ghostPath, CurrentGhostVersion, GhostMigrationSteps);
            Debug.Log($"Ghost restored outcome {result.Outcome} from v{result.InputVersion} to v{result.ResultingVersion}");
            if (result.Outcome != SaveMigrationOutcome.Migrated && result.Outcome != SaveMigrationOutcome.AlreadyCurrent)
            {
                Debug.LogWarning($"Ghost replay failed outcome {result.Outcome} diagnostic {result.DiagnosticMessage}");
                return;
            }

            string routeBase64 = ExtractJsonString(result.Payload, "route");
            if (string.IsNullOrEmpty(routeBase64))
            {
                Debug.LogWarning("Ghost replay failed the migrated ghost carries no route");
                return;
            }

            Vector3[] restored = DecodeRoute(Convert.FromBase64String(routeBase64));
            if (restored == null)
            {
                Debug.LogWarning("Ghost replay failed the migrated ghost route is malformed");
                return;
            }

            int steps = Math.Min(restored.Length, recorded.Length);
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

        private static byte[] EncodeRoute(Vector3[] route)
        {
            byte[] bytes = new byte[route.Length * 6];
            int offset = 0;
            for (int step = 0; step < route.Length; step++)
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

        private static string MergeArena(string arenas, string arena)
        {
            string[] existing = arenas.Split(',');
            for (int index = 0; index < existing.Length; index++)
            {
                if (string.Equals(existing[index].Trim(), arena, StringComparison.Ordinal))
                {
                    return arenas;
                }
            }

            return string.IsNullOrEmpty(arenas) ? arena : arenas + "," + arena;
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
