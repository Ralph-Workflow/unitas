using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Unitas.Scripting;

namespace Unitas.Bridge.Tests
{
    public static class CodeLoopTests
    {
        public static void Run()
        {
            RunAll();
        }

        public static void RunAll()
        {
            TestDiagnoseCleanBreakingFixingAndLastValidLoop();
            TestSymbolQueryTypesMembersSignaturesAndSerializedFields();
            TestIdeCsprojDiagnosticParityWithDiagnose();
            TestCliDiagnoseAndSymbolsSubcommands();
        }

        private static void TestDiagnoseCleanBreakingFixingAndLastValidLoop()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "codeloop_diagnose");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                string gameplayScriptPath = Path.Combine(tempProject, "Assets", "Gameplay", "GameplayController.cs");
                string originalGameplaySource = File.ReadAllText(gameplayScriptPath);

                File.WriteAllText(
                    gameplayScriptPath,
                    "using UnityEngine;\nnamespace MyGame.Gameplay\n{\n    public class GameplayController : MonoBehaviour\n    {\n        public void BadBeforeFirstValid() { unresolvable_first_compile_symbol(); }\n    }\n}\n");

                DiagnoseReport noPriorValidReport = ScriptCompilation.DiagnoseProject(tempProject, options);
                Check.Equal("CodeLoop diagnose: initial broken compile fails", false, noPriorValidReport.Succeeded);
                Check.Equal("CodeLoop diagnose: runningCompilation is 'none' when no prior valid compilation exists", "none", noPriorValidReport.RunningCompilation);
                Check.That("CodeLoop diagnose: GetLastValidCompilation is null before any clean compile", ScriptCompilation.GetLastValidCompilation(tempProject) == null);

                File.WriteAllText(gameplayScriptPath, originalGameplaySource);

                DiagnoseReport cleanReport = ScriptCompilation.DiagnoseProject(tempProject, options);
                Check.Equal("CodeLoop diagnose: clean compile succeeds", true, cleanReport.Succeeded);
                Check.Equal("CodeLoop diagnose: clean compile has 0 errors", 0, cleanReport.ErrorCount);
                Check.Equal("CodeLoop diagnose: clean compile reports runningCompilation 'new'", "new", cleanReport.RunningCompilation);
                Check.That("CodeLoop diagnose: clean compile populates activeAssemblyPaths", cleanReport.ActiveAssemblyPaths.Count >= 6);

                string cleanJson = cleanReport.ToJson();
                using (JsonDocument cleanDoc = JsonDocument.Parse(cleanJson))
                {
                    JsonElement root = cleanDoc.RootElement;
                    Check.Equal("CodeLoop diagnose JSON: clean succeeded true", true, root.GetProperty("succeeded").GetBoolean());
                    Check.Equal("CodeLoop diagnose JSON: clean runningCompilation 'new'", "new", root.GetProperty("runningCompilation").GetString() ?? string.Empty);
                    Check.Equal("CodeLoop diagnose JSON: clean errorCount 0", 0, root.GetProperty("errorCount").GetInt32());
                }

                ScriptCompilationResult? lastValidAfterClean = ScriptCompilation.GetLastValidCompilation(tempProject);
                Check.That("CodeLoop diagnose: last valid compilation recorded after clean compile", lastValidAfterClean != null && lastValidAfterClean.Succeeded);

                string brokenSource =
                    "using MyGame.Core;\n"
                    + "using UnityEngine;\n"
                    + "\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    public class GameplayController : MonoBehaviour\n"
                    + "    {\n"
                    + "        public void TriggerUnknownIdentifier()\n"
                    + "        {\n"
                    + "            int value = missing_identifier_for_cs0103 + 1;\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n";
                File.WriteAllText(gameplayScriptPath, brokenSource);

