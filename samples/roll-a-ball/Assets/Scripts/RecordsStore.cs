using System;
using System.Globalization;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    /// <summary>
    /// Owns the run's records: the arena's medal, its best time and score and
    /// the arenas the player has unlocked. They live in PlayerPrefs, which is
    /// where the sample's save data belongs.
    /// </summary>
    public static class RecordsStore
    {
        private const string Arena = "arena1";
        private const string NextArena = "arena2";
        private const string MedalKey = "records.medals." + Arena;
        private const string BestTimeKey = "records.bestTime";
        private const string BestScoreKey = "records.bestScore";
        private const string UnlockedArenasKey = "records.unlockedArenas";
        private const string QualityKey = "settings.quality";
        private const string Quality = "high";
        private const float GoldMedalSeconds = 30f;
        private const float SilverMedalSeconds = 40f;
        private const float BronzeMedalSeconds = 60f;

        /// <summary>
        /// The persistence lane arms a data path. Without one the sample keeps
        /// no records, so an unarmed run leaves nothing behind on disk.
        /// </summary>
        public static bool IsArmed
        {
            get { return !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("UNITAS_PERSISTENT_DATA_PATH")); }
        }

        /// <summary>The arena's best run time in seconds, or zero when there is none.</summary>
        public static float BestTime
        {
            get { return PlayerPrefs.GetFloat(BestTimeKey, 0f); }
        }

        /// <summary>Reports the records the store holds after a launch.</summary>
        public static void Restore()
        {
            Debug.Log($"Records loaded {Describe()} bytes {PlayerPrefs.CurrentCommittedByteSize}");
        }

        /// <summary>
        /// Awards a finished run: it keeps the arena's best time and score,
        /// earns the medal the run's duration earns and unlocks the next arena.
        /// </summary>
        /// <returns>True when the run is the arena's best run so far.</returns>
        public static bool SaveRun(float seconds, int score)
        {
            float previousBest = BestTime;
            bool isBest = previousBest <= 0f || seconds <= previousBest;
            string earned = MedalFor(seconds);
            if (MedalRank(earned) > MedalRank(PlayerPrefs.GetString(MedalKey, string.Empty)))
            {
                PlayerPrefs.SetString(MedalKey, earned);
            }

            if (isBest)
            {
                PlayerPrefs.SetFloat(BestTimeKey, seconds);
            }

            if (score > PlayerPrefs.GetInt(BestScoreKey, 0))
            {
                PlayerPrefs.SetInt(BestScoreKey, score);
            }

            string arenas = MergeArena(PlayerPrefs.GetString(UnlockedArenasKey, string.Empty), Arena);
            arenas = MergeArena(arenas, NextArena);
            PlayerPrefs.SetString(UnlockedArenasKey, arenas);
            PlayerPrefs.SetString(QualityKey, Quality);
            PlayerPrefs.Save();
            Debug.Log($"Records saved {Describe()} bytes {PlayerPrefs.CurrentCommittedByteSize}");
            return isBest;
        }

        private static string Describe()
        {
            string medal = PlayerPrefs.GetString(MedalKey, string.Empty);
            if (string.IsNullOrEmpty(medal))
            {
                medal = "none";
            }

            string arenas = PlayerPrefs.GetString(UnlockedArenasKey, string.Empty);
            if (string.IsNullOrEmpty(arenas))
            {
                arenas = "none";
            }

            return string.Concat(
                "medals ",
                Arena,
                "=",
                medal,
                " bestTime ",
                FormatSeconds(PlayerPrefs.GetFloat(BestTimeKey, 0f)),
                " bestScore ",
                PlayerPrefs.GetInt(BestScoreKey, 0).ToString(CultureInfo.InvariantCulture),
                " arenas ",
                arenas,
                " quality ",
                PlayerPrefs.GetString(QualityKey, Quality));
        }

        private static string MedalFor(float seconds)
        {
            if (seconds <= GoldMedalSeconds)
            {
                return "gold";
            }

            if (seconds <= SilverMedalSeconds)
            {
                return "silver";
            }

            if (seconds <= BronzeMedalSeconds)
            {
                return "bronze";
            }

            return "none";
        }

        private static int MedalRank(string medal)
        {
            if (string.Equals(medal, "gold", StringComparison.Ordinal))
            {
                return 3;
            }

            if (string.Equals(medal, "silver", StringComparison.Ordinal))
            {
                return 2;
            }

            if (string.Equals(medal, "bronze", StringComparison.Ordinal))
            {
                return 1;
            }

            return 0;
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

        private static string FormatSeconds(float seconds)
        {
            return seconds.ToString("F3", CultureInfo.InvariantCulture);
        }
    }
}
