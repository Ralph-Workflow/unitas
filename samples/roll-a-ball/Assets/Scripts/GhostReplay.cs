using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Samples.RollABall
{
    /// <summary>
    /// Replays the route the arena's ghost file holds on a ghost object, so the
    /// next attempt runs against the route the previous attempt recorded. The
    /// replay walks the recorded positions one fixed step at a time and reports
    /// how far the live ghost strayed from the recorded route.
    /// </summary>
    public class GhostReplay : MonoBehaviour
    {
        private const float ReplayToleranceMetres = 0.01f;

        private readonly List<Vector3> _route = new List<Vector3>();
        private Vector3 _previous;
        private int _step;
        private float _maxDeviation;
        private bool _running;

        /// <summary>Starts the replay of the recorded route.</summary>
        public void Begin(IReadOnlyList<Vector3> route)
        {
            _route.Clear();
            for (int index = 0; index < route.Count; index++)
            {
                _route.Add(route[index]);
            }

            if (_route.Count == 0)
            {
                Debug.LogWarning("Ghost attempt replay failed the recorded route is empty");
                return;
            }

            _step = 0;
            _maxDeviation = 0f;
            _previous = _route[0];
            transform.position = _previous;
            _running = true;
            Debug.Log($"Ghost attempt replay started steps {_route.Count}");
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
