using System.Globalization;
using UnityEngine;
using UnityEngine.UI;

namespace Unitas.Samples.RollABall
{
    public class HudController : MonoBehaviour
    {
        private ScoreManager? _manager;
        private Text? _scoreLabel;
        private Text? _comboLabel;
        private Text? _timerLabel;
        private Text? _resultsTitle;
        private Text? _resultsTime;
        private Text? _resultsScore;
        private Text? _pauseTitle;
        private Text? _pauseStatus;
        private Button? _playButton;
        private Button? _quitButton;
        private Button? _retryButton;
        private Button? _arenaSelectButton;
        private Button? _resumeButton;
        private GameObject? _titlePanel;
        private GameObject? _resultsPanel;
        private GameObject? _pausePanel;
        private Button? _focusedPauseButton;
        private AudioSource? _gameplayAudioSource;
        private AudioSource? _menuAudioSource;
        private AudioClip? _menuChimeClip;
        private bool _resultsShown;
        private bool _pauseOverlayShown;
        private bool _gameplayWasPlayingOnPause;
        private bool _nonResumeInputLogged;
        private float _pauseResumeTimeScale = 1f;

        private new void Awake()
        {
            _manager = GameObject.Find("PickupSensor")?.GetComponent<ScoreManager>();
            ResolveLabels();
            ResolveButtons();
            ResolvePanels();
            ResolveAudio();
            WireButtons();
            InitializePanels();
        }

        private void ResolveLabels()
        {
            _scoreLabel = FindText("ScoreLabel");
            _comboLabel = FindText("ComboLabel");
            _timerLabel = FindText("TimerLabel");
            _resultsTitle = FindText("ResultsTitle");
            _resultsTime = FindText("ResultsTime");
            _resultsScore = FindText("ResultsScore");
            _pauseTitle = FindText("PauseTitle");
            _pauseStatus = FindText("PauseStatus");
        }

        private void ResolveButtons()
        {
            _playButton = FindButton("PlayButton");
            _quitButton = FindButton("QuitButton");
            _retryButton = FindButton("RetryButton");
            _arenaSelectButton = FindButton("ArenaSelectButton");
            _resumeButton = FindButton("ResumeButton");
        }

        private void ResolvePanels()
        {
            _titlePanel = GameObject.Find("TitlePanel");
            _resultsPanel = GameObject.Find("ResultsPanel");
            _pausePanel = GameObject.Find("PausePanel");
        }

        private void ResolveAudio()
        {
            GameObject? chimeHolder = GameObject.Find("CollectChime");
            _gameplayAudioSource = chimeHolder != null ? chimeHolder.GetComponent<AudioSource>() : null;
            _menuChimeClip = _gameplayAudioSource != null ? _gameplayAudioSource.clip : null;
            _menuAudioSource = gameObject.AddComponent<AudioSource>();
            if (_menuAudioSource != null && _menuChimeClip != null)
            {
                _menuAudioSource.clip = _menuChimeClip;
            }
        }

        private void WireButtons()
        {
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
            if (_resumeButton != null)
            {
                _resumeButton.onClick.AddListener(OnResumeClicked);
            }
        }

        private void InitializePanels()
        {
            if (_resultsPanel != null)
            {
                _resultsPanel.SetActive(false);
            }
            if (_pausePanel != null)
            {
                _pausePanel.SetActive(false);
            }
        }

        private new void Update()
        {
            SyncPauseOverlayWithTimeScale();
            if (_pauseOverlayShown)
            {
                HandlePausedInput();
            }
            ScoreManager? manager = _manager;
            if (manager == null)
            {
                return;
            }
            UpdateHudLabels();
            if (manager.Won && !_resultsShown)
            {
                ShowResults();
            }
        }

        private void UpdateHudLabels()
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
        }

        private void SyncPauseOverlayWithTimeScale()
        {
            bool isTimePaused = Time.timeScale <= 0f;
            if (isTimePaused && !_pauseOverlayShown)
            {
                ShowPauseOverlay();
                return;
            }
            if (!isTimePaused && _pauseOverlayShown)
            {
                HidePauseOverlay(false);
            }
        }

        private void ShowPauseOverlay()
        {
            _pauseOverlayShown = true;
            _nonResumeInputLogged = false;
            if (Time.timeScale > 0f)
            {
                _pauseResumeTimeScale = Time.timeScale;
            }
            if (_pausePanel != null)
            {
                _pausePanel.SetActive(true);
            }
            if (_pauseTitle != null)
            {
                _pauseTitle.text = "PAUSED";
            }
            if (_pauseStatus != null)
            {
                _pauseStatus.text = "Focus ResumeButton";
            }
            FocusResumeButton("open");
            SilenceGameplayAudioAndPlayMenuChime();
            bool active = _pausePanel != null && _pausePanel.activeSelf;
            string focusName = _focusedPauseButton != null ? _focusedPauseButton.gameObject.name : "none";
            Debug.Log($"Pause overlay shown active={active} focus={focusName}");
        }

