using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;

namespace Unitas.Editor.ShellProject
{
    public class WorkspaceShellController : MonoBehaviour
    {
        private const int HierarchyRowCount = 12;
        private const int PropertyRowCount = 8;

        private readonly Button?[] _hierarchyRowButtons = new Button?[HierarchyRowCount];
        private readonly Text?[] _hierarchyRowLabels = new Text?[HierarchyRowCount];
        private readonly Button?[] _propertyRowButtons = new Button?[PropertyRowCount];
        private readonly Text?[] _propertyRowLabels = new Text?[PropertyRowCount];

        private Text? _titleText;
        private Text? _playStatusText;
        private Button? _selectNextButton;
        private Button? _playToggleButton;
        private Button? _stepUpButton;
        private Button? _stepDownButton;
        private Button? _toggleValueButton;
        private Button? _createObjectButton;
        private Button? _duplicateObjectButton;
        private Button? _deleteObjectButton;
        private Button? _activeToggleButton;
        private Button? _undoButton;
        private Button? _redoButton;

        public Text? TitleText => _titleText;

        public Text? PlayStatusText => _playStatusText;

        public IReadOnlyList<Button?> HierarchyRowButtons => _hierarchyRowButtons;

        public IReadOnlyList<Text?> HierarchyRowLabels => _hierarchyRowLabels;

        public IReadOnlyList<Button?> PropertyRowButtons => _propertyRowButtons;

        public IReadOnlyList<Text?> PropertyRowLabels => _propertyRowLabels;

        public Button? SelectNextButton => _selectNextButton;

        public Button? PlayToggleButton => _playToggleButton;

        public Button? StepUpButton => _stepUpButton;

        public Button? StepDownButton => _stepDownButton;

        public Button? ToggleValueButton => _toggleValueButton;

        public Button? CreateObjectButton => _createObjectButton;

        public Button? DuplicateObjectButton => _duplicateObjectButton;

        public Button? DeleteObjectButton => _deleteObjectButton;

        public Button? ActiveToggleButton => _activeToggleButton;

        public Button? UndoButton => _undoButton;

        public Button? RedoButton => _redoButton;

        private new void Awake()
        {
            _titleText = FindText("WorkspaceTitleText");
            _playStatusText = FindText("PlayStatusText");
            _selectNextButton = FindButton("SelectNextButton");
            _playToggleButton = FindButton("PlayToggleButton");
            _stepUpButton = FindButton("StepUpButton");
            _stepDownButton = FindButton("StepDownButton");
            _toggleValueButton = FindButton("ToggleValueButton");
            _createObjectButton = FindButton("CreateObjectButton");
            _duplicateObjectButton = FindButton("DuplicateObjectButton");
            _deleteObjectButton = FindButton("DeleteObjectButton");
            _activeToggleButton = FindButton("ActiveToggleButton");
            _undoButton = FindButton("UndoButton");
            _redoButton = FindButton("RedoButton");
            DiscoverRows();
        }

        public void ApplyWorkspaceView(
            string title,
            IReadOnlyList<string> hierarchyRows,
            IReadOnlyList<string> propertyRows,
            string playStatus)
        {
            if (_titleText != null)
            {
                _titleText.text = title ?? string.Empty;
            }

            if (_playStatusText != null)
            {
                _playStatusText.text = playStatus ?? string.Empty;
            }

            ApplyRows(_hierarchyRowLabels, _hierarchyRowButtons, hierarchyRows);
            ApplyRows(_propertyRowLabels, _propertyRowButtons, propertyRows);
        }

        private void DiscoverRows()
        {
            for (int i = 0; i < HierarchyRowCount; i++)
            {
                _hierarchyRowButtons[i] = FindButton($"HierarchyRowButton{i}");
                _hierarchyRowLabels[i] = FindText($"HierarchyRowLabel{i}");
            }

            for (int i = 0; i < PropertyRowCount; i++)
            {
                _propertyRowButtons[i] = FindButton($"PropertyRowButton{i}");
                _propertyRowLabels[i] = FindText($"PropertyRowLabel{i}");
            }
        }

        private static void ApplyRows(
            Text?[] labels,
            Button?[] buttons,
            IReadOnlyList<string>? values)
        {
            int count = values?.Count ?? 0;
            for (int i = 0; i < labels.Length; i++)
            {
                bool active = i < count;
                string value = active && values != null ? values[i] ?? string.Empty : string.Empty;
                if (labels[i] != null)
                {
                    labels[i]!.text = value;
                }

                if (buttons[i] != null)
                {
                    buttons[i]!.interactable = active;
                }
            }
        }

        private static Text? FindText(string objectName)
        {
            GameObject? owner = GameObject.Find(objectName);
            return owner != null ? owner.GetComponent<Text>() : null;
        }

        private static Button? FindButton(string objectName)
        {
            GameObject? owner = GameObject.Find(objectName);
            return owner != null ? owner.GetComponent<Button>() : null;
        }
    }
}
