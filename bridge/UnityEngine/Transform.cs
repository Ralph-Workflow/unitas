using System.Collections.Generic;

namespace UnityEngine
{
    public class Transform : Component
    {
        private readonly List<Transform> _children = new List<Transform>();
        private Transform? _parent;
        private Vector3 _localPosition;

        public new Transform transform => this;

        public Vector3 localPosition
        {
            get => _localPosition;
            set => _localPosition = value;
        }

        public Vector3 position
        {
            get
            {
                if (_parent != null)
                {
                    return _parent.position + _localPosition;
                }
                return _localPosition;
            }
            set
            {
                if (_parent != null)
                {
                    _localPosition = value - _parent.position;
                }
                else
                {
                    _localPosition = value;
                }
                PushKinematicPositionToNative();
            }
        }

        public Transform? parent
        {
            get => _parent;
            set => SetParent(value);
        }

        public int childCount => _children.Count;

        public Transform GetChild(int index) => _children[index];

        public void SetParent(Transform? parent)
        {
            SetParent(parent, true);
        }

        public void SetParent(Transform? parent, bool worldPositionStays)
        {
            if (_parent == parent)
            {
                return;
            }

            Vector3 worldPos = position;

            if (_parent != null)
            {
                _parent._children.Remove(this);
            }

            _parent = parent;

            if (_parent != null)
            {
                _parent._children.Add(this);
                if (worldPositionStays)
                {
                    _localPosition = worldPos - _parent.position;
                }
            }
            else if (worldPositionStays)
            {
                _localPosition = worldPos;
            }
        }

        public void Translate(Vector3 translation)
        {
            position += translation;
        }

        private void PushKinematicPositionToNative()
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return;
            }
            Rigidbody? body = owner.GetComponent<Rigidbody>();
            if (body == null || !body.isKinematic)
            {
                return;
            }
            IPhysicsProvider? provider = Physics.Provider;
            if (provider != null)
            {
                provider.TrySetBodyPosition(owner.NativeEntityId, position);
            }
        }
    }
}