                DiagnoseReport brokenReport = ScriptCompilation.DiagnoseProject(tempProject, options);
                Check.Equal("CodeLoop diagnose: breaking edit fails compilation", false, brokenReport.Succeeded);
                Check.Equal("CodeLoop diagnose: breaking edit reports runningCompilation 'lastValid'", "lastValid", brokenReport.RunningCompilation);
                Check.That("CodeLoop diagnose: breaking edit has errorCount >= 1", brokenReport.ErrorCount >= 1);
                Check.That("CodeLoop diagnose: breaking edit preserves activeAssemblyPaths from last valid compilation", brokenReport.ActiveAssemblyPaths.Count >= 6);

                DiagnoseEntry? cs0103 = FindDiagnosticById(brokenReport.Diagnostics, "CS0103");
                Check.That("CodeLoop diagnose: breaking edit emits CS0103 diagnostic", cs0103 != null);
                if (cs0103 != null)
                {
                    Check.Equal("CodeLoop diagnose: CS0103 severity is error", "error", cs0103.Severity);
                    Check.Equal("CodeLoop diagnose: CS0103 assembly is MyGame.Gameplay", "MyGame.Gameplay", cs0103.Assembly);
                    Check.Equal("CodeLoop diagnose: CS0103 file matches GameplayController.cs", NormalizeRealPath(gameplayScriptPath), NormalizeRealPath(cs0103.File));
                    Check.Equal("CodeLoop diagnose: CS0103 line is 10", 10, cs0103.Line);
                    Check.Equal("CodeLoop diagnose: CS0103 column is 25", 25, cs0103.Column);
                    Check.That("CodeLoop diagnose: CS0103 message names missing_identifier_for_cs0103", cs0103.Message.Contains("missing_identifier_for_cs0103", StringComparison.Ordinal));
                }

                string brokenJson = brokenReport.ToJson();
                using (JsonDocument brokenDoc = JsonDocument.Parse(brokenJson))
                {
                    JsonElement root = brokenDoc.RootElement;
                    Check.Equal("CodeLoop diagnose JSON: broken succeeded false", false, root.GetProperty("succeeded").GetBoolean());
                    Check.Equal("CodeLoop diagnose JSON: broken runningCompilation 'lastValid'", "lastValid", root.GetProperty("runningCompilation").GetString() ?? string.Empty);
                    JsonElement diags = root.GetProperty("diagnostics");
                    Check.That("CodeLoop diagnose JSON: diagnostics array non-empty", diags.GetArrayLength() >= 1);
                    JsonElement firstDiag = diags[0];
                    Check.Equal("CodeLoop diagnose JSON: diagnostic identifier CS0103", "CS0103", firstDiag.GetProperty("identifier").GetString() ?? string.Empty);
                    Check.Equal("CodeLoop diagnose JSON: diagnostic severity error", "error", firstDiag.GetProperty("severity").GetString() ?? string.Empty);
                    Check.Equal("CodeLoop diagnose JSON: diagnostic assembly MyGame.Gameplay", "MyGame.Gameplay", firstDiag.GetProperty("assembly").GetString() ?? string.Empty);
                    Check.Equal("CodeLoop diagnose JSON: diagnostic line 10", 10, firstDiag.GetProperty("line").GetInt32());
                    Check.Equal("CodeLoop diagnose JSON: diagnostic column 25", 25, firstDiag.GetProperty("column").GetInt32());
                }

                ScriptCompilationResult? lastValidDuringBroken = ScriptCompilation.GetLastValidCompilation(tempProject);
                Check.That("CodeLoop diagnose: last valid compilation remains inspectable during broken edit", lastValidDuringBroken != null && lastValidDuringBroken.Succeeded);
                if (lastValidDuringBroken != null)
                {
                    Check.That("CodeLoop diagnose: last valid compilation retains script bindings", lastValidDuringBroken.ScriptBindings.Count > 0);
                    Check.That("CodeLoop diagnose: last valid compilation DLLs still exist on disk", lastValidDuringBroken.ActiveAssemblyPaths.Count >= 6 && File.Exists(lastValidDuringBroken.ActiveAssemblyPaths[0]));
                }

