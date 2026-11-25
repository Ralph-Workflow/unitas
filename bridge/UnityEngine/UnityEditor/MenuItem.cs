using System;
using System.Collections.Generic;

namespace UnityEditor
{
    [AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = false)]
    public sealed class MenuItem : Attribute
    {
        private static readonly object s_gate = new object();
        private static readonly Dictionary<string, MenuCommandEntry> s_entries =
            new Dictionary<string, MenuCommandEntry>(StringComparer.Ordinal);
        private static readonly List<string> s_registrationOrder = new List<string>();
        private static readonly List<MenuAgentMutationRecord> s_agentMutations =
            new List<MenuAgentMutationRecord>();

        private sealed class MenuCommandEntry
        {
            public MenuCommandEntry(string itemName)
            {
                ItemName = itemName;
                Priority = 1000;
            }

            public string ItemName { get; }

            public Action? ActionCallback { get; set; }

            public Func<bool>? ValidateCallback { get; set; }

            public string DisabledReason { get; set; } = string.Empty;

            public int Priority { get; set; }

            public string Shortcut { get; set; } = string.Empty;

            public string AssemblyName { get; set; } = string.Empty;

            public string DeclaringTypeName { get; set; } = string.Empty;

            public string MethodName { get; set; } = string.Empty;
        }

        public MenuItem(string itemName)
            : this(itemName, false, 1000)
        {
        }

        public MenuItem(string itemName, bool isValidateFunction)
            : this(itemName, isValidateFunction, 1000)
        {
        }

        public MenuItem(string itemName, bool isValidateFunction, int priority)
        {
            if (itemName == null)
            {
                throw new ArgumentNullException(nameof(itemName));
            }

            ParseItemNameAndShortcut(itemName, out string parsedMenuItem, out string parsedShortcut);
            menuItem = parsedMenuItem;
            validate = isValidateFunction;
            this.priority = priority;
            shortcut = parsedShortcut;
        }

        public string menuItem { get; set; }

        public bool validate { get; set; }

        public int priority { get; set; }

        public string shortcut { get; }

        public static string? CurrentInvokingSessionId { get; private set; }

        public static string? CurrentInvokingCommandPath { get; private set; }

        public static void RegisterCommand(
            string itemName,
            Action callback,
            int priority = 1000,
            string? assemblyName = null,
            string? declaringTypeName = null,
            string? methodName = null)
        {
            if (itemName == null)
            {
                throw new ArgumentNullException(nameof(itemName));
            }

            if (callback == null)
            {
                throw new ArgumentNullException(nameof(callback));
            }

            ParseItemNameAndShortcut(itemName, out string normalizedPath, out string parsedShortcut);
            if (string.IsNullOrEmpty(normalizedPath))
            {
                throw new ArgumentException("Menu item path must be non-empty.", nameof(itemName));
            }

            lock (s_gate)
            {
                MenuCommandEntry entry = GetOrAddEntryLocked(normalizedPath);
                entry.ActionCallback = callback;
                entry.Priority = priority;
                if (!string.IsNullOrEmpty(parsedShortcut))
                {
                    entry.Shortcut = parsedShortcut;
                }

                if (!string.IsNullOrEmpty(assemblyName))
                {
                    entry.AssemblyName = assemblyName!;
                }

                if (!string.IsNullOrEmpty(declaringTypeName))
                {
                    entry.DeclaringTypeName = declaringTypeName!;
                }

                if (!string.IsNullOrEmpty(methodName))
                {
                    entry.MethodName = methodName!;
                }
            }
        }

        public static void RegisterValidation(
            string itemName,
            Func<bool> validator,
            string? disabledReason = null,
            int priority = 1000)
        {
            if (itemName == null)
            {
                throw new ArgumentNullException(nameof(itemName));
            }

            if (validator == null)
            {
                throw new ArgumentNullException(nameof(validator));
            }

            ParseItemNameAndShortcut(itemName, out string normalizedPath, out string parsedShortcut);
            if (string.IsNullOrEmpty(normalizedPath))
            {
                throw new ArgumentException("Menu item path must be non-empty.", nameof(itemName));
            }

            lock (s_gate)
            {
                MenuCommandEntry entry = GetOrAddEntryLocked(normalizedPath);
                entry.ValidateCallback = validator;
                if (!string.IsNullOrEmpty(disabledReason))
                {
                    entry.DisabledReason = disabledReason!;
                }

                if (entry.ActionCallback == null)
                {
                    entry.Priority = priority;
                }

                if (string.IsNullOrEmpty(entry.Shortcut) && !string.IsNullOrEmpty(parsedShortcut))
                {
                    entry.Shortcut = parsedShortcut;
                }
            }
        }

        public static void ClearRegisteredCommands()
        {
            lock (s_gate)
            {
                s_entries.Clear();
                s_registrationOrder.Clear();
                s_agentMutations.Clear();
                CurrentInvokingSessionId = null;
                CurrentInvokingCommandPath = null;
            }
        }

        public static IReadOnlyList<MenuCommandDescriptor> GetCommandPaletteEntries()
        {
            List<MenuCommandEntry> snapshot;
            lock (s_gate)
            {
                snapshot = GetOrderedActionEntriesLocked();
            }

            List<MenuCommandDescriptor> palette = new List<MenuCommandDescriptor>(snapshot.Count);
            for (int i = 0; i < snapshot.Count; i++)
            {
                palette.Add(EvaluateDescriptor(snapshot[i]));
            }

            return palette;
        }

        public static IReadOnlyList<MenuAutomationToolDescriptor> GetAutomationTools()
        {
            IReadOnlyList<MenuCommandDescriptor> commands = GetCommandPaletteEntries();
            List<MenuAutomationToolDescriptor> tools = new List<MenuAutomationToolDescriptor>(commands.Count);
            for (int i = 0; i < commands.Count; i++)
            {
                MenuCommandDescriptor cmd = commands[i];
                tools.Add(new MenuAutomationToolDescriptor(
                    cmd.ToolId,
                    cmd.MenuPath,
                    cmd.DisplayName,
                    cmd.Priority,
                    cmd.Shortcut,
                    cmd.IsEnabled,
                    cmd.DisabledReason,
                    "project",
                    string.IsNullOrEmpty(cmd.AssemblyName) ? "Assembly-CSharp-Editor" : cmd.AssemblyName));
            }

            return tools;
        }

        public static MenuCommandDescriptor? FindCommand(string itemNameOrToolId)
        {
            if (string.IsNullOrEmpty(itemNameOrToolId))
            {
                return null;
            }

            IReadOnlyList<MenuCommandDescriptor> palette = GetCommandPaletteEntries();
            ParseItemNameAndShortcut(itemNameOrToolId, out string normalizedTarget, out string _);
            for (int i = 0; i < palette.Count; i++)
            {
                MenuCommandDescriptor d = palette[i];
                if (string.Equals(d.MenuPath, normalizedTarget, StringComparison.Ordinal)
                    || string.Equals(d.ToolId, itemNameOrToolId, StringComparison.Ordinal))
                {
                    return d;
                }
            }

            return null;
        }

        public static MenuCommandInvocationResult Invoke(string itemNameOrToolId, string? agentSessionId = null)
        {
            if (string.IsNullOrEmpty(itemNameOrToolId))
            {
                throw new ArgumentException("Menu item name or tool identifier must be non-empty.", nameof(itemNameOrToolId));
            }

            ParseItemNameAndShortcut(itemNameOrToolId, out string normalizedTarget, out string _);
            MenuCommandEntry? found = null;
            lock (s_gate)
            {
                if (!s_entries.TryGetValue(normalizedTarget, out found))
                {
                    for (int i = 0; i < s_registrationOrder.Count; i++)
                    {
                        string key = s_registrationOrder[i];
                        if (string.Equals(BuildToolId(key), itemNameOrToolId, StringComparison.Ordinal))
                        {
                            found = s_entries[key];
                            break;
                        }
                    }
                }
            }

            if (found == null || found.ActionCallback == null)
            {
                return new MenuCommandInvocationResult(
                    false,
                    normalizedTarget,
                    BuildToolId(normalizedTarget),
                    agentSessionId,
                    "Menu command '" + itemNameOrToolId + "' is not registered.",
                    Array.Empty<MenuAgentMutationRecord>());
            }

            MenuCommandDescriptor descriptor = EvaluateDescriptor(found);
            if (!descriptor.IsEnabled)
            {
                return new MenuCommandInvocationResult(
                    false,
                    descriptor.MenuPath,
                    descriptor.ToolId,
                    agentSessionId,
                    descriptor.DisabledReason,
                    Array.Empty<MenuAgentMutationRecord>());
            }

            int mutationsStartCount;
            string? previousSession = CurrentInvokingSessionId;
            string? previousCommand = CurrentInvokingCommandPath;
            string? previousDialogSession = EditorUtility.CurrentAgentSessionId;

            lock (s_gate)
            {
                mutationsStartCount = s_agentMutations.Count;
                CurrentInvokingSessionId = agentSessionId;
                CurrentInvokingCommandPath = descriptor.MenuPath;
                if (!string.IsNullOrEmpty(agentSessionId))
                {
                    EditorUtility.CurrentAgentSessionId = agentSessionId;
                }
            }

            try
            {
                found.ActionCallback();
            }
            finally
            {
                lock (s_gate)
                {
                    CurrentInvokingSessionId = previousSession;
                    CurrentInvokingCommandPath = previousCommand;
                    EditorUtility.CurrentAgentSessionId = previousDialogSession;
                }
            }

            List<MenuAgentMutationRecord> recordedForInvocation = new List<MenuAgentMutationRecord>();
            lock (s_gate)
            {
                for (int i = mutationsStartCount; i < s_agentMutations.Count; i++)
                {
                    recordedForInvocation.Add(s_agentMutations[i]);
                }
            }

            return new MenuCommandInvocationResult(
                true,
                descriptor.MenuPath,
                descriptor.ToolId,
                agentSessionId,
                string.Empty,
                recordedForInvocation);
        }

        public static void RecordAttributedMutation(
            string targetPathOrIdentity,
            string operationKind,
            Action? undoCallback = null)
        {
            if (string.IsNullOrEmpty(targetPathOrIdentity))
            {
                throw new ArgumentException("Target path or identity must be non-empty.", nameof(targetPathOrIdentity));
            }

            if (string.IsNullOrEmpty(operationKind))
            {
                throw new ArgumentException("Operation kind must be non-empty.", nameof(operationKind));
            }

            lock (s_gate)
            {
                string sessionId = CurrentInvokingSessionId ?? EditorUtility.CurrentAgentSessionId ?? "person";
                string commandPath = CurrentInvokingCommandPath ?? string.Empty;
                s_agentMutations.Add(new MenuAgentMutationRecord(
                    sessionId,
                    commandPath,
                    targetPathOrIdentity,
                    operationKind,
                    undoCallback != null,
                    undoCallback));
            }
        }

        public static bool UndoLastMutationForSession(string agentSessionId)
        {
            if (string.IsNullOrEmpty(agentSessionId))
            {
                throw new ArgumentException("Agent session ID must be non-empty.", nameof(agentSessionId));
            }

            Action? undoToRun = null;
            lock (s_gate)
            {
                for (int i = s_agentMutations.Count - 1; i >= 0; i--)
                {
                    MenuAgentMutationRecord rec = s_agentMutations[i];
                    if (string.Equals(rec.AgentSessionId, agentSessionId, StringComparison.Ordinal)
                        && rec.IsUndoable
                        && !rec.WasUndone)
                    {
                        rec.MarkUndone();
                        undoToRun = rec.UndoCallback;
                        break;
                    }
                }
            }

            if (undoToRun == null)
            {
                return false;
            }

            undoToRun();
            return true;
        }

        public static string BuildToolId(string menuPath)
        {
            if (string.IsNullOrEmpty(menuPath))
            {
                return "menu.";
            }

            char[] buffer = new char[menuPath.Length];
            int length = 0;
            bool lastWasSeparator = false;
            for (int i = 0; i < menuPath.Length; i++)
            {
                char c = menuPath[i];
                if (c == '/')
                {
                    if (!lastWasSeparator && length > 0)
                    {
                        buffer[length++] = '.';
                        lastWasSeparator = true;
                    }
                }
                else if (char.IsLetterOrDigit(c))
                {
                    buffer[length++] = char.ToLowerInvariant(c);
                    lastWasSeparator = false;
                }
                else if (c == ' ' || c == '-' || c == '_')
                {
                    if (!lastWasSeparator && length > 0)
                    {
                        buffer[length++] = '_';
                        lastWasSeparator = true;
                    }
                }
            }

            while (length > 0 && (buffer[length - 1] == '.' || buffer[length - 1] == '_'))
            {
                length--;
            }

            return "menu." + new string(buffer, 0, length);
        }

        private static MenuCommandEntry GetOrAddEntryLocked(string normalizedPath)
        {
            if (!s_entries.TryGetValue(normalizedPath, out MenuCommandEntry? entry))
            {
                entry = new MenuCommandEntry(normalizedPath);
                s_entries.Add(normalizedPath, entry);
                s_registrationOrder.Add(normalizedPath);
            }

            return entry;
        }

        private static List<MenuCommandEntry> GetOrderedActionEntriesLocked()
        {
            List<MenuCommandEntry> list = new List<MenuCommandEntry>();
            for (int i = 0; i < s_registrationOrder.Count; i++)
            {
                MenuCommandEntry entry = s_entries[s_registrationOrder[i]];
                if (entry.ActionCallback != null)
                {
                    list.Add(entry);
                }
            }

            list.Sort((a, b) =>
            {
                int cmp = a.Priority.CompareTo(b.Priority);
                if (cmp != 0)
                {
                    return cmp;
                }

                return string.CompareOrdinal(a.ItemName, b.ItemName);
            });
            return list;
        }

        private static MenuCommandDescriptor EvaluateDescriptor(MenuCommandEntry entry)
        {
            bool isEnabled = true;
            string reason = string.Empty;
            if (entry.ValidateCallback != null)
            {
                isEnabled = entry.ValidateCallback();
                if (!isEnabled)
                {
                    reason = !string.IsNullOrEmpty(entry.DisabledReason)
                        ? entry.DisabledReason
                        : "Validation function for '" + entry.ItemName + "' returned false.";
                }
            }

            int lastSlash = entry.ItemName.LastIndexOf('/');
            string displayName = lastSlash >= 0 && lastSlash + 1 < entry.ItemName.Length
                ? entry.ItemName.Substring(lastSlash + 1)
                : entry.ItemName;

            return new MenuCommandDescriptor(
                entry.ItemName,
                BuildToolId(entry.ItemName),
                displayName,
                entry.Priority,
                entry.Shortcut,
                isEnabled,
                reason,
                entry.AssemblyName,
                entry.DeclaringTypeName,
                entry.MethodName);
        }

        private static void ParseItemNameAndShortcut(string rawItemName, out string menuPath, out string shortcut)
        {
            string trimmed = rawItemName.Trim();
            int lastSpace = trimmed.LastIndexOf(' ');
            if (lastSpace > 0 && lastSpace + 1 < trimmed.Length)
            {
                string trailing = trimmed.Substring(lastSpace + 1);
                if (IsUnityMenuShortcutToken(trailing))
                {
                    menuPath = trimmed.Substring(0, lastSpace).TrimEnd();
                    shortcut = trailing;
                    return;
                }
            }

            menuPath = trimmed;
            shortcut = string.Empty;
        }

        private static bool IsUnityMenuShortcutToken(string token)
        {
            if (string.IsNullOrEmpty(token))
            {
                return false;
            }

            char first = token[0];
            return (first == '%' || first == '#' || first == '&' || first == '_') && token.Length >= 2;
        }
    }

    public sealed class MenuCommandDescriptor
    {
        public MenuCommandDescriptor(
            string menuPath,
            string toolId,
            string displayName,
            int priority,
            string shortcut,
            bool isEnabled,
            string disabledReason,
            string assemblyName,
            string declaringTypeName,
            string methodName)
        {
            MenuPath = menuPath;
            ToolId = toolId;
            DisplayName = displayName;
            Priority = priority;
            Shortcut = shortcut;
            IsEnabled = isEnabled;
            DisabledReason = disabledReason;
            AssemblyName = assemblyName;
            DeclaringTypeName = declaringTypeName;
            MethodName = methodName;
        }

        public string MenuPath { get; }

        public string ToolId { get; }

        public string DisplayName { get; }

        public int Priority { get; }

        public string Shortcut { get; }

        public bool IsEnabled { get; }

        public string DisabledReason { get; }

        public string AssemblyName { get; }

        public string DeclaringTypeName { get; }

        public string MethodName { get; }
    }

    public sealed class MenuAutomationToolDescriptor
    {
        public MenuAutomationToolDescriptor(
            string toolId,
            string menuPath,
            string displayName,
            int priority,
            string shortcut,
            bool isAvailable,
            string disabledReason,
            string origin,
            string assemblyName)
        {
            ToolId = toolId;
            MenuPath = menuPath;
            DisplayName = displayName;
            Priority = priority;
            Shortcut = shortcut;
            IsAvailable = isAvailable;
            DisabledReason = disabledReason;
            Origin = origin;
            AssemblyName = assemblyName;
        }

        public string ToolId { get; }

        public string MenuPath { get; }

        public string DisplayName { get; }

        public int Priority { get; }

        public string Shortcut { get; }

        public bool IsAvailable { get; }

        public string DisabledReason { get; }

        public string Origin { get; }

        public string AssemblyName { get; }
    }

    public sealed class MenuCommandInvocationResult
    {
        public MenuCommandInvocationResult(
            bool succeeded,
            string menuPath,
            string toolId,
            string? agentSessionId,
            string refusalReason,
            IReadOnlyList<MenuAgentMutationRecord> mutations)
        {
            Succeeded = succeeded;
            MenuPath = menuPath;
            ToolId = toolId;
            AgentSessionId = agentSessionId;
            RefusalReason = refusalReason;
            Mutations = mutations;
        }

        public bool Succeeded { get; }

        public string MenuPath { get; }

        public string ToolId { get; }

        public string? AgentSessionId { get; }

        public string RefusalReason { get; }

        public IReadOnlyList<MenuAgentMutationRecord> Mutations { get; }
    }

    public sealed class MenuAgentMutationRecord
    {
        public MenuAgentMutationRecord(
            string agentSessionId,
            string commandPath,
            string targetIdentity,
            string operationKind,
            bool isUndoable,
            Action? undoCallback)
        {
            AgentSessionId = agentSessionId;
            CommandPath = commandPath;
            TargetIdentity = targetIdentity;
            OperationKind = operationKind;
            IsUndoable = isUndoable;
            UndoCallback = undoCallback;
        }

        public string AgentSessionId { get; }

        public string CommandPath { get; }

        public string TargetIdentity { get; }

        public string OperationKind { get; }

        public bool IsUndoable { get; }

        public bool WasUndone { get; private set; }

        internal Action? UndoCallback { get; }

        internal void MarkUndone()
        {
            WasUndone = true;
        }
    }
}
