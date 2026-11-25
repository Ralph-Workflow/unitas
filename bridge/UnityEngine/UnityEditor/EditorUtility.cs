using System;
using System.Collections.Generic;
using UnityEngine;

namespace UnityEditor
{
    public static class EditorUtility
    {
        private static readonly object s_gate = new object();
        private static readonly HashSet<int> s_dirtyInstanceIds = new HashSet<int>();
        private static readonly Dictionary<string, string> s_dialogButtonOverridesByTitle =
            new Dictionary<string, string>(StringComparer.Ordinal);
        private static readonly Dictionary<string, bool> s_dialogBoolOverridesByTitle =
            new Dictionary<string, bool>(StringComparer.Ordinal);
        private static readonly List<EditorDialogEventRecord> s_dialogEvents =
            new List<EditorDialogEventRecord>();

        public static string? CurrentAgentSessionId { get; set; }

        public static IReadOnlyList<EditorDialogEventRecord> RecordedDialogEvents
        {
            get
            {
                lock (s_gate)
                {
                    return new List<EditorDialogEventRecord>(s_dialogEvents);
                }
            }
        }

        public static void ConfigureDialogAnswer(string title, bool chooseOk)
        {
            if (title == null)
            {
                throw new ArgumentNullException(nameof(title));
            }

            lock (s_gate)
            {
                s_dialogBoolOverridesByTitle[title] = chooseOk;
                s_dialogButtonOverridesByTitle.Remove(title);
            }
        }

        public static void ConfigureDialogAnswer(string title, string buttonLabel)
        {
            if (title == null)
            {
                throw new ArgumentNullException(nameof(title));
            }

            if (buttonLabel == null)
            {
                throw new ArgumentNullException(nameof(buttonLabel));
            }

            lock (s_gate)
            {
                s_dialogButtonOverridesByTitle[title] = buttonLabel;
                s_dialogBoolOverridesByTitle.Remove(title);
            }
        }

        public static void ClearDialogState()
        {
            lock (s_gate)
            {
                s_dialogButtonOverridesByTitle.Clear();
                s_dialogBoolOverridesByTitle.Clear();
                s_dialogEvents.Clear();
                s_dirtyInstanceIds.Clear();
                CurrentAgentSessionId = null;
            }
        }

        public static bool DisplayDialog(string title, string message, string ok)
        {
            return DisplayDialog(title, message, ok, string.Empty);
        }

        public static bool DisplayDialog(string title, string message, string ok, string cancel)
        {
            string safeTitle = title ?? string.Empty;
            string safeMessage = message ?? string.Empty;
            string safeOk = string.IsNullOrEmpty(ok) ? "OK" : ok;
            string safeCancel = cancel ?? string.Empty;

            bool chosenResult = true;
            string chosenButton = safeOk;
            bool usedSuppliedAnswer = false;
            string? sessionId;

            lock (s_gate)
            {
                sessionId = CurrentAgentSessionId ?? MenuItem.CurrentInvokingSessionId;
                if (s_dialogButtonOverridesByTitle.TryGetValue(safeTitle, out string? overrideButton))
                {
                    usedSuppliedAnswer = true;
                    if (!string.IsNullOrEmpty(safeCancel)
                        && (string.Equals(overrideButton, safeCancel, StringComparison.OrdinalIgnoreCase)
                            || string.Equals(overrideButton, "cancel", StringComparison.OrdinalIgnoreCase)))
                    {
                        chosenResult = false;
                        chosenButton = safeCancel;
                    }
                    else
                    {
                        chosenResult = true;
                        chosenButton = safeOk;
                    }
                }
                else if (s_dialogBoolOverridesByTitle.TryGetValue(safeTitle, out bool overrideBool))
                {
                    usedSuppliedAnswer = true;
                    if (!overrideBool && !string.IsNullOrEmpty(safeCancel))
                    {
                        chosenResult = false;
                        chosenButton = safeCancel;
                    }
                    else
                    {
                        chosenResult = overrideBool;
                        chosenButton = overrideBool ? safeOk : safeCancel;
                    }
                }

                List<string> buttons = new List<string> { safeOk };
                if (!string.IsNullOrEmpty(safeCancel))
                {
                    buttons.Add(safeCancel);
                }

                s_dialogEvents.Add(new EditorDialogEventRecord(
                    "DisplayDialog",
                    safeTitle,
                    safeMessage,
                    buttons,
                    chosenButton,
                    chosenResult ? 0 : 1,
                    chosenResult,
                    usedSuppliedAnswer,
                    sessionId));
            }

            return chosenResult;
        }

