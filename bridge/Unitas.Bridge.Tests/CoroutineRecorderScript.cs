using System.Collections;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class CoroutineRecorderScript : MonoBehaviour
    {
        private readonly Collection<string> _events;
        private readonly string _prefix;
        private readonly bool _startAllOnStart;

        public CoroutineRecorderScript()
            : this(new Collection<string>(), string.Empty, true)
        {
        }

        public CoroutineRecorderScript(Collection<string> sharedEvents, string prefix, bool startAllOnStart)
        {
            _events = sharedEvents ?? new Collection<string>();
            _prefix = prefix;
            _startAllOnStart = startAllOnStart;
        }

        public IReadOnlyList<string> Events => _events;

        public string ObservedOrder => string.Join(", ", _events);

        public void Record(string tag)
        {
            if (string.IsNullOrEmpty(_prefix))
            {
                _events.Add(tag);
            }
            else
            {
                _events.Add(_prefix + ":" + tag);
            }
        }

        public override void Awake()
        {
            Record("Awake");
        }

        public override void OnEnable()
        {
            Record("OnEnable");
        }

        public override void Start()
        {
            Record("Start");
            if (_startAllOnStart)
            {
                StartCoroutine(NextFrameRoutine());
                StartCoroutine(FixedUpdateRoutine());
                StartCoroutine(EndOfFrameRoutine());
            }
        }

        public override void FixedUpdate()
        {
            Record("FixedUpdate");
        }

        public override void Update()
        {
            Record("Update");
        }

        public override void LateUpdate()
        {
            Record("LateUpdate");
        }

        public override void OnDisable()
        {
            Record("OnDisable");
        }

        public override void OnDestroy()
        {
            Record("OnDestroy");
        }

        public void StartFixedLoop()
        {
            StartCoroutine(FixedUpdateRoutine());
        }

        private IEnumerator NextFrameRoutine()
        {
            Record("NextFrame:Init");
            int step = 0;
            while (true)
            {
                yield return null;
                step++;
                Record("NextFrame:" + step.ToString(System.Globalization.CultureInfo.InvariantCulture));
            }
        }

        private IEnumerator FixedUpdateRoutine()
        {
            Record("FixedWait:Init");
            int step = 0;
            while (true)
            {
                yield return new WaitForFixedUpdate();
                step++;
                Record("FixedWait:" + step.ToString(System.Globalization.CultureInfo.InvariantCulture));
            }
        }

        private IEnumerator EndOfFrameRoutine()
        {
            Record("EndOfFrame:Init");
            int step = 0;
            while (true)
            {
                yield return new WaitForEndOfFrame();
                step++;
                Record("EndOfFrame:" + step.ToString(System.Globalization.CultureInfo.InvariantCulture));
            }
        }
    }
}
