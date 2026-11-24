using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    /// <summary>
    /// Ordered log shared by the scene recorders and the scene lifecycle events so the integration tests can
    /// assert the relative order of teardown callbacks and the sceneUnloaded event.
    /// </summary>
    public static class SceneIntegrationEventLog
    {
        public static readonly List<string> Entries = new List<string>();

        public static void Clear()
        {
            Entries.Clear();
        }

        public static void Record(string entry)
        {
            Entries.Add(entry);
        }

        public static int IndexOf(string entry)
        {
            for (int i = 0; i < Entries.Count; i++)
            {
                if (Entries[i] == entry)
                {
                    return i;
                }
            }

            return -1;
        }
    }

    public class AlphaSceneRecorder : MonoBehaviour
    {
        public int AwakeCount;

        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public int DisableCount;

        public int DestroyCount;

        public override void Awake()
        {
            AwakeCount++;
            SceneIntegrationEventLog.Record("alpha:Awake");
        }

        public override void OnEnable()
        {
            EnableCount++;
            SceneIntegrationEventLog.Record("alpha:OnEnable");
        }

        public override void Start()
        {
            StartCount++;
            SceneIntegrationEventLog.Record("alpha:Start");
        }

        public override void Update()
        {
            UpdateCount++;
        }

        public override void OnDisable()
        {
            DisableCount++;
            SceneIntegrationEventLog.Record("alpha:OnDisable");
        }

        public override void OnDestroy()
        {
            DestroyCount++;
            SceneIntegrationEventLog.Record("alpha:OnDestroy");
        }
    }

    public class BetaSceneRecorder : MonoBehaviour
    {
        public int AwakeCount;

        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public int DisableCount;

        public int DestroyCount;

        public override void Awake()
        {
            AwakeCount++;
            SceneIntegrationEventLog.Record("beta:Awake");
        }

        public override void OnEnable()
        {
            EnableCount++;
            SceneIntegrationEventLog.Record("beta:OnEnable");
        }

        public override void Start()
        {
            StartCount++;
            SceneIntegrationEventLog.Record("beta:Start");
        }

        public override void Update()
        {
            UpdateCount++;
        }

        public override void OnDisable()
        {
            DisableCount++;
            SceneIntegrationEventLog.Record("beta:OnDisable");
        }

        public override void OnDestroy()
        {
            DestroyCount++;
            SceneIntegrationEventLog.Record("beta:OnDestroy");
        }
    }

    public sealed class PersistentSceneIntegrationRecorder : MonoBehaviour
    {
        public const string MarkerText = "ddol-marker";

        public string Marker = string.Empty;

        public int GameObjectInstanceId;

        public int UpdateCount;

        public int DisableCount;

        public int DestroyCount;

        public bool MarkSelfPersistent;

        public override void Awake()
        {
            Marker = MarkerText;
            GameObjectInstanceId = gameObject.GetInstanceID();
            UnityEngine.Object.DontDestroyOnLoad(MarkSelfPersistent ? (UnityEngine.Object)this : gameObject);
            SceneIntegrationEventLog.Record("ddol:Awake");
        }

        public override void Update()
        {
            UpdateCount++;
        }

        public override void OnDisable()
        {
            DisableCount++;
            SceneIntegrationEventLog.Record("ddol:OnDisable");
        }

        public override void OnDestroy()
        {
            DestroyCount++;
            SceneIntegrationEventLog.Record("ddol:OnDestroy");
        }
    }
}
