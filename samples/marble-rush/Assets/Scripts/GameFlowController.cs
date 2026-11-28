using UnityEngine;
using UnityEngine.Events;
using UnityEngine.SceneManagement;
using UnityEngine.UI;

namespace Unitas.Samples.MarbleRush
{
    /// <summary>
    /// Owns the title screen: the menu that starts a run, opens the settings
    /// panel and leaves the game, and the state of that panel.
    /// </summary>
    public class GameFlowController : MonoBehaviour
    {
        private const string ArenaScene = "Assets/Scenes/MarbleRush.unity";

        private Text? _panelHeader;

        private new void Awake()
        {
            _panelHeader = FindText("SettingsHeader");
            Wire("PlayButton", OnPlayClicked);
            Wire("SettingsButton", OnSettingsClicked);
            Wire("QuitButton", OnQuitClicked);
            Wire("SettingsCloseButton", OnSettingsClosed);
            Debug.Log("Title screen ready");
        }

        private void OnPlayClicked()
        {
            Debug.Log("Title play clicked");
            SceneManager.LoadScene(ArenaScene);
        }

        private void OnSettingsClicked()
        {
            Debug.Log("Settings panel opened");
            if (_panelHeader != null)
            {
                _panelHeader.text = "SETTINGS - press Close to go back";
            }
        }

        private void OnSettingsClosed()
        {
            Debug.Log("Settings panel closed");
            if (_panelHeader != null)
            {
                _panelHeader.text = "SETTINGS";
            }
        }

        private void OnQuitClicked()
        {
            Debug.Log("Title quit clicked");
        }

        private static void Wire(string name, UnityAction handler)
        {
            Button? button = GameObject.Find(name)?.GetComponent<Button>();
            if (button != null)
            {
                button.onClick.AddListener(handler);
            }
        }

        private static Text? FindText(string name)
        {
            GameObject? owner = GameObject.Find(name);
            return owner != null ? owner.GetComponent<Text>() : null;
        }
    }
}
