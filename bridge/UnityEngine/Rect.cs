using System;
using System.Globalization;

namespace UnityEngine
{
    public struct Rect : IEquatable<Rect>
    {
        private float _xMin;
        private float _yMin;
        private float _width;
        private float _height;

        public Rect(float x, float y, float width, float height)
        {
            _xMin = x;
            _yMin = y;
            _width = width;
            _height = height;
        }

        public Rect(Vector2 position, Vector2 size)
        {
            _xMin = position.x;
            _yMin = position.y;
            _width = size.x;
            _height = size.y;
        }

        public Rect(Rect source)
        {
            _xMin = source._xMin;
            _yMin = source._yMin;
            _width = source._width;
            _height = source._height;
        }

        public static Rect zero => new Rect(0f, 0f, 0f, 0f);

        public static Rect MinMaxRect(float xmin, float ymin, float xmax, float ymax)
        {
            return new Rect(xmin, ymin, xmax - xmin, ymax - ymin);
        }

        public void Set(float x, float y, float width, float height)
        {
            _xMin = x;
            _yMin = y;
            _width = width;
            _height = height;
        }

        public float x
        {
            get => _xMin;
            set => _xMin = value;
        }

        public float y
        {
            get => _yMin;
            set => _yMin = value;
        }

        public Vector2 position
        {
            get => new Vector2(_xMin, _yMin);
            set
            {
                _xMin = value.x;
                _yMin = value.y;
            }
        }

        public Vector2 center
        {
            get => new Vector2(x + (_width / 2f), y + (_height / 2f));
            set
            {
                _xMin = value.x - (_width / 2f);
                _yMin = value.y - (_height / 2f);
            }
        }

        public Vector2 min
        {
            get => new Vector2(xMin, yMin);
            set
            {
                xMin = value.x;
                yMin = value.y;
            }
        }

        public Vector2 max
        {
            get => new Vector2(xMax, yMax);
            set
            {
                xMax = value.x;
                yMax = value.y;
            }
        }

        public float width
        {
            get => _width;
            set => _width = value;
        }

        public float height
        {
            get => _height;
            set => _height = value;
        }

        public Vector2 size
        {
            get => new Vector2(_width, _height);
            set
            {
                _width = value.x;
                _height = value.y;
            }
        }

        public float xMin
        {
            get => _xMin;
            set
            {
                float oldxmax = xMax;
                _xMin = value;
                _width = oldxmax - _xMin;
            }
        }

        public float yMin
        {
            get => _yMin;
            set
            {
                float oldymax = yMax;
                _yMin = value;
                _height = oldymax - _yMin;
            }
        }

        public float xMax
        {
            get => _width + _xMin;
            set => _width = value - _xMin;
        }

        public float yMax
        {
            get => _height + _yMin;
            set => _height = value - _yMin;
        }

        public bool Contains(Vector2 point)
        {
            return point.x >= xMin && point.x < xMax && point.y >= yMin && point.y < yMax;
        }

        public bool Contains(Vector3 point)
        {
            return point.x >= xMin && point.x < xMax && point.y >= yMin && point.y < yMax;
        }

        public bool Contains(Vector3 point, bool allowInverse)
        {
            if (!allowInverse)
            {
                return Contains(point);
            }
            bool xAxis = (width < 0f && point.x <= xMin && point.x > xMax) || (width >= 0f && point.x >= xMin && point.x < xMax);
            bool yAxis = (height < 0f && point.y <= yMin && point.y > yMax) || (height >= 0f && point.y >= yMin && point.y < yMax);
            return xAxis && yAxis;
        }

        private static Rect OrderMinMax(Rect rect)
        {
            if (rect.xMin > rect.xMax)
            {
                float temp = rect.xMin;
                rect.xMin = rect.xMax;
                rect.xMax = temp;
            }
            if (rect.yMin > rect.yMax)
            {
                float temp = rect.yMin;
                rect.yMin = rect.yMax;
                rect.yMax = temp;
            }
            return rect;
        }

        public bool Overlaps(Rect other)
        {
            return other.xMax > xMin && other.xMin < xMax && other.yMax > yMin && other.yMin < yMax;
        }

        public bool Overlaps(Rect other, bool allowInverse)
        {
            Rect self = this;
            if (allowInverse)
            {
                self = OrderMinMax(self);
                other = OrderMinMax(other);
            }
            return self.Overlaps(other);
        }

        public static Vector2 NormalizedToPoint(Rect rectangle, Vector2 normalizedRectCoordinates)
        {
            return new Vector2(
                Mathf.Lerp(rectangle.x, rectangle.xMax, normalizedRectCoordinates.x),
                Mathf.Lerp(rectangle.y, rectangle.yMax, normalizedRectCoordinates.y));
        }

        public static Vector2 PointToNormalized(Rect rectangle, Vector2 point)
        {
            return new Vector2(
                Mathf.InverseLerp(rectangle.x, rectangle.xMax, point.x),
                Mathf.InverseLerp(rectangle.y, rectangle.yMax, point.y));
        }

        public static bool operator !=(Rect lhs, Rect rhs) => !(lhs == rhs);

        public static bool operator ==(Rect lhs, Rect rhs) =>
            lhs.x == rhs.x && lhs.y == rhs.y && lhs.width == rhs.width && lhs.height == rhs.height;

        public bool Equals(Rect other) =>
            x.Equals(other.x) && y.Equals(other.y) && width.Equals(other.width) && height.Equals(other.height);

        public override bool Equals(object? other) => other is Rect r && Equals(r);

        public override int GetHashCode() => HashCode.Combine(x, width, y, height);

        public override string ToString() =>
            string.Format(CultureInfo.InvariantCulture, "(x:{0:F2}, y:{1:F2}, width:{2:F2}, height:{3:F2})", x, y, width, height);
    }
}
