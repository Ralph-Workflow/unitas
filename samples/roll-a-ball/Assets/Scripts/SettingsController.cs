using System;
using System.Globalization;
using UnityEngine;
using UnityEngine.Events;
using UnityEngine.UI;

namespace Unitas.Samples.RollABall
{
    public class SettingsController : MonoBehaviour
    {
        private const string MasterVolumeKey = "settings.master_volume";
        private const string MusicVolumeKey = "settings.music_volume";
        private const string EffectsVolumeKey = "settings.effects_volume";
        private const string FullScreenKey = "settings.fullscreen";
        private const string ResolutionKey = "settings.resolution";
        private const string VerticalSyncKey = "settings.vsync";
        private const string QualityKey = "settings.quality";
        private const string GhostKey = "settings.ghost";
        private const string CameraShakeKey = "settings.camera_shake";
        private const string ReducedMotionKey = "reduced_motion";

        private const float HalfVolume = 0.5f;
        private const float FullVolume = 1f;
        private const float ShakeSeconds = 0.25f;
        private const float ShakeMetres = 0.05f;

        private static readonly string[] ResolutionNames = { "1280x720", "1920x1080" };
        private static readonly int[] ResolutionWidths = { 1280, 1920 };
        private static readonly int[] ResolutionHeights = { 720, 1080 };
        private static readonly string[] QualityNames = { "low", "medium", "high" };

        private Text? _volumeSummary;
        private Text? _modeSummary;
        private Transform? _camera;
        private Vector3 _cameraHome;
        private float _shakeTime;

        public static bool GhostEnabled => PlayerPrefs.GetInt(GhostKey, 1) != 0;

        public static bool CameraShakeEnabled => PlayerPrefs.GetInt(CameraShakeKey, 1) != 0;

        public static bool IsCameraShakeActive => CameraShakeEnabled && !ReducedMotion.IsEnabled;

        private new void Awake()
        {
            _volumeSummary = FindText("SettingsSummaryVolumes");
            _modeSummary = FindText("SettingsSummaryModes");
            GameObject? camera = GameObject.Find("Main Camera");
            if (camera != null)
            {
                _camera = camera.transform;
                _cameraHome = _camera.position;
            }

            Wire("ReducedMotionQuickButton", OnReducedMotionQuick);
            Wire("ReducedMotionOnButton", OnReducedMotionOn);
            Wire("ReducedMotionOffButton", OnReducedMotionOff);
            Wire("MasterVolumeHalfButton", OnMasterVolumeHalf);
            Wire("MasterVolumeFullButton", OnMasterVolumeFull);
            Wire("MusicVolumeHalfButton", OnMusicVolumeHalf);
            Wire("MusicVolumeFullButton", OnMusicVolumeFull);
            Wire("EffectsVolumeHalfButton", OnEffectsVolumeHalf);
            Wire("EffectsVolumeFullButton", OnEffectsVolumeFull);
            Wire("FullScreenButton", OnFullScreen);
            Wire("WindowedButton", OnWindowed);
            Wire("Resolution720Button", OnResolution720);
            Wire("Resolution1080Button", OnResolution1080);
            Wire("VerticalSyncOnButton", OnVerticalSyncOn);
            Wire("VerticalSyncOffButton", OnVerticalSyncOff);
            Wire("QualityLowButton", OnQualityLow);
            Wire("QualityMediumButton", OnQualityMedium);
            Wire("QualityHighButton", OnQualityHigh);
            Wire("GhostOnButton", OnGhostOn);
            Wire("GhostOffButton", OnGhostOff);
            Wire("CameraShakeOnButton", OnCameraShakeOn);
            Wire("CameraShakeOffButton", OnCameraShakeOff);

            ApplySettings();
            Debug.Log("Settings restored " + Describe());
        }

        private new void Update()
        {
            if (_shakeTime <= 0f)
            {
                return;
            }

            _shakeTime -= Time.deltaTime;
            Transform? camera = _camera;
            if (camera == null)
            {
                return;
            }

            float offset = IsCameraShakeActive ? ShakeMetres : 0f;
            camera.position = _cameraHome + new Vector3(offset, offset * 0.5f, 0f);
            if (_shakeTime <= 0f)
            {
                camera.position = _cameraHome;
            }
        }

        private void OnReducedMotionQuick()
        {
            SetReducedMotion(1);
        }

        private void OnReducedMotionOn()
        {
            SetReducedMotion(1);
        }

