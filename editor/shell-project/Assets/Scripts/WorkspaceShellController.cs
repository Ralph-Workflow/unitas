using UnityEngine;
using UnityEngine.UI;

namespace Unitas.Editor.ShellProject
{
    public class WorkspaceShellController : MonoBehaviour
    {
        private Text? _titleText;
        private Text? _hierarchyListText;
        private Text? _inspectorPanelText;
        private Text? _playStatusText;
        private Button? _selectNextButton;
        private Button? _playToggleButton;

        public Text? TitleText => _titleText;

        public Text? HierarchyListText => _hierarchyListText;

        public Text? InspectorPanelText => _inspectorPanelText;

        public Text? PlayStatusText => _playStatusText;

        public Button? SelectNextButton => _selectNextButton;

        public Button? PlayToggleButton => _playToggleButton;

        private new void Awake()
        {
            _titleText = FindText("WorkspaceTitleText");
            _hierarchyListText = FindText("HierarchyListText");
            _inspectorPanelText = FindText("InspectorPanelText");
            _playStatusText = FindText("PlayStatusText");
            _selectNextButton = FindButton("SelectNextButton");
            _playToggleButton = FindButton("PlayToggleButton");
        }

        public void ApplyWorkspaceView(
            string title,
            string hierarchy,
            string inspector,
            string playStatus)
        {
            if (_titleText != null)
            {
                _titleText.text = title ?? string.Empty;
            }

            if (_hierarchyListText != null)
            {
                _hierarchyListText.text = hierarchy ?? string.Empty;
            }

            if (_inspectorPanelText != null)
            {
                _inspectorPanelText.text = inspector ?? string.Empty;
            }

            if (_playStatusText != null)
            {
                _playStatusText.text = playStatus ?? string.Empty;
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