        public static int DisplayDialogComplex(string title, string message, string ok, string cancel, string alt)
        {
            string safeTitle = title ?? string.Empty;
            string safeMessage = message ?? string.Empty;
            string safeOk = string.IsNullOrEmpty(ok) ? "OK" : ok;
            string safeCancel = cancel ?? string.Empty;
            string safeAlt = alt ?? string.Empty;

            int chosenIndex = 0;
            string chosenButton = safeOk;
            bool usedSuppliedAnswer = false;
            string? sessionId;

            lock (s_gate)
            {
                sessionId = CurrentAgentSessionId ?? MenuItem.CurrentInvokingSessionId;
                if (s_dialogButtonOverridesByTitle.TryGetValue(safeTitle, out string? overrideButton))
                {
                    usedSuppliedAnswer = true;
                    if (string.Equals(overrideButton, safeCancel, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(overrideButton, "cancel", StringComparison.OrdinalIgnoreCase))
                    {
                        chosenIndex = 1;
                    }
                    else if (string.Equals(overrideButton, safeAlt, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(overrideButton, "alt", StringComparison.OrdinalIgnoreCase))
                    {
                        chosenIndex = 2;
                    }
                    else
                    {
                        chosenIndex = 0;
                    }
                }

                chosenButton = chosenIndex switch
                {
                    1 => safeCancel,
                    2 => safeAlt,
                    _ => safeOk
                };

                List<string> buttons = new List<string> { safeOk, safeCancel, safeAlt };
                s_dialogEvents.Add(new EditorDialogEventRecord(
                    "DisplayDialogComplex",
                    safeTitle,
                    safeMessage,
                    buttons,
                    chosenButton,
                    chosenIndex,
                    chosenIndex == 0,
                    usedSuppliedAnswer,
                    sessionId));
            }

            return chosenIndex;
        }

        public static void SetDirty(UnityEngine.Object target)
        {
            if (target == null)
            {
                throw new ArgumentNullException(nameof(target));
            }

            int instanceId = target.GetInstanceID();
            string targetName = target.name;
            bool wasAlreadyDirty;
            lock (s_gate)
            {
                wasAlreadyDirty = !s_dirtyInstanceIds.Add(instanceId);
            }

            MenuItem.RecordAttributedMutation(
                "Object:" + targetName + "#" + instanceId,
                "EditorUtility.SetDirty",
                () =>
                {
                    if (!wasAlreadyDirty)
                    {
                        lock (s_gate)
                        {
                            s_dirtyInstanceIds.Remove(instanceId);
                        }
                    }
                });
        }

        public static bool IsDirty(UnityEngine.Object target)
        {
            if (target == null)
            {
                throw new ArgumentNullException(nameof(target));
            }

            lock (s_gate)
            {
                return s_dirtyInstanceIds.Contains(target.GetInstanceID());
            }
        }
    }

    public sealed class EditorDialogEventRecord
    {
        public EditorDialogEventRecord(
            string dialogKind,
            string title,
            string message,
            IReadOnlyList<string> buttons,
            string chosenButton,
            int chosenIndex,
            bool booleanResult,
            bool usedSuppliedAnswer,
            string? sessionId)
        {
            DialogKind = dialogKind;
            Title = title;
            Message = message;
            Buttons = buttons;
            ChosenButton = chosenButton;
            ChosenIndex = chosenIndex;
            BooleanResult = booleanResult;
            UsedSuppliedAnswer = usedSuppliedAnswer;
            SessionId = sessionId;
        }

        public string DialogKind { get; }

        public string Title { get; }

        public string Message { get; }

        public IReadOnlyList<string> Buttons { get; }

        public string ChosenButton { get; }

        public int ChosenIndex { get; }

        public bool BooleanResult { get; }

        public bool UsedSuppliedAnswer { get; }

        public string? SessionId { get; }
    }
}