        private void OnReducedMotionOff()
        {
            SetReducedMotion(0);
        }

        private void OnMasterVolumeHalf()
        {
            SetVolume("master_volume", MasterVolumeKey, HalfVolume);
        }

        private void OnMasterVolumeFull()
        {
            SetVolume("master_volume", MasterVolumeKey, FullVolume);
        }

        private void OnMusicVolumeHalf()
        {
            SetVolume("music_volume", MusicVolumeKey, HalfVolume);
        }

        private void OnMusicVolumeFull()
        {
            SetVolume("music_volume", MusicVolumeKey, FullVolume);
        }

        private void OnEffectsVolumeHalf()
        {
            SetVolume("effects_volume", EffectsVolumeKey, HalfVolume);
        }

        private void OnEffectsVolumeFull()
        {
            SetVolume("effects_volume", EffectsVolumeKey, FullVolume);
        }

        private void OnFullScreen()
        {
            SetFullScreen(1);
        }

        private void OnWindowed()
        {
            SetFullScreen(0);
        }

        private void OnResolution720()
        {
            SetResolution(0);
        }

        private void OnResolution1080()
        {
            SetResolution(1);
        }

        private void OnVerticalSyncOn()
        {
            SetVerticalSync(1);
        }

        private void OnVerticalSyncOff()
        {
            SetVerticalSync(0);
        }

        private void OnQualityLow()
        {
            SetQuality(0);
        }

        private void OnQualityMedium()
        {
            SetQuality(1);
        }

        private void OnQualityHigh()
        {
            SetQuality(2);
        }

        private void OnGhostOn()
        {
            SetGhost(1);
        }

        private void OnGhostOff()
        {
            SetGhost(0);
        }

        private void OnCameraShakeOn()
        {
            SetCameraShake(1);
        }

        private void OnCameraShakeOff()
        {
            SetCameraShake(0);
        }

        private void SetReducedMotion(int value)
        {
            PlayerPrefs.SetInt(ReducedMotionKey, value);
            Save();
            ApplySettings();
            Debug.Log("Settings reduced_motion " + value.ToString(CultureInfo.InvariantCulture));
        }

        private void SetVolume(string name, string key, float value)
        {
            PlayerPrefs.SetFloat(key, value);
            Save();
            ApplySettings();
            Debug.Log("Settings " + name + " " + FormatVolume(value));
        }

        private void SetFullScreen(int value)
        {
            PlayerPrefs.SetInt(FullScreenKey, value);
            Save();
            ApplySettings();
            Debug.Log("Settings fullscreen " + value.ToString(CultureInfo.InvariantCulture));
        }

        private void SetResolution(int index)
        {
            PlayerPrefs.SetInt(ResolutionKey, index);
            Save();
            ApplySettings();
            Debug.Log("Settings resolution " + ResolutionName(index));
        }

        private void SetVerticalSync(int value)
        {
            PlayerPrefs.SetInt(VerticalSyncKey, value);
            Save();
            ApplySettings();
            Debug.Log("Settings vsync " + value.ToString(CultureInfo.InvariantCulture));
        }

        private void SetQuality(int index)
        {
            PlayerPrefs.SetString(QualityKey, QualityName(index));
            Save();
            ApplySettings();
            Debug.Log("Settings quality " + QualityName(index));
        }

        private void SetGhost(int value)
        {
            PlayerPrefs.SetInt(GhostKey, value);
            Save();
            ApplySettings();
            Debug.Log("Settings ghost " + value.ToString(CultureInfo.InvariantCulture));
        }

        private void SetCameraShake(int value)
        {
            PlayerPrefs.SetInt(CameraShakeKey, value);
            Save();
            ApplySettings();
            Debug.Log("Settings camera_shake " + value.ToString(CultureInfo.InvariantCulture));
        }

