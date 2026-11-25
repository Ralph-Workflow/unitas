using System;
using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    [Serializable]
    public sealed class JsonTestNestedStats
    {
        public int strength = 10;
        [SerializeField]
        private float _critChance = 0.25f;

        public int ComputedRating => strength * 10;

        public float GetCritChance() => _critChance;

        public void SetCritChance(float value) => _critChance = value;
    }

    [Serializable]
    public sealed class JsonTestPlayerProfile : ISerializationCallbackReceiver
    {
        public string playerName = string.Empty;
        public int level = 1;
        public float experience = 0f;
        public bool isVip;
        public Vector3 spawnPosition;
        [SerializeField]
        private int _secretToken = 777;
        [SerializeField]
        private string _privateLabel = "default_secret";
        private int _unmarkedPrivateField = 42;
        [NonSerialized]
        public int transientBonus = 999;
        public JsonTestNestedStats stats = new JsonTestNestedStats();
        public List<int> completedQuests = new List<int>();
        public string[] inventoryTags = Array.Empty<string>();

        public string ComputedTitle
        {
            get => playerName + "_Title";
            set => playerName = value;
        }

        public int BeforeSerializeCount { get; private set; }

        public int AfterDeserializeCount { get; private set; }

        public int GetSecretToken() => _secretToken;

        public void SetSecretToken(int token) => _secretToken = token;

        public string GetPrivateLabel() => _privateLabel;

        public void SetPrivateLabel(string label) => _privateLabel = label;

        public int GetUnmarkedPrivateField() => _unmarkedPrivateField;

        public void SetUnmarkedPrivateField(int val) => _unmarkedPrivateField = val;

        public void OnBeforeSerialize()
        {
            BeforeSerializeCount++;
        }

        public void OnAfterDeserialize()
        {
            AfterDeserializeCount++;
        }
    }

    public sealed class JsonTestUnregisteredPayload
    {
        public int id = 5;
        public string label = "unregistered";
    }

    public static class JsonUtilityTests
    {
        private static bool _typesRegistered;

        public static void Run()
        {
            EnsureTestTypesRegistered();
            TestRoundTripPublicAndPrivateSerializeFieldWhileSkippingPropertiesAndNonSerialized();
            TestFromJsonOverwriteUpdatesExistingInstanceInPlace();
            TestCultureInvariantFloatFormattingAndPrettyPrint();
            TestMalformedJsonRaisesExceptionWithParsePosition();
            TestUnregisteredTypeLogsDiagnosticErrorAndThrows();
            TestSourceGeneratorEmitsGlueForPrivateSerializeFieldAndSkipsProperties();
        }

        public static void RunAll() => Run();

        private static void EnsureTestTypesRegistered()
        {
            if (_typesRegistered)
            {
                return;
            }

            _typesRegistered = true;

            JsonUtility.RegisterType<JsonTestNestedStats>(
                () => new JsonTestNestedStats(),
                (val, w) =>
                {
                    w.WriteInt("strength", val.strength);
                    w.WriteFloat("_critChance", val.GetCritChance());
                },
                (ref JsonTestNestedStats target, JsonParseNode n) =>
                {
                    target.strength = n.ReadInt("strength", target.strength);
                    target.SetCritChance(n.ReadFloat("_critChance", target.GetCritChance()));
                });

            JsonUtility.RegisterType<JsonTestPlayerProfile>(
                () => new JsonTestPlayerProfile(),
                (val, w) =>
                {
                    w.WriteString("playerName", val.playerName);
                    w.WriteInt("level", val.level);
                    w.WriteFloat("experience", val.experience);
                    w.WriteBool("isVip", val.isVip);
                    w.WriteVector3("spawnPosition", val.spawnPosition);
                    w.WriteInt("_secretToken", val.GetSecretToken());
                    w.WriteString("_privateLabel", val.GetPrivateLabel());
                    w.WriteNested("stats", val.stats);
                    w.WriteIntList("completedQuests", val.completedQuests);
                    w.WriteStringList("inventoryTags", val.inventoryTags);
                },
                (ref JsonTestPlayerProfile target, JsonParseNode n) =>
                {
                    target.playerName = n.ReadString("playerName", target.playerName);
                    target.level = n.ReadInt("level", target.level);
                    target.experience = n.ReadFloat("experience", target.experience);
                    target.isVip = n.ReadBool("isVip", target.isVip);
                    target.spawnPosition = n.ReadVector3("spawnPosition", target.spawnPosition);
                    target.SetSecretToken(n.ReadInt("_secretToken", target.GetSecretToken()));
                    target.SetPrivateLabel(n.ReadString("_privateLabel", target.GetPrivateLabel()));
                    target.stats = n.ReadNested("stats", target.stats);
                    target.completedQuests = n.ReadIntList("completedQuests", target.completedQuests);
                    target.inventoryTags = n.ReadStringArray("inventoryTags", target.inventoryTags);
                });
        }

        private static void TestRoundTripPublicAndPrivateSerializeFieldWhileSkippingPropertiesAndNonSerialized()
        {
            JsonTestPlayerProfile original = new JsonTestPlayerProfile
            {
                playerName = "Kael",
                level = 14,
                experience = 1234.5f,
                isVip = true,
                spawnPosition = new Vector3(10.5f, -2.0f, 33.25f),
                transientBonus = 5555
            };
            original.SetSecretToken(9001);
            original.SetPrivateLabel("hidden_vault_key");
            original.SetUnmarkedPrivateField(888);
            original.stats.strength = 45;
            original.stats.SetCritChance(0.75f);
            original.completedQuests.Add(101);
            original.completedQuests.Add(205);
            original.inventoryTags = new[] { "sword", "shield" };

            string json = JsonUtility.ToJson(original);
            Check.That("ToJson output is non-empty", !string.IsNullOrEmpty(json));
            Check.Equal("OnBeforeSerialize callback ran once", 1, original.BeforeSerializeCount);
            Check.That("ToJson includes public field playerName", json.Contains("\"playerName\":\"Kael\"", StringComparison.Ordinal));
            Check.That("ToJson includes private [SerializeField] _secretToken", json.Contains("\"_secretToken\":9001", StringComparison.Ordinal));
            Check.That("ToJson includes private [SerializeField] _privateLabel", json.Contains("\"_privateLabel\":\"hidden_vault_key\"", StringComparison.Ordinal));
            Check.That("ToJson skips property ComputedTitle", !json.Contains("ComputedTitle", StringComparison.Ordinal));
            Check.That("ToJson skips nested property ComputedRating", !json.Contains("ComputedRating", StringComparison.Ordinal));
            Check.That("ToJson skips [NonSerialized] field transientBonus", !json.Contains("transientBonus", StringComparison.Ordinal));
            Check.That("ToJson skips unmarked private field _unmarkedPrivateField", !json.Contains("_unmarkedPrivateField", StringComparison.Ordinal));

            JsonTestPlayerProfile? restored = JsonUtility.FromJson<JsonTestPlayerProfile>(json);
            Check.That("FromJson returns non-null instance", restored != null);
            if (restored != null)
            {
                Check.Equal("OnAfterDeserialize callback ran once", 1, restored.AfterDeserializeCount);
                Check.Equal("Round-trip public string playerName", "Kael", restored.playerName);
                Check.Equal("Round-trip public int level", 14, restored.level);
                Check.Equal("Round-trip public float experience", 1234.5f, restored.experience);
                Check.Equal("Round-trip public bool isVip", true, restored.isVip);
                Check.Equal("Round-trip Vector3 spawnPosition.x", 10.5f, restored.spawnPosition.x);
                Check.Equal("Round-trip Vector3 spawnPosition.y", -2.0f, restored.spawnPosition.y);
                Check.Equal("Round-trip Vector3 spawnPosition.z", 33.25f, restored.spawnPosition.z);
                Check.Equal("Round-trip private [SerializeField] _secretToken", 9001, restored.GetSecretToken());
                Check.Equal("Round-trip private [SerializeField] _privateLabel", "hidden_vault_key", restored.GetPrivateLabel());
                Check.Equal("Unmarked private field retains default value", 42, restored.GetUnmarkedPrivateField());
                Check.Equal("[NonSerialized] field retains default value", 999, restored.transientBonus);
                Check.Equal("Nested [Serializable] public field strength", 45, restored.stats.strength);
                Check.Equal("Nested [Serializable] private [SerializeField] _critChance", 0.75f, restored.stats.GetCritChance());
                Check.Equal("Round-trip List<int> count", 2, restored.completedQuests.Count);
                Check.Equal("Round-trip List<int>[0]", 101, restored.completedQuests[0]);
                Check.Equal("Round-trip List<int>[1]", 205, restored.completedQuests[1]);
                Check.Equal("Round-trip string[] Length", 2, restored.inventoryTags.Length);
                Check.Equal("Round-trip string[][0]", "sword", restored.inventoryTags[0]);
                Check.Equal("Round-trip string[][1]", "shield", restored.inventoryTags[1]);
            }
        }

        private static void TestFromJsonOverwriteUpdatesExistingInstanceInPlace()
        {
            JsonTestPlayerProfile target = new JsonTestPlayerProfile
            {
                playerName = "Initial",
                level = 3,
                transientBonus = 1234
            };
            target.SetSecretToken(11);
            target.SetUnmarkedPrivateField(77);

            JsonTestPlayerProfile originalRef = target;
            JsonUtility.FromJsonOverwrite("{\"level\":50,\"_secretToken\":4444,\"ComputedTitle\":\"IgnoredProperty\"}", ref target);

            Check.That("FromJsonOverwrite preserves object identity", ReferenceEquals(originalRef, target));
            Check.Equal("FromJsonOverwrite keeps untouched field playerName", "Initial", target.playerName);
            Check.Equal("FromJsonOverwrite updates level", 50, target.level);
            Check.Equal("FromJsonOverwrite updates private [SerializeField] _secretToken", 4444, target.GetSecretToken());
            Check.Equal("FromJsonOverwrite leaves [NonSerialized] field untouched", 1234, target.transientBonus);
            Check.Equal("FromJsonOverwrite leaves unmarked private field untouched", 77, target.GetUnmarkedPrivateField());
        }

        private static void TestCultureInvariantFloatFormattingAndPrettyPrint()
        {
            CultureInfo savedCulture = CultureInfo.CurrentCulture;
            try
            {
                CultureInfo.CurrentCulture = new CultureInfo("de-DE");
                Vector3 v = new Vector3(1.5f, -2.25f, 0.125f);
                string compact = JsonUtility.ToJson(v, false);
                Check.That("Culture-invariant float uses decimal point under de-DE culture", compact.Contains("1.5", StringComparison.Ordinal) && !compact.Contains("1,5", StringComparison.Ordinal));

                string pretty = JsonUtility.ToJson(v, true);
                Check.That("PrettyPrint output contains newlines and indentation", pretty.Contains("\n    \"x\": ", StringComparison.Ordinal));

                Vector3 roundTripped = JsonUtility.FromJson<Vector3>(pretty);
                Check.Equal("Vector3 round-trip x under de-DE", 1.5f, roundTripped.x);
                Check.Equal("Vector3 round-trip y under de-DE", -2.25f, roundTripped.y);
                Check.Equal("Vector3 round-trip z under de-DE", 0.125f, roundTripped.z);
            }
            finally
            {
                CultureInfo.CurrentCulture = savedCulture;
            }
        }

        private static void TestMalformedJsonRaisesExceptionWithParsePosition()
        {
            bool threwWithPosition = false;
            try
            {
                JsonUtility.FromJson<JsonTestPlayerProfile>("{\"playerName\":\"Kael\", \"level\": }");
            }
            catch (ArgumentException ex)
            {
                threwWithPosition = ex.Message.Contains("position", StringComparison.OrdinalIgnoreCase);
            }

            Check.Equal("Malformed JSON throws ArgumentException naming parse position", true, threwWithPosition);
        }

        private static void TestUnregisteredTypeLogsDiagnosticErrorAndThrows()
        {
            LogRecorder.Clear();
            bool threwOnToJson = false;
            try
            {
                JsonUtility.ToJson(new JsonTestUnregisteredPayload());
            }
            catch (InvalidOperationException ex)
            {
                threwOnToJson = ex.Message.Contains("JsonTestUnregisteredPayload", StringComparison.Ordinal);
            }

            Check.Equal("ToJson on unregistered type throws InvalidOperationException naming type", true, threwOnToJson);
            IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
            Check.Equal("ToJson on unregistered type logs one error diagnostic", 1, errors.Count);
            Check.That(
                "Error diagnostic names unregistered type JsonTestUnregisteredPayload",
                errors.Count == 1 && errors[0].Message.Contains("JsonTestUnregisteredPayload", StringComparison.Ordinal));

            LogRecorder.Clear();
            bool threwOnFromJson = false;
            try
            {
                JsonUtility.FromJson<JsonTestUnregisteredPayload>("{\"id\":10}");
            }
            catch (InvalidOperationException ex)
            {
                threwOnFromJson = ex.Message.Contains("JsonTestUnregisteredPayload", StringComparison.Ordinal);
            }

            Check.Equal("FromJson on unregistered type throws InvalidOperationException naming type", true, threwOnFromJson);
            Check.Equal("FromJson on unregistered type logs one error diagnostic", 1, LogRecorder.OfSeverity(LogSeverity.Error).Count);
        }

        private static void TestSourceGeneratorEmitsGlueForPrivateSerializeFieldAndSkipsProperties()
        {
            string tempDir = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                "unitas_json_glue_gen_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            System.IO.Directory.CreateDirectory(tempDir);
            try
            {
                string scriptPath = System.IO.Path.Combine(tempDir, "SavePayload.cs");
                System.IO.File.WriteAllText(
                    scriptPath,
                    "using System;\n"
                    + "using UnityEngine;\n"
                    + "namespace SampleGame\n"
                    + "{\n"
                    + "    [Serializable]\n"
                    + "    public class SavePayload : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int publicCoins;\n"
                    + "        [SerializeField] private string _secretKey = \"abc\";\n"
                    + "        private int _plainPrivate = 4;\n"
                    + "        [NonSerialized] public int skippedPublic = 9;\n"
                    + "        public string IgnoredProperty { get; set; }\n"
                    + "    }\n"
                    + "}\n");
                System.IO.File.WriteAllText(
                    scriptPath + ".meta",
                    "fileFormatVersion: 2\nguid: 40000000000000000000000000000077\n");

                HashSet<string> defines = new HashSet<string>(StringComparer.Ordinal);
                Dictionary<string, string> pathToGuid = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    [scriptPath] = "40000000000000000000000000000077"
                };

                List<Unitas.Scripting.ScriptGuidBinding> bindings =
                    Unitas.Scripting.ScriptAnalyzerAndGlueGenerator.DiscoverScriptBindingsForAssembly(
                        "Assembly-CSharp",
                        new[] { scriptPath },
                        defines,
                        pathToGuid);
                List<Unitas.Scripting.SerializableTypeBinding> serializableTypes =
                    Unitas.Scripting.ScriptAnalyzerAndGlueGenerator.DiscoverSerializableTypesForAssembly(
                        "Assembly-CSharp",
                        new[] { scriptPath },
                        defines);

                Check.Equal("Generator discovers 1 MonoBehaviour binding", 1, bindings.Count);
                Check.Equal("Generator discovers 2 serializer-visible fields (publicCoins and _secretKey)", 2, bindings[0].SerializedFields.Count);

                string glueSource = Unitas.Scripting.ScriptAnalyzerAndGlueGenerator.GenerateGlueSource(bindings, serializableTypes);
                Check.That("Generated glue registers SavePayload with JsonUtility", glueSource.Contains("JsonUtility.RegisterType<global::SampleGame.SavePayload>", StringComparison.Ordinal));
                Check.That("Generated glue writes publicCoins", glueSource.Contains("writer.WriteInt(\"publicCoins\", value.publicCoins);", StringComparison.Ordinal));
                Check.That("Generated glue emits UnsafeAccessor for private [SerializeField] _secretKey", glueSource.Contains("[UnsafeAccessor(UnsafeAccessorKind.Field, Name = \"_secretKey\")]", StringComparison.Ordinal));
                Check.That("Generated glue writes private [SerializeField] _secretKey", glueSource.Contains("writer.WriteString(\"_secretKey\", FieldRef__secretKey(value));", StringComparison.Ordinal));
                Check.That("Generated glue skips unmarked private field _plainPrivate", !glueSource.Contains("_plainPrivate", StringComparison.Ordinal));
                Check.That("Generated glue skips [NonSerialized] field skippedPublic", !glueSource.Contains("skippedPublic", StringComparison.Ordinal));
                Check.That("Generated glue skips property IgnoredProperty", !glueSource.Contains("IgnoredProperty", StringComparison.Ordinal));
            }
            finally
            {
                if (System.IO.Directory.Exists(tempDir))
                {
                    System.IO.Directory.Delete(tempDir, true);
                }
            }
        }
    }
}