        private void FocusResumeButton(string source)
        {
            _focusedPauseButton = _resumeButton;
            if (_resumeButton != null)
            {
                _resumeButton.interactable = true;
            }
            if (_pauseStatus != null)
            {
                _pauseStatus.text = $"Focus ResumeButton ({source})";
            }
            string focusName = _focusedPauseButton != null ? _focusedPauseButton.gameObject.name : "none";
            Debug.Log($"Pause focus {focusName} source={source}");
        }

        private void SilenceGameplayAudioAndPlayMenuChime()
        {
            _gameplayWasPlayingOnPause = false;
            if (_gameplayAudioSource != null)
            {
                _gameplayWasPlayingOnPause = _gameplayAudioSource.state == AudioSourceState.Playing;
                _gameplayAudioSource.Pause();
                _gameplayAudioSource.mute = true;
            }
            PlayMenuSound("pause");
        }

        private void RestoreGameplayAudio()
        {
            if (_gameplayAudioSource == null)
            {
                return;
            }
            _gameplayAudioSource.mute = false;
            if (_gameplayWasPlayingOnPause)
            {
                _gameplayAudioSource.UnPause();
            }
            _gameplayWasPlayingOnPause = false;
        }

        private void PlayMenuSound(string cue)
        {
            if (_menuAudioSource != null && _menuChimeClip != null)
            {
                _menuAudioSource.PlayOneShot(_menuChimeClip);
            }
            AudioSourceState gameplayState = _gameplayAudioSource != null ? _gameplayAudioSource.state : AudioSourceState.Stopped;
            AudioSilenceReason gameplaySilence = _gameplayAudioSource != null ? _gameplayAudioSource.silenceReason : AudioSilenceReason.Stopped;
            AudioSourceState menuState = _menuAudioSource != null ? _menuAudioSource.state : AudioSourceState.Stopped;
            AudioSilenceReason menuSilence = _menuAudioSource != null ? _menuAudioSource.silenceReason : AudioSilenceReason.Stopped;
            Debug.Log($"Pause audio cue={cue} gameplay_state={gameplayState} gameplay_silence={gameplaySilence} menu_state={menuState} menu_silence={menuSilence}");
        }

        private void HandlePausedInput()
        {
            if (IsFocusNavigationPressed())
            {
                FocusResumeButton("navigation");
                PlayMenuSound("focus");
            }
            if (IsGamepadFocusNavigationPressed())
            {
                FocusResumeButton("gamepad");
                PlayMenuSound("focus");
            }
            if (IsNonResumeGameplayInputPressed())
            {
                Time.timeScale = 0f;
                if (!_nonResumeInputLogged)
                {
                    _nonResumeInputLogged = true;
                    bool active = _pausePanel != null && _pausePanel.activeSelf;
                    Debug.Log($"Pause held non-resume input ignored active={active} timeScale={FormatTime(Time.timeScale)}");
                }
            }
        }

        private static bool IsFocusNavigationPressed()
        {
            if (Input.GetKeyDown(KeyCode.Tab) || Input.GetKeyDown(KeyCode.UpArrow) || Input.GetKeyDown(KeyCode.DownArrow))
            {
                return true;
            }
            return Input.GetKeyDown(KeyCode.LeftArrow) || Input.GetKeyDown(KeyCode.RightArrow);
        }

        private static bool IsGamepadFocusNavigationPressed()
        {
            float vertical = Input.GetInjectedAxis("Vertical");
            float horizontal = Input.GetInjectedAxis("Horizontal");
            return Mathf.Abs(vertical) > 0.5f || Mathf.Abs(horizontal) > 0.5f;
        }

        private static bool IsNonResumeGameplayInputPressed()
        {
            if (Input.GetKeyDown(KeyCode.W) || Input.GetKeyDown(KeyCode.A) || Input.GetKeyDown(KeyCode.S) || Input.GetKeyDown(KeyCode.D))
            {
                return true;
            }
            return Input.GetKeyDown(KeyCode.Space);
        }

        private void HidePauseOverlay(bool logGameResumed)
        {
            _pauseOverlayShown = false;
            _focusedPauseButton = null;
            if (_pausePanel != null)
            {
                _pausePanel.SetActive(false);
            }
            RestoreGameplayAudio();
            if (Time.timeScale <= 0f)
            {
                Time.timeScale = _pauseResumeTimeScale > 0f ? _pauseResumeTimeScale : 1f;
            }
            bool active = _pausePanel != null && _pausePanel.activeSelf;
            Debug.Log($"Pause overlay hidden active={active}");
            if (logGameResumed)
            {
                Debug.Log("Game resumed");
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

        private void OnResumeClicked()
        {
            if (!_pauseOverlayShown)
            {
                return;
            }
            PlayMenuSound("resume");
            Debug.Log("Pause resume clicked");
            HidePauseOverlay(true);
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