        private void ApplySettings()
        {
            float master = PlayerPrefs.GetFloat(MasterVolumeKey, FullVolume);
            float music = PlayerPrefs.GetFloat(MusicVolumeKey, FullVolume);
            float effects = PlayerPrefs.GetFloat(EffectsVolumeKey, FullVolume);
            AudioListener.volume = master;
            AudioSource[] sources = UnityEngine.Object.FindObjectsOfType<AudioSource>();
            for (int index = 0; index < sources.Length; index++)
            {
                AudioSource? source = sources[index];
                if (source == null)
                {
                    continue;
                }

                source.volume = IsMusic(source) ? music : effects;
            }

            int fullScreen = PlayerPrefs.GetInt(FullScreenKey, 0);
            int resolution = PlayerPrefs.GetInt(ResolutionKey, 0);
            Screen.fullScreen = fullScreen != 0;
            Screen.SetResolution(ResolutionWidth(resolution), ResolutionHeight(resolution), fullScreen != 0);
            QualitySettings.vSyncCount = PlayerPrefs.GetInt(VerticalSyncKey, 1);
            QualitySettings.SetQualityLevel(QualityIndex());
            GhostReplay[] ghosts = UnityEngine.Object.FindObjectsOfType<GhostReplay>();
            bool ghostEnabled = GhostEnabled;
            for (int index = 0; index < ghosts.Length; index++)
            {
                GhostReplay? ghost = ghosts[index];
                if (ghost != null)
                {
                    ghost.enabled = ghostEnabled;
                }
            }

            UpdateSummaries(master, music, effects, fullScreen, resolution);
            _shakeTime = ShakeSeconds;
        }

        private void UpdateSummaries(float master, float music, float effects, int fullScreen, int resolution)
        {
            if (_volumeSummary != null)
            {
                _volumeSummary.text = "Master " + FormatVolume(master) + "  Music " + FormatVolume(music) + "  Effects " + FormatVolume(effects);
            }

            if (_modeSummary != null)
            {
                _modeSummary.text = (fullScreen != 0 ? "Fullscreen" : "Windowed")
                    + "  " + ResolutionName(resolution)
                    + "  VSync " + (PlayerPrefs.GetInt(VerticalSyncKey, 1) != 0 ? "On" : "Off")
                    + "  Quality " + QualityName(QualityIndex())
                    + "  Ghost " + (GhostEnabled ? "On" : "Off")
                    + "  Shake " + (CameraShakeEnabled ? "On" : "Off")
                    + "  Reduced Motion " + (ReducedMotion.IsEnabled ? "On" : "Off");
            }
        }

        private string Describe()
        {
            return "master_volume " + FormatVolume(AudioListener.volume)
                + " music_volume " + FormatVolume(PlayerPrefs.GetFloat(MusicVolumeKey, FullVolume))
                + " effects_volume " + FormatVolume(PlayerPrefs.GetFloat(EffectsVolumeKey, FullVolume))
                + " fullscreen " + (Screen.fullScreen ? "1" : "0")
                + " resolution " + Screen.width.ToString(CultureInfo.InvariantCulture) + "x" + Screen.height.ToString(CultureInfo.InvariantCulture)
                + " vsync " + QualitySettings.vSyncCount.ToString(CultureInfo.InvariantCulture)
                + " quality " + QualityName(QualitySettings.GetQualityLevel())
                + " ghost " + (GhostEnabled ? "1" : "0")
                + " camera_shake " + (CameraShakeEnabled ? "1" : "0")
                + " reduced_motion " + (ReducedMotion.IsEnabled ? "1" : "0");
        }

        private static int QualityIndex()
        {
            string stored = PlayerPrefs.GetString(QualityKey, "high");
            for (int index = 0; index < QualityNames.Length; index++)
            {
                if (string.Equals(QualityNames[index], stored, StringComparison.Ordinal))
                {
                    return index;
                }
            }

            return QualityNames.Length - 1;
        }

        private static string QualityName(int index)
        {
            if (index < 0 || index >= QualityNames.Length)
            {
                return QualityNames[QualityNames.Length - 1];
            }

            return QualityNames[index];
        }

        private static string ResolutionName(int index)
        {
            if (index < 0 || index >= ResolutionNames.Length)
            {
                return ResolutionNames[0];
            }

            return ResolutionNames[index];
        }

        private static int ResolutionWidth(int index)
        {
            if (index < 0 || index >= ResolutionWidths.Length)
            {
                return ResolutionWidths[0];
            }

            return ResolutionWidths[index];
        }

        private static int ResolutionHeight(int index)
        {
            if (index < 0 || index >= ResolutionHeights.Length)
            {
                return ResolutionHeights[0];
            }

            return ResolutionHeights[index];
        }

        private static bool IsMusic(AudioSource source)
        {
            return source.name.StartsWith("Music", StringComparison.Ordinal);
        }

        private static string FormatVolume(float value)
        {
            return value.ToString("0.0", CultureInfo.InvariantCulture);
        }

        private static void Save()
        {
            PlayerPrefs.Save();
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
