using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Ray
    {
        public Vector3 origin;
        public Vector3 direction;

        public Ray(Vector3 origin, Vector3 direction)
        {
            this.origin = origin;
            this.direction = direction;
        }

        public Vector3 GetPoint(float distance) => origin + (direction * distance);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "Origin: {0}, Dir: {1}", origin, direction);
    }
}
