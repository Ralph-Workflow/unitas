using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Samples.MarbleRush
{
    public class GhostReplay : MonoBehaviour
    {
        private const float ReplayToleranceMetres = 0.01f;

        private static readonly Vector3[] DefaultAttractRoute =
        {
            new Vector3(0f, 0.5f, 0f),
            new Vector3(0f, 0.5f, 0.75f),
            new Vector3(0f, 0.5f, 1.5f),
            new Vector3(0f, 0.5f, 2.25f),
            new Vector3(-0.75f, 0.5f, 3f),
            new Vector3(-1.5f, 0.5f, 3.75f),
            new Vector3(-2.25f, 0.5f, 4.5f),
            new Vector3(-3f, 0.5f, 3.5f),
            new Vector3(-3.75f, 0.5f, 2.5f),
            new Vector3(-4.5f, 0.5f, 1.5f),
            new Vector3(-4.5f, 0.5f, 0f),
            new Vector3(-4.5f, 0.5f, -1.5f),
            new Vector3(-4.5f, 0.5f, -3f),
            new Vector3(-4.5f, 0.5f, -4.5f),
            new Vector3(-3f, 0.5f, -4.5f),
            new Vector3(-1.5f, 0.5f, -4.5f),
            new Vector3(0f, 0.5f, -4.5f),
            new Vector3(1.5f, 0.5f, -4.5f),
            new Vector3(3f, 0.5f, -4.5f),
            new Vector3(4.5f, 0.5f, -4.5f),
            new Vector3(4.5f, 0.5f, -3f),
            new Vector3(4.5f, 0.5f, -1.5f),
            new Vector3(4.5f, 0.5f, 0f),
            new Vector3(4.5f, 0.5f, 1.5f),
            new Vector3(4.5f, 0.5f, 3f),
            new Vector3(4.5f, 0.5f, 4.5f),
            new Vector3(3f, 0.5f, 4.5f),
            new Vector3(1.5f, 0.5f, 4.5f),
            new Vector3(0.75f, 0.5f, 2.25f),
        };

        [SerializeField]
        private int attractMode;

        private readonly List<Vector3> _route = new List<Vector3>();
        private Vector3 _previous;
        private int _step;
        private int _loopCount;
        private float _maxDeviation;
        private bool _running;

        private new void Start()
        {
            if (!_running && (attractMode != 0 || string.Equals(gameObject.name, "AttractGhost", System.StringComparison.Ordinal)))
            {
                BeginAttract();
            }
        }

        public void BeginAttract()
        {
            attractMode = 1;
            Vector3[]? persisted = SavedState.LoadGhostRoute();
            IReadOnlyList<Vector3> source = persisted != null && persisted.Length > 0
                ? persisted
                : DefaultAttractRoute;
            string sourceKind = persisted != null && persisted.Length > 0 ? "persisted" : "default";
            LoadRoute(source);
            _loopCount = 0;
            Debug.Log($"Attract ghost replay started source {sourceKind} steps {_route.Count}");
        }

        public void Begin(IReadOnlyList<Vector3> route)
        {
            attractMode = 0;
            LoadRoute(route);
            if (!_running)
            {
                Debug.LogWarning("Ghost attempt replay failed the recorded route is empty");
                return;
            }

            Debug.Log($"Ghost attempt replay started steps {_route.Count}");
        }

        private void LoadRoute(IReadOnlyList<Vector3> route)
        {
            _route.Clear();
            for (int index = 0; index < route.Count; index++)
            {
                _route.Add(route[index]);
            }

            if (_route.Count == 0)
            {
                _running = false;
                return;
            }

            _step = 0;
            _maxDeviation = 0f;
            _previous = _route[0];
            transform.position = _previous;
            _running = true;
        }

        private new void FixedUpdate()
        {
            if (!_running)
            {
                return;
            }

            _step++;
            if (_step >= _route.Count)
            {
                if (attractMode != 0)
                {
                    _loopCount++;
                    _step = 0;
                    _previous = _route[0];
                    transform.position = _previous;
                    Debug.Log($"Attract ghost replay looped {_loopCount} steps {_route.Count} max deviation {FormatSeconds(_maxDeviation)} m");
                    _maxDeviation = 0f;
                    return;
                }

                _running = false;
                Debug.Log($"Ghost attempt replay max deviation {FormatSeconds(_maxDeviation)} m over {_route.Count} steps");
                if (_maxDeviation <= ReplayToleranceMetres)
                {
                    Debug.Log("Ghost attempt replay within 0.01 m");
                }
                else
                {
                    Debug.LogWarning("Ghost attempt replay failed the ghost left the recorded route");
                }

                return;
            }

            Vector3 target = _route[_step];
            transform.position = transform.position + (target - _previous);
            _previous = target;
            float deviation = (transform.position - target).magnitude;
            if (deviation > _maxDeviation)
            {
                _maxDeviation = deviation;
            }
        }

        private static string FormatSeconds(float seconds)
        {
            return seconds.ToString("F3", CultureInfo.InvariantCulture);
        }
    }
}