                File.WriteAllText(gameplayScriptPath, originalGameplaySource);

                DiagnoseReport fixedReport = ScriptCompilation.DiagnoseProject(tempProject, options);
                Check.Equal("CodeLoop diagnose: fixing edit succeeds", true, fixedReport.Succeeded);
                Check.Equal("CodeLoop diagnose: fixing edit clears errors to 0", 0, fixedReport.ErrorCount);
                Check.Equal("CodeLoop diagnose: fixing edit updates runningCompilation to 'new'", "new", fixedReport.RunningCompilation);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestSymbolQueryTypesMembersSignaturesAndSerializedFields()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "codeloop_symbols");
            try
            {
                string customAttrsAndScriptsPath = Path.Combine(tempProject, "Assets", "Gameplay", "SymbolInspectionSample.cs");
                string sampleSource =
                    "using System;\n"
                    + "using System.Collections.Generic;\n"
                    + "using UnityEngine;\n"
                    + "using UnityEngine.Serialization;\n"
                    + "\n"
                    + "namespace UnityEngine\n"
                    + "{\n"
                    + "    [AttributeUsage(AttributeTargets.Field)]\n"
                    + "    public sealed class SerializeField : Attribute {}\n"
                    + "\n"
                    + "    [AttributeUsage(AttributeTargets.Field)]\n"
                    + "    public sealed class SerializeReference : Attribute {}\n"
                    + "\n"
                    + "    [AttributeUsage(AttributeTargets.Class)]\n"
                    + "    public sealed class RequireComponent : Attribute\n"
                    + "    {\n"
                    + "        public RequireComponent(Type requiredComponent) {}\n"
                    + "    }\n"
                    + "\n"
                    + "    public class ScriptableObject : Object {}\n"
                    + "}\n"
                    + "\n"
                    + "namespace UnityEngine.Serialization\n"
                    + "{\n"
                    + "    [AttributeUsage(AttributeTargets.Field, AllowMultiple = true)]\n"
                    + "    public sealed class FormerlySerializedAsAttribute : Attribute\n"
                    + "    {\n"
                    + "        public string oldName { get; }\n"
                    + "        public FormerlySerializedAsAttribute(string oldName) { this.oldName = oldName; }\n"
                    + "    }\n"
                    + "}\n"
                    + "\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    public interface IInteractable\n"
                    + "    {\n"
                    + "        string GetPrompt(int priority);\n"
                    + "    }\n"
                    + "\n"
                    + "    public interface IPolymorphicEffect\n"
                    + "    {\n"
                    + "        void Apply();\n"
                    + "    }\n"
                    + "\n"
                    + "    [Serializable]\n"
                    + "    public struct WeaponStats\n"
                    + "    {\n"
                    + "        public int Damage;\n"
                    + "        [SerializeField] private float _critMultiplier;\n"
                    + "        [NonSerialized] public int TempRuntimeCache;\n"
                    + "    }\n"
                    + "\n"
                    + "    public abstract class BaseActorBehaviour : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int BaseHealth = 100;\n"
                    + "        [SerializeField] private string _baseFaction = \"Neutral\";\n"
                    + "        private int _unserializedBaseSecret;\n"
                    + "    }\n"
                    + "\n"
                    + "    [RequireComponent(typeof(Rigidbody))]\n"
                    + "    public class HeroActor : BaseActorBehaviour, IInteractable\n"
                    + "    {\n"
                    + "        public float MoveSpeed = 5.5f;\n"
                    + "        [SerializeField] private int _privateMana = 50;\n"
                    + "        [NonSerialized] public int PublicNonSerializedScore = 999;\n"
                    + "        [SerializeReference] private IPolymorphicEffect _activeEffect;\n"
                    + "        [SerializeField, FormerlySerializedAs(\"legacy_player_title\")] private string _playerTitle = \"Knight\";\n"
                    + "        public WeaponStats PrimaryWeapon;\n"
                    + "        public List<string> InventoryTags;\n"
                    + "        private float _plainPrivateUnserialized;\n"
                    + "        public static int GlobalHeroCount;\n"
                    + "        public readonly int ReadOnlyId = 7;\n"
                    + "\n"
                    + "        public int CurrentMana => _privateMana;\n"
                    + "\n"
                    + "        public string GetPrompt(int priority)\n"
                    + "        {\n"
                    + "            return _playerTitle + \":\" + priority;\n"
                    + "        }\n"
                    + "\n"
                    + "        private void OnCustomEvent(string eventName, float weight)\n"
                    + "        {\n"
                    + "        }\n"
                    + "    }\n"
                    + "\n"
                    + "    public class InventoryConfigAsset : ScriptableObject\n"
                    + "    {\n"
                    + "        public int MaxSlots = 24;\n"
                    + "        [SerializeField, FormerlySerializedAs(\"old_weight_limit\")] private float _weightLimit = 100f;\n"
                    + "        [NonSerialized] public string TransientSessionToken = string.Empty;\n"
                    + "        [SerializeReference] private IPolymorphicEffect _defaultLoadoutEffect;\n"
                    + "    }\n"
                    + "}\n";
                File.WriteAllText(customAttrsAndScriptsPath, sampleSource);

                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                SymbolQueryResult allSymbols = SymbolQuery.QueryProject(tempProject, null, options);
                Check.That("CodeLoop symbols: discovers project types across assemblies", allSymbols.Types.Count >= 10);

                SymbolTypeRecord? coreCounter = allSymbols.FindType("MyGame.Core.CoreCounter");
                Check.That("CodeLoop symbols: finds MyGame.Core.CoreCounter", coreCounter != null);
                if (coreCounter != null)
                {
                    Check.Equal("CodeLoop symbols: CoreCounter assembly is MyGame.Core", "MyGame.Core", coreCounter.Assembly);
                    Check.Equal("CodeLoop symbols: CoreCounter baseType is UnityEngine.MonoBehaviour", "UnityEngine.MonoBehaviour", coreCounter.BaseType);
                    Check.Equal("CodeLoop symbols: CoreCounter isMonoBehaviour is true", true, coreCounter.IsMonoBehaviour);
                }

                SymbolQueryResult heroQuery = SymbolQuery.QueryProject(tempProject, "HeroActor", options);
                Check.Equal("CodeLoop symbols: filtered query for HeroActor returns 1 type", 1, heroQuery.Types.Count);
                SymbolTypeRecord? hero = heroQuery.FindType("MyGame.Gameplay.HeroActor");
                Check.That("CodeLoop symbols: HeroActor record resolved", hero != null);

                if (hero != null)
                {
                    Check.Equal("CodeLoop symbols: HeroActor assembly is MyGame.Gameplay", "MyGame.Gameplay", hero.Assembly);
                    Check.Equal("CodeLoop symbols: HeroActor baseType is MyGame.Gameplay.BaseActorBehaviour", "MyGame.Gameplay.BaseActorBehaviour", hero.BaseType);
                    Check.That("CodeLoop symbols: HeroActor baseTypeChain includes UnityEngine.MonoBehaviour", ContainsString(hero.BaseTypeChain, "UnityEngine.MonoBehaviour"));
                    Check.That("CodeLoop symbols: HeroActor interfaces includes MyGame.Gameplay.IInteractable", ContainsString(hero.Interfaces, "MyGame.Gameplay.IInteractable"));
                    Check.That("CodeLoop symbols: HeroActor attributes includes RequireComponent(typeof(UnityEngine.Rigidbody))", ContainsSubstring(hero.Attributes, "RequireComponent(typeof(UnityEngine.Rigidbody))"));
                    Check.Equal("CodeLoop symbols: HeroActor isMonoBehaviour is true", true, hero.IsMonoBehaviour);

                    SymbolMemberRecord? getPrompt = hero.FindMember("GetPrompt");
                    Check.That("CodeLoop symbols: HeroActor lists method GetPrompt", getPrompt != null);
                    if (getPrompt != null)
                    {
                        Check.Equal("CodeLoop symbols: GetPrompt kind is method", "method", getPrompt.Kind);
                        Check.Equal("CodeLoop symbols: GetPrompt signature", "public string GetPrompt(int priority)", getPrompt.Signature);
                    }

                    SymbolMemberRecord? onCustomEvent = hero.FindMember("OnCustomEvent");
                    Check.That("CodeLoop symbols: HeroActor lists private method OnCustomEvent", onCustomEvent != null);
                    if (onCustomEvent != null)
                    {
                        Check.Equal("CodeLoop symbols: OnCustomEvent signature", "private void OnCustomEvent(string eventName, float weight)", onCustomEvent.Signature);
                    }

                    SymbolMemberRecord? currentManaProp = hero.FindMember("CurrentMana");
                    Check.That("CodeLoop symbols: HeroActor lists property CurrentMana in members", currentManaProp != null && currentManaProp.Kind == "property");
                    Check.That("CodeLoop symbols: HeroActor does not serialize property CurrentMana", hero.FindSerializedField("CurrentMana") == null);

                    Check.That("CodeLoop symbols: HeroActor serializes inherited public BaseHealth", hero.FindSerializedField("BaseHealth") != null);
                    Check.That("CodeLoop symbols: HeroActor serializes inherited [SerializeField] _baseFaction", hero.FindSerializedField("_baseFaction") != null);
                    Check.That("CodeLoop symbols: HeroActor omits inherited unannotated private _unserializedBaseSecret", hero.FindSerializedField("_unserializedBaseSecret") == null);

                    SerializedFieldRecord? moveSpeed = hero.FindSerializedField("MoveSpeed");
                    Check.That("CodeLoop symbols: HeroActor serializes public MoveSpeed", moveSpeed != null && moveSpeed.FieldType == "float");

                    SerializedFieldRecord? privateMana = hero.FindSerializedField("_privateMana");
                    Check.That("CodeLoop symbols: HeroActor serializes [SerializeField] private _privateMana", privateMana != null && privateMana.HasSerializeField && privateMana.Accessibility == "private");

                    Check.That("CodeLoop symbols: HeroActor omits [NonSerialized] PublicNonSerializedScore", hero.FindSerializedField("PublicNonSerializedScore") == null);
                    Check.That("CodeLoop symbols: HeroActor omits unannotated private _plainPrivateUnserialized", hero.FindSerializedField("_plainPrivateUnserialized") == null);
                    Check.That("CodeLoop symbols: HeroActor omits static GlobalHeroCount", hero.FindSerializedField("GlobalHeroCount") == null);
                    Check.That("CodeLoop symbols: HeroActor omits readonly ReadOnlyId", hero.FindSerializedField("ReadOnlyId") == null);

                    SerializedFieldRecord? activeEffect = hero.FindSerializedField("_activeEffect");
                    Check.That("CodeLoop symbols: HeroActor serializes [SerializeReference] _activeEffect", activeEffect != null && activeEffect.IsSerializeReference);
                    if (activeEffect != null)
                    {
                        Check.Equal("CodeLoop symbols: _activeEffect type is MyGame.Gameplay.IPolymorphicEffect", "MyGame.Gameplay.IPolymorphicEffect", activeEffect.FieldType);
                    }

                    SerializedFieldRecord? playerTitle = hero.FindSerializedField("_playerTitle");
                    Check.That("CodeLoop symbols: HeroActor serializes _playerTitle with FormerlySerializedAs", playerTitle != null);
                    if (playerTitle != null)
                    {
                        Check.Equal("CodeLoop symbols: _playerTitle has 1 former name", 1, playerTitle.FormerlySerializedAs.Count);
                        Check.Equal("CodeLoop symbols: _playerTitle former name is legacy_player_title", "legacy_player_title", playerTitle.FormerlySerializedAs[0]);
                    }

                    Check.That("CodeLoop symbols: HeroActor serializes [Serializable] struct PrimaryWeapon", hero.FindSerializedField("PrimaryWeapon") != null);
                    Check.That("CodeLoop symbols: HeroActor serializes List<string> InventoryTags", hero.FindSerializedField("InventoryTags") != null);
                }

                SymbolQueryResult soQuery = SymbolQuery.QueryProject(tempProject, "InventoryConfigAsset", options);
                SymbolTypeRecord? soType = soQuery.FindType("MyGame.Gameplay.InventoryConfigAsset");
                Check.That("CodeLoop symbols: finds ScriptableObject InventoryConfigAsset", soType != null);
                if (soType != null)
                {
                    Check.Equal("CodeLoop symbols: InventoryConfigAsset isScriptableObject is true", true, soType.IsScriptableObject);
                    Check.That("CodeLoop symbols: InventoryConfigAsset serializes MaxSlots", soType.FindSerializedField("MaxSlots") != null);
                    SerializedFieldRecord? weightLimit = soType.FindSerializedField("_weightLimit");
                    Check.That("CodeLoop symbols: InventoryConfigAsset serializes _weightLimit with former name", weightLimit != null && ContainsString(weightLimit.FormerlySerializedAs, "old_weight_limit"));
                    Check.That("CodeLoop symbols: InventoryConfigAsset omits [NonSerialized] TransientSessionToken", soType.FindSerializedField("TransientSessionToken") == null);
                    SerializedFieldRecord? loadoutEffect = soType.FindSerializedField("_defaultLoadoutEffect");
                    Check.That("CodeLoop symbols: InventoryConfigAsset serializes [SerializeReference] _defaultLoadoutEffect", loadoutEffect != null && loadoutEffect.IsSerializeReference);
                }

                string heroJson = heroQuery.ToJson();
                using (JsonDocument doc = JsonDocument.Parse(heroJson))
                {
                    JsonElement typesElem = doc.RootElement.GetProperty("types");
                    Check.Equal("CodeLoop symbols JSON: 1 type in filtered output", 1, typesElem.GetArrayLength());
                    JsonElement heroElem = typesElem[0];
                    Check.Equal("CodeLoop symbols JSON: fullName is MyGame.Gameplay.HeroActor", "MyGame.Gameplay.HeroActor", heroElem.GetProperty("fullName").GetString() ?? string.Empty);
                    Check.That("CodeLoop symbols JSON: serializedFields array has >= 7 entries", heroElem.GetProperty("serializedFields").GetArrayLength() >= 7);
                }
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestIdeCsprojDiagnosticParityWithDiagnose()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "codeloop_ide_parity");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                string brokenFile = Path.Combine(tempProject, "Assets", "Gameplay", "GameplayController.cs");
                string brokenContent =
                    "using MyGame.Core;\n"
                    + "using UnityEngine;\n"
                    + "\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    public class GameplayController : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int TriggerSeededError()\n"
                    + "        {\n"
                    + "            return undefined_symbol_in_gameplay + 5;\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n";
                File.WriteAllText(brokenFile, brokenContent);

                DiagnoseReport diagnoseReport = ScriptCompilation.DiagnoseProject(tempProject, options);
                Check.Equal("IDE parity with diagnose: diagnose fails on seeded error", false, diagnoseReport.Succeeded);
                DiagnoseEntry? diagnoseError = FindDiagnosticById(diagnoseReport.Diagnostics, "CS0103");
                Check.That("IDE parity with diagnose: diagnose reports CS0103", diagnoseError != null);

                IdeGenerationResult ideGen = IdeProjectGenerator.Generate(tempProject, options);
                Check.Equal("IDE parity with diagnose: IdeProjectGenerator succeeds", true, ideGen.Succeeded);
                IdeAssemblyProjectDescriptor? gameplayProj = ideGen.FindProject("MyGame.Gameplay");
                Check.That("IDE parity with diagnose: MyGame.Gameplay.csproj generated", gameplayProj != null && File.Exists(gameplayProj.CsprojPath));

                if (gameplayProj != null && diagnoseError != null)
                {
                    IReadOnlyList<ScriptDiagnostic> csprojDiagnostics = SymbolQuery.EvaluateCsprojDiagnostics(gameplayProj.CsprojPath);
                    ScriptDiagnostic? csprojError = null;
                    for (int i = 0; i < csprojDiagnostics.Count; i++)
                    {
                        if (csprojDiagnostics[i].Severity == ScriptDiagnosticSeverity.Error)
                        {
                            csprojError = csprojDiagnostics[i];
                            break;
                        }
                    }

                    Check.That("IDE parity with diagnose: generated .csproj evaluation reports error diagnostic", csprojError != null);
                    if (csprojError != null)
                    {
                        Check.Equal("IDE parity with diagnose: exact diagnostic identifier match (CS0103)", diagnoseError.Identifier, csprojError.Id);
                        Check.Equal(
                            "IDE parity with diagnose: exact source file match",
                            NormalizeRealPath(diagnoseError.File),
                            NormalizeRealPath(csprojError.File));
                        Check.Equal("IDE parity with diagnose: exact line number match", diagnoseError.Line, csprojError.Line);
                        Check.Equal("IDE parity with diagnose: exact column number match", diagnoseError.Column, csprojError.Column);
                    }
                }
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestCliDiagnoseAndSymbolsSubcommands()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "codeloop_cli");
            try
            {
                TextWriter savedOut = Console.Out;
                try
                {
                    using StringWriter cleanOut = new StringWriter();
                    Console.SetOut(cleanOut);
                    int cleanExit = Unitas.Scripting.Program.Main(new[]
                    {
                        "diagnose",
                        tempProject,
                        "--editor",
                        "--target",
                        "StandaloneOSX",
                        "--arch",
                        "x86_64"
                    });
                    Console.SetOut(savedOut);

                    Check.Equal("CLI diagnose: clean project exits 0", 0, cleanExit);
                    using (JsonDocument doc = JsonDocument.Parse(cleanOut.ToString()))
                    {
                        Check.Equal("CLI diagnose: clean JSON runningCompilation is 'new'", "new", doc.RootElement.GetProperty("runningCompilation").GetString() ?? string.Empty);
                    }

                    using StringWriter symbolsOut = new StringWriter();
                    Console.SetOut(symbolsOut);
                    int symbolsExit = Unitas.Scripting.Program.Main(new[]
                    {
                        "symbols",
                        tempProject,
                        "--type",
                        "CoreCounter"
                    });
                    Console.SetOut(savedOut);

                    Check.Equal("CLI symbols: exits 0", 0, symbolsExit);
                    using (JsonDocument symDoc = JsonDocument.Parse(symbolsOut.ToString()))
                    {
                        JsonElement types = symDoc.RootElement.GetProperty("types");
                        Check.Equal("CLI symbols: --type CoreCounter returns 1 type", 1, types.GetArrayLength());
                        Check.Equal("CLI symbols: returned type is MyGame.Core.CoreCounter", "MyGame.Core.CoreCounter", types[0].GetProperty("fullName").GetString() ?? string.Empty);
                    }

                    string brokenFile = Path.Combine(tempProject, "Assets", "Gameplay", "GameplayController.cs");
                    File.WriteAllText(
                        brokenFile,
                        "using UnityEngine;\nnamespace MyGame.Gameplay\n{\n    public class GameplayController : MonoBehaviour\n    {\n        public void Broken() { int a = \"bad\"; }\n    }\n}\n");

                    using StringWriter brokenOut = new StringWriter();
                    Console.SetOut(brokenOut);
                    int brokenExit = Unitas.Scripting.Program.Main(new[]
                    {
                        "diagnose",
                        tempProject,
                        "--editor",
                        "--target",
                        "StandaloneOSX",
                        "--arch",
                        "x86_64"
                    });
                    Console.SetOut(savedOut);

                    Check.Equal("CLI diagnose: broken project exits 1", 1, brokenExit);
                    using (JsonDocument brokenDoc = JsonDocument.Parse(brokenOut.ToString()))
                    {
                        Check.Equal("CLI diagnose: broken JSON runningCompilation is 'lastValid'", "lastValid", brokenDoc.RootElement.GetProperty("runningCompilation").GetString() ?? string.Empty);
                        Check.That("CLI diagnose: broken JSON has diagnostics", brokenDoc.RootElement.GetProperty("diagnostics").GetArrayLength() >= 1);
                    }
                }
                finally
                {
                    Console.SetOut(savedOut);
                }
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static DiagnoseEntry? FindDiagnosticById(IReadOnlyList<DiagnoseEntry> list, string id)
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (string.Equals(list[i].Identifier, id, StringComparison.OrdinalIgnoreCase))
                {
                    return list[i];
                }
            }

            return null;
        }

        private static bool ContainsString(IReadOnlyList<string> list, string target)
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (string.Equals(list[i], target, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool ContainsSubstring(IReadOnlyList<string> list, string substring)
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (list[i].Contains(substring, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static string NormalizeRealPath(string path)
        {
            string full = Path.GetFullPath(path).Replace('\\', '/');
            if (full.StartsWith("/private/var/", StringComparison.Ordinal))
            {
                return full.Substring("/private".Length);
            }

            if (full.StartsWith("/private/tmp/", StringComparison.Ordinal))
            {
                return full.Substring("/private".Length);
            }

            return full;
        }

        private static string ResolveFixtureProject()
        {
            string baseDir = AppContext.BaseDirectory;
            string? current = baseDir;
            while (!string.IsNullOrEmpty(current))
            {
                string candidate = Path.Combine(current, "bridge", "Unitas.Bridge.Tests", "Fixtures", "ScriptingProject");
                if (Directory.Exists(candidate))
                {
                    return candidate;
                }

                string localCandidate = Path.Combine(current, "Fixtures", "ScriptingProject");
                if (Directory.Exists(localCandidate))
                {
                    return localCandidate;
                }

                current = Path.GetDirectoryName(current);
            }

            return Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "bridge", "Unitas.Bridge.Tests", "Fixtures", "ScriptingProject"));
        }

        private static string CloneProjectToTemp(string sourceDir, string label)
        {
            string baseTemp = Path.GetTempPath();
            if (OperatingSystem.IsMacOS())
            {
                if (baseTemp.StartsWith("/var/", StringComparison.Ordinal) && Directory.Exists("/private" + baseTemp))
                {
                    baseTemp = "/private" + baseTemp;
                }
                else if (baseTemp.StartsWith("/tmp/", StringComparison.Ordinal) && Directory.Exists("/private" + baseTemp))
                {
                    baseTemp = "/private" + baseTemp;
                }
            }

            string tempDir = Path.Combine(
                baseTemp,
                "unitas_codeloop_test_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            CopyDirectoryRecursive(sourceDir, tempDir);
            return tempDir;
        }

        private static void CopyDirectoryRecursive(string sourceDir, string targetDir)
        {
            Directory.CreateDirectory(targetDir);
            string[] files = Directory.GetFiles(sourceDir);
            for (int i = 0; i < files.Length; i++)
            {
                string fileName = Path.GetFileName(files[i]);
                File.Copy(files[i], Path.Combine(targetDir, fileName), true);
            }

            string[] dirs = Directory.GetDirectories(sourceDir);
            for (int i = 0; i < dirs.Length; i++)
            {
                string dirName = Path.GetFileName(dirs[i]);
                if (string.Equals(dirName, "Library", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(dirName, "obj", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(dirName, "bin", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                CopyDirectoryRecursive(dirs[i], Path.Combine(targetDir, dirName));
            }
        }

        private static void DeleteDirectoryIfExists(string dir)
        {
            if (Directory.Exists(dir))
            {
                try
                {
                    Directory.Delete(dir, true);
                }
                catch (IOException)
                {
                }
            }
        }
    }
}
