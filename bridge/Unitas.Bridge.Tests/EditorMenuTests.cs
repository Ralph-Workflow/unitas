using System.Collections.Generic;
using UnityEditor;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class EditorMenuTests
    {
        private static bool s_canRunBake;
        private static int s_bakeExecutionCount;
        private static string s_lastObservedSessionInMenu = string.Empty;

        [MenuItem("Tools/Bake Navigation Mesh %#b", false, 150)]
        public static void ExecuteBakeNavigationMesh()
        {
            s_bakeExecutionCount++;
            s_lastObservedSessionInMenu = MenuItem.CurrentInvokingSessionId ?? string.Empty;
        }

        [MenuItem("Tools/Bake Navigation Mesh %#b", true, 150)]
        public static bool ValidateBakeNavigationMesh()
        {
            return s_canRunBake;
        }

        public static void Run()
        {
            TestStory27MenuItemAttributePropertiesAndShortcutParsing();
            TestStory27CommandPaletteAndAutomationToolDiscoveryAndValidation();
            TestStory27AgentSessionAttributionAndUndoableMutation();
            TestStory44DisplayDialogNonBlockingDefaultAndOverrideInAgentSession();
            TestStory44DisplayDialogComplexFromMenuCommandInAgentSession();
        }

        private static void TestStory27MenuItemAttributePropertiesAndShortcutParsing()
        {
            MenuItem plain = new MenuItem("Tools/Rebuild Lighting");
            Check.Equal("Story 27 default MenuItem menuItem path", "Tools/Rebuild Lighting", plain.menuItem);
            Check.Equal("Story 27 default MenuItem validate is false", false, plain.validate);
            Check.Equal("Story 27 default MenuItem priority is 1000", 1000, plain.priority);
            Check.Equal("Story 27 default MenuItem shortcut is empty", string.Empty, plain.shortcut);

            MenuItem validateOnly = new MenuItem("Tools/Rebuild Lighting", true);
            Check.Equal("Story 27 validate MenuItem validate is true", true, validateOnly.validate);
            Check.Equal("Story 27 validate MenuItem default priority is 1000", 1000, validateOnly.priority);

            MenuItem withShortcutAndPriority = new MenuItem("Tools/Bake Navigation Mesh %#b", false, 150);
            Check.Equal("Story 27 shortcut MenuItem strips shortcut from menuItem path", "Tools/Bake Navigation Mesh", withShortcutAndPriority.menuItem);
            Check.Equal("Story 27 shortcut MenuItem extracts shortcut token", "%#b", withShortcutAndPriority.shortcut);
            Check.Equal("Story 27 shortcut MenuItem preserves priority 150", 150, withShortcutAndPriority.priority);
            Check.Equal("Story 27 BuildToolId normalizes menu path", "menu.tools.bake_navigation_mesh", MenuItem.BuildToolId(withShortcutAndPriority.menuItem));
        }

        private static void TestStory27CommandPaletteAndAutomationToolDiscoveryAndValidation()
        {
            MenuItem.ClearRegisteredCommands();
            EditorUtility.ClearDialogState();
            s_canRunBake = false;
            s_bakeExecutionCount = 0;
            s_lastObservedSessionInMenu = string.Empty;

            try
            {
                MenuItem.RegisterCommand(
                    "Tools/Export Report",
                    () => { },
                    priority: 300,
                    assemblyName: "MyGame.EditorTools",
                    declaringTypeName: "MyGame.EditorTools.ReportMenu",
                    methodName: "ExportReport");

                MenuItem.RegisterCommand(
                    "Tools/Bake Navigation Mesh %#b",
                    ExecuteBakeNavigationMesh,
                    priority: 150,
                    assemblyName: "MyGame.EditorTools",
                    declaringTypeName: "Unitas.Bridge.Tests.EditorMenuTests",
                    methodName: nameof(ExecuteBakeNavigationMesh));

                MenuItem.RegisterValidation(
                    "Tools/Bake Navigation Mesh %#b",
                    ValidateBakeNavigationMesh,
                    disabledReason: "No active navigation surface is selected.");

                IReadOnlyList<MenuCommandDescriptor> paletteDisabled = MenuItem.GetCommandPaletteEntries();
                Check.Equal("Story 27 command palette lists 2 registered commands", 2, paletteDisabled.Count);
                Check.Equal("Story 27 command palette orders by priority ascending (150 first)", "Tools/Bake Navigation Mesh", paletteDisabled[0].MenuPath);
                Check.Equal("Story 27 command palette second item has priority 300", "Tools/Export Report", paletteDisabled[1].MenuPath);
                Check.Equal("Story 27 command palette entry exposes display name", "Bake Navigation Mesh", paletteDisabled[0].DisplayName);
                Check.Equal("Story 27 command palette entry exposes shortcut", "%#b", paletteDisabled[0].Shortcut);
                Check.Equal("Story 27 command palette entry records source assembly", "MyGame.EditorTools", paletteDisabled[0].AssemblyName);
                Check.Equal("Story 27 command palette entry records declaring type", "Unitas.Bridge.Tests.EditorMenuTests", paletteDisabled[0].DeclaringTypeName);
                Check.Equal("Story 27 command palette entry records method name", "ExecuteBakeNavigationMesh", paletteDisabled[0].MethodName);
                Check.Equal("Story 27 command palette entry is disabled when validator returns false", false, paletteDisabled[0].IsEnabled);
                Check.Equal(
                    "Story 27 command palette disabled entry reports validation failure reason",
                    "No active navigation surface is selected.",
                    paletteDisabled[0].DisabledReason);

                IReadOnlyList<MenuAutomationToolDescriptor> toolsDisabled = MenuItem.GetAutomationTools();
                Check.Equal("Story 27 automation tool surface lists 2 tools", 2, toolsDisabled.Count);
                Check.Equal("Story 27 automation tool shares toolId with command palette", paletteDisabled[0].ToolId, toolsDisabled[0].ToolId);
                Check.Equal("Story 27 automation tool keeps the command menu path", "Tools/Bake Navigation Mesh", toolsDisabled[0].MenuPath);
                Check.Equal("Story 27 automation tool keeps the command display name", "Bake Navigation Mesh", toolsDisabled[0].DisplayName);
                Check.Equal("Story 27 automation tool keeps the command priority", 150, toolsDisabled[0].Priority);
                Check.Equal("Story 27 automation tool keeps the command shortcut", "%#b", toolsDisabled[0].Shortcut);
                Check.Equal("Story 27 automation tool is marked project-contributed", "project", toolsDisabled[0].Origin);
                Check.Equal("Story 27 automation tool records source assembly", "MyGame.EditorTools", toolsDisabled[0].AssemblyName);
                Check.Equal("Story 27 automation tool reflects disabled availability when validation fails", false, toolsDisabled[0].IsAvailable);
                Check.Equal(
                    "Story 27 automation tool exposes disabled reason when validation fails",
                    "No active navigation surface is selected.",
                    toolsDisabled[0].DisabledReason);

                MenuCommandInvocationResult refusedResult = MenuItem.Invoke("Tools/Bake Navigation Mesh", "session-agent-27");
                Check.Equal("Story 27 invoking disabled command is refused", false, refusedResult.Succeeded);
                Check.Equal(
                    "Story 27 refused invocation reports validation reason",
                    "No active navigation surface is selected.",
                    refusedResult.RefusalReason);
                Check.Equal("Story 27 refused command does not execute callback", 0, s_bakeExecutionCount);

                s_canRunBake = true;

                MenuCommandDescriptor? enabledCmd = MenuItem.FindCommand("menu.tools.bake_navigation_mesh");
                Check.That("Story 27 FindCommand locates command by automation toolId", enabledCmd != null);
                if (enabledCmd != null)
                {
                    Check.Equal("Story 27 command becomes enabled when validation returns true", true, enabledCmd.IsEnabled);
                    Check.Equal("Story 27 enabled command has empty disabled reason", string.Empty, enabledCmd.DisabledReason);
                }

                IReadOnlyList<MenuAutomationToolDescriptor> toolsEnabled = MenuItem.GetAutomationTools();
                Check.Equal("Story 27 automation tool becomes available when validation returns true", true, toolsEnabled[0].IsAvailable);

                MenuCommandInvocationResult okResult = MenuItem.Invoke("menu.tools.bake_navigation_mesh", "session-agent-27");
                Check.Equal("Story 27 invoking enabled command by toolId succeeds", true, okResult.Succeeded);
                Check.Equal("Story 27 enabled command executes callback once", 1, s_bakeExecutionCount);
                Check.Equal("Story 27 callback observes invoking agent session ID", "session-agent-27", s_lastObservedSessionInMenu);
            }
            finally
            {
                MenuItem.ClearRegisteredCommands();
                EditorUtility.ClearDialogState();
            }
        }

        private static void TestStory27AgentSessionAttributionAndUndoableMutation()
        {
            MenuItem.ClearRegisteredCommands();
            EditorUtility.ClearDialogState();

            GameObject assetObject = new GameObject("GeneratedNavAsset");
            string simulatedAssetLabel = "InitialAssetContent";

            try
            {
                MenuItem.RegisterCommand(
                    "Tools/Modify Nav Asset",
                    () =>
                    {
                        string previousLabel = simulatedAssetLabel;
                        simulatedAssetLabel = "MutatedByMenuCommand";
                        MenuItem.RecordAttributedMutation(
                            "Assets/Nav/GeneratedNavAsset.asset",
                            "AssetDatabase.WriteAsset",
                            () => { simulatedAssetLabel = previousLabel; });
                        EditorUtility.SetDirty(assetObject);
                    });

                MenuCommandInvocationResult result = MenuItem.Invoke("Tools/Modify Nav Asset", "session-agent-42");
                Check.Equal("Story 27 agent invocation of asset-modifying MenuItem succeeds", true, result.Succeeded);
                Check.Equal("Story 27 invocation result records invoking agent session", "session-agent-42", result.AgentSessionId ?? string.Empty);
                Check.Equal("Story 27 invocation result captures 2 attributed mutations", 2, result.Mutations.Count);
                Check.Equal("Story 27 mutation applied to asset state", "MutatedByMenuCommand", simulatedAssetLabel);
                Check.Equal("Story 27 EditorUtility.SetDirty marks target object dirty", true, EditorUtility.IsDirty(assetObject));

                Check.Equal("Story 27 first mutation attributed to session-agent-42", "session-agent-42", result.Mutations[0].AgentSessionId);
                Check.Equal("Story 27 first mutation records command path", "Tools/Modify Nav Asset", result.Mutations[0].CommandPath);
                Check.Equal("Story 27 first mutation records target asset path", "Assets/Nav/GeneratedNavAsset.asset", result.Mutations[0].TargetIdentity);
                Check.Equal("Story 27 first mutation is undoable", true, result.Mutations[0].IsUndoable);

                Check.Equal("Story 27 second mutation (SetDirty) attributed to session-agent-42", "session-agent-42", result.Mutations[1].AgentSessionId);
                Check.Equal("Story 27 second mutation records EditorUtility.SetDirty kind", "EditorUtility.SetDirty", result.Mutations[1].OperationKind);

                bool undidOtherSession = MenuItem.UndoLastMutationForSession("session-other");
                Check.Equal("Story 27 undo for unrelated agent session changes nothing", false, undidOtherSession);
                Check.Equal("Story 27 asset remains dirty after unrelated session undo", true, EditorUtility.IsDirty(assetObject));

                bool undidDirty = MenuItem.UndoLastMutationForSession("session-agent-42");
                Check.Equal("Story 27 first undo for session-agent-42 reverts SetDirty", true, undidDirty);
                Check.Equal("Story 27 undone mutation is marked as undone", true, result.Mutations[1].WasUndone);
                Check.Equal("Story 27 target object is no longer dirty after undo", false, EditorUtility.IsDirty(assetObject));

                bool undidAssetWrite = MenuItem.UndoLastMutationForSession("session-agent-42");
                Check.Equal("Story 27 second undo for session-agent-42 reverts asset mutation", true, undidAssetWrite);
                Check.Equal("Story 27 asset content restored after undo", "InitialAssetContent", simulatedAssetLabel);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(assetObject);
                MenuItem.ClearRegisteredCommands();
                EditorUtility.ClearDialogState();
            }
        }

        private static void TestStory44DisplayDialogNonBlockingDefaultAndOverrideInAgentSession()
        {
            MenuItem.ClearRegisteredCommands();
            EditorUtility.ClearDialogState();

            try
            {
                EditorUtility.CurrentAgentSessionId = "session-agent-44";

                bool defaultOutcome = EditorUtility.DisplayDialog(
                    "Confirm Asset Overwrite",
                    "Overwrite existing navigation data?",
                    "Overwrite",
                    "Cancel");

                Check.Equal("Story 44 DisplayDialog without supplied answer returns true (ok) immediately", true, defaultOutcome);
                IReadOnlyList<EditorDialogEventRecord> eventsAfterDefault = EditorUtility.RecordedDialogEvents;
                Check.Equal("Story 44 DisplayDialog records 1 structured dialog event", 1, eventsAfterDefault.Count);
                Check.Equal("Story 44 event records dialog kind DisplayDialog", "DisplayDialog", eventsAfterDefault[0].DialogKind);
                Check.Equal("Story 44 event records dialog title", "Confirm Asset Overwrite", eventsAfterDefault[0].Title);
                Check.Equal("Story 44 event records dialog message", "Overwrite existing navigation data?", eventsAfterDefault[0].Message);
                Check.Equal("Story 44 event records 2 buttons", 2, eventsAfterDefault[0].Buttons.Count);
                Check.Equal("Story 44 event records ok button label", "Overwrite", eventsAfterDefault[0].ChosenButton);
                Check.Equal("Story 44 event records boolean result true", true, eventsAfterDefault[0].BooleanResult);
                Check.Equal("Story 44 event records UsedSuppliedAnswer false for default", false, eventsAfterDefault[0].UsedSuppliedAnswer);
                Check.Equal("Story 44 event attributes dialog to active agent session", "session-agent-44", eventsAfterDefault[0].SessionId ?? string.Empty);

                EditorUtility.ConfigureDialogAnswer("Confirm Asset Overwrite", "Cancel");

                bool overrideCancelOutcome = EditorUtility.DisplayDialog(
                    "Confirm Asset Overwrite",
                    "Overwrite existing navigation data?",
                    "Overwrite",
                    "Cancel");

                Check.Equal("Story 44 DisplayDialog with supplied Cancel answer returns false", false, overrideCancelOutcome);
                IReadOnlyList<EditorDialogEventRecord> eventsAfterCancel = EditorUtility.RecordedDialogEvents;
                Check.Equal("Story 44 second DisplayDialog appends second structured event", 2, eventsAfterCancel.Count);
                Check.Equal("Story 44 override event records ChosenButton Cancel", "Cancel", eventsAfterCancel[1].ChosenButton);
                Check.Equal("Story 44 override event records BooleanResult false", false, eventsAfterCancel[1].BooleanResult);
                Check.Equal("Story 44 override event records UsedSuppliedAnswer true", true, eventsAfterCancel[1].UsedSuppliedAnswer);
                Check.Equal("Story 44 override event records session-agent-44", "session-agent-44", eventsAfterCancel[1].SessionId ?? string.Empty);

                EditorUtility.ConfigureDialogAnswer("Delete Temporary Cache", chooseOk: false);
                bool boolOverrideOutcome = EditorUtility.DisplayDialog(
                    "Delete Temporary Cache",
                    "Remove temporary build cache?",
                    "Delete",
                    "Keep");
                Check.Equal("Story 44 DisplayDialog with boolean false override returns false", false, boolOverrideOutcome);
                IReadOnlyList<EditorDialogEventRecord> eventsAfterBool = EditorUtility.RecordedDialogEvents;
                Check.Equal("Story 44 boolean override selects cancel button label 'Keep'", "Keep", eventsAfterBool[2].ChosenButton);
                Check.Equal("Story 44 boolean override marks UsedSuppliedAnswer true", true, eventsAfterBool[2].UsedSuppliedAnswer);
            }
            finally
            {
                MenuItem.ClearRegisteredCommands();
                EditorUtility.ClearDialogState();
            }
        }

        private static void TestStory44DisplayDialogComplexFromMenuCommandInAgentSession()
        {
            MenuItem.ClearRegisteredCommands();
            EditorUtility.ClearDialogState();

            try
            {
                int complexResultFromMenu = -1;
                MenuItem.RegisterCommand(
                    "Tools/Run Scene UpgradeWithDialog",
                    () =>
                    {
                        complexResultFromMenu = EditorUtility.DisplayDialogComplex(
                            "Save Modified Scene",
                            "Scene has unsaved changes before upgrade.",
                            "Save",
                            "Cancel",
                            "Don't Save");
                    });

                EditorUtility.ConfigureDialogAnswer("Save Modified Scene", "Don't Save");

                MenuCommandInvocationResult invokeRes = MenuItem.Invoke("Tools/Run Scene UpgradeWithDialog", "session-agent-99");
                Check.Equal("Story 44 MenuItem calling DisplayDialogComplex succeeds without blocking", true, invokeRes.Succeeded);
                Check.Equal("Story 44 DisplayDialogComplex with 'Don't Save' override returns alt index 2", 2, complexResultFromMenu);

                IReadOnlyList<EditorDialogEventRecord> dialogEvents = EditorUtility.RecordedDialogEvents;
                Check.Equal("Story 44 DisplayDialogComplex records 1 dialog event", 1, dialogEvents.Count);
                Check.Equal("Story 44 DisplayDialogComplex event kind", "DisplayDialogComplex", dialogEvents[0].DialogKind);
                Check.Equal("Story 44 DisplayDialogComplex event title", "Save Modified Scene", dialogEvents[0].Title);
                Check.Equal("Story 44 DisplayDialogComplex event message", "Scene has unsaved changes before upgrade.", dialogEvents[0].Message);
                Check.Equal("Story 44 DisplayDialogComplex event records 3 buttons", 3, dialogEvents[0].Buttons.Count);
                Check.Equal("Story 44 DisplayDialogComplex event chosen button", "Don't Save", dialogEvents[0].ChosenButton);
                Check.Equal("Story 44 DisplayDialogComplex event chosen index is 2", 2, dialogEvents[0].ChosenIndex);
                Check.Equal("Story 44 DisplayDialogComplex event boolean result is false for alt button", false, dialogEvents[0].BooleanResult);
                Check.Equal("Story 44 DisplayDialogComplex event UsedSuppliedAnswer is true", true, dialogEvents[0].UsedSuppliedAnswer);
                Check.Equal("Story 44 DisplayDialogComplex inherits invoking MenuItem agent session ID", "session-agent-99", dialogEvents[0].SessionId ?? string.Empty);
            }
            finally
            {
                MenuItem.ClearRegisteredCommands();
                EditorUtility.ClearDialogState();
            }
        }
    }
}
