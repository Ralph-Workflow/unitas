using System.Globalization;
using UnityEngine;
using UnityEngine.UI;

namespace Unitas.Samples.RollABall
{
    // The HUD drives the title screen, the score readout and the results
    // panel. The scene carries no serialized object references, so every
    // piece is resolved by name in Awake.
    public class HudController : MonoBehaviour
    {
        private ScoreManager? _manager;
        private Text? _scoreLabel;
        private Text? _comboLabel;
        private Text? _timerLabel;
        private Text? _resultsTitle;
        private Text? _resultsTime;
        private Text? _resultsScore;
        private Button? _playButton;
        private Button? _quitButton;
        private Button? _retryButton;
        private Button? _arenaSelectButton;
        private GameObject? _titlePanel;
        private GameObject? _resultsPanel;
        private bool _resultsShown;

        private new void Awake()
        {
            _manager = GameObject.Find("ScoreManager")?.GetComponent<ScoreManager>();
            _scoreLabel = FindText("ScoreLabel");
            _comboLabel = FindText("ComboLabel");
            _timerLabel = FindText("TimerLabel");
            _resultsTitle = FindText("ResultsTitle");
            _resultsTime = FindText("ResultsTime");
            _resultsScore = FindText("ResultsScore");
            _playButton = FindButton("PlayButton");
            _quitButton = FindButton("QuitButton");
            _retryButton = FindButton("RetryButton");
            _arenaSelectButton = FindButton("ArenaSelectButton");
            _titlePanel = GameObject.Find("TitlePanel");
            _resultsPanel = GameObject.Find("ResultsPanel");
            if (_playButton != null)
            {
                _playButton.onClick.AddListener(OnPlayClicked);
            }
            if (_quitButton != null)
            {
                _quitButton.onClick.AddListener(OnQuitClicked);
            }
            if (_retryButton != null)
            {
                _retryButton.onClick.AddListener(OnRetryClicked);
            }
            if (_arenaSelectButton != null)
            {
                _arenaSelectButton.onClick.AddListener(OnArenaSelectClicked);
            }
            // The results panel stays hidden until the run is won.
            if (_resultsPanel != null)
            {
                _resultsPanel.SetActive(false);
            }
        }

        private new void Update()
        {
            ScoreManager? manager = _manager;
            if (manager == null)
            {
                return;
            }
            if (_scoreLabel != null)
            {
                _scoreLabel.text = $"Score {manager.Score}";
            }
            if (_comboLabel != null)
            {
                _comboLabel.text = $"Combo x{manager.Multiplier}";
            }
            if (_timerLabel != null)
            {
                _timerLabel.text = $"Time {FormatTime(manager.RunTime)}";
            }
            if (manager.Won && !_resultsShown)
            {
                ShowResults();
            }
        }

        private void ShowResults()
        {
            ScoreManager? manager = _manager;
            if (manager == null)
            {
                return;
            }

            _resultsShown = true;
            if (_resultsPanel != null)
            {
                _resultsPanel.SetActive(true);
            }
            if (_resultsTitle != null)
            {
                _resultsTitle.text = "RESULTS";
            }
            if (_resultsTime != null)
            {
                _resultsTime.text = $"Time {FormatTime(manager.RunTime)}";
            }
            if (_resultsScore != null)
            {
                _resultsScore.text = $"Score {manager.Score}";
            }
            if (_titlePanel != null)
            {
                _titlePanel.SetActive(false);
            }
            Debug.Log("Results panel shown");
        }

        private void OnPlayClicked()
        {
            Debug.Log("Title play clicked");
            if (_titlePanel != null)
            {
                _titlePanel.SetActive(false);
            }
            if (_manager != null)
            {
                _manager.StartRun();
            }
        }

        private void OnQuitClicked()
        {
            Debug.Log("Title quit clicked");
        }

        private void OnRetryClicked()
        {
            Debug.Log("Results retry clicked");
        }

        private void OnArenaSelectClicked()
        {
            Debug.Log("Results arena select clicked");
        }

        private static Text? FindText(string name)
        {
            GameObject? owner = GameObject.Find(name);
            return owner != null ? owner.GetComponent<Text>() : null;
        }

        private static Button? FindButton(string name)
        {
            GameObject? owner = GameObject.Find(name);
            return owner != null ? owner.GetComponent<Button>() : null;
        }

        private static string FormatTime(float seconds)
        {
            return seconds.ToString("0.0", CultureInfo.InvariantCulture);
        }
    }
}
