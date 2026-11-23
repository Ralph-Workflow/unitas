using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class LifecycleScript : MonoBehaviour
    {
        private readonly System.Collections.Generic.List<string> _events = new System.Collections.Generic.List<string>();

        public string ObservedOrder => string.Join(", ", _events);

        public override void Awake()
        {
            _events.Add("Awake");
        }

        public override void OnEnable()
        {
            _events.Add("OnEnable");
        }

        public override void Start()
        {
            _events.Add("Start");
        }

        public override void FixedUpdate()
        {
            _events.Add("FixedUpdate");
        }

        public override void Update()
        {
            _events.Add("Update");
        }

        public override void LateUpdate()
        {
            _events.Add("LateUpdate");
        }

        public override void OnDisable()
        {
            _events.Add("OnDisable");
        }

        public override void OnDestroy()
        {
            _events.Add("OnDestroy");
        }
    }
}
