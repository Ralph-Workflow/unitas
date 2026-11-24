using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public struct InputFrameSnapshot
    {
        public bool KeyDown;
        public bool KeyHeld;
        public bool KeyUp;
        public bool MouseButtonDown;
        public bool MouseButtonHeld;
        public bool MouseButtonUp;
        public Vector3 MousePosition;
        public Vector3 MousePositionDelta;
        public Vector3 MouseScrollDelta;
        public float Horizontal;
        public float HorizontalRaw;
        public float Vertical;
        public float VerticalRaw;
        public bool Fire1Down;
        public bool Fire1Held;
        public bool Fire1Up;
        public float MouseX;
        public float MouseY;
        public float MouseScrollWheel;
    }

    public class InputRecorderScript : MonoBehaviour
    {
        private readonly List<InputFrameSnapshot> _fixedSnapshots = new List<InputFrameSnapshot>();
        private readonly List<InputFrameSnapshot> _updateSnapshots = new List<InputFrameSnapshot>();

        public KeyCode WatchedKey { get; set; } = KeyCode.Space;

        public int WatchedMouseButton { get; set; }

        public IReadOnlyList<InputFrameSnapshot> FixedSnapshots => _fixedSnapshots;

        public IReadOnlyList<InputFrameSnapshot> UpdateSnapshots => _updateSnapshots;

        public InputFrameSnapshot LastFixedSnapshot => _fixedSnapshots[_fixedSnapshots.Count - 1];

        public InputFrameSnapshot LastUpdateSnapshot => _updateSnapshots[_updateSnapshots.Count - 1];

        public override void FixedUpdate()
        {
            _fixedSnapshots.Add(CaptureSnapshot());
        }

        public override void Update()
        {
            _updateSnapshots.Add(CaptureSnapshot());
        }

        public void ClearSnapshots()
        {
            _fixedSnapshots.Clear();
            _updateSnapshots.Clear();
        }

        private InputFrameSnapshot CaptureSnapshot()
        {
            return new InputFrameSnapshot
            {
                KeyDown = Input.GetKeyDown(WatchedKey),
                KeyHeld = Input.GetKey(WatchedKey),
                KeyUp = Input.GetKeyUp(WatchedKey),
                MouseButtonDown = Input.GetMouseButtonDown(WatchedMouseButton),
                MouseButtonHeld = Input.GetMouseButton(WatchedMouseButton),
                MouseButtonUp = Input.GetMouseButtonUp(WatchedMouseButton),
                MousePosition = Input.mousePosition,
                MousePositionDelta = Input.mousePositionDelta,
                MouseScrollDelta = Input.mouseScrollDelta,
                Horizontal = Input.GetAxis("Horizontal"),
                HorizontalRaw = Input.GetAxisRaw("Horizontal"),
                Vertical = Input.GetAxis("Vertical"),
                VerticalRaw = Input.GetAxisRaw("Vertical"),
                Fire1Down = Input.GetButtonDown("Fire1"),
                Fire1Held = Input.GetButton("Fire1"),
                Fire1Up = Input.GetButtonUp("Fire1"),
                MouseX = Input.GetAxis("Mouse X"),
                MouseY = Input.GetAxis("Mouse Y"),
                MouseScrollWheel = Input.GetAxis("Mouse ScrollWheel"),
            };
        }
    }
}
