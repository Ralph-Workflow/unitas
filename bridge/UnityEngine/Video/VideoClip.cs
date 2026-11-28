using System;
using UnityEngine;

namespace UnityEngine.Video
{
    public sealed class VideoClip : Object
    {
        private string _originalPath = string.Empty;
        private uint _width;
        private uint _height;
        private ulong _frameCount;
        private double _frameRate;
        private double _length;

        public VideoClip()
        {
        }

        public VideoClip(
            string originalPath,
            uint width,
            uint height,
            ulong frameCount,
            double frameRate,
            double length)
        {
            _originalPath = originalPath ?? string.Empty;
            _width = width;
            _height = height;
            _frameCount = frameCount;
            _frameRate = frameRate;
            _length = length;
        }

        public string originalPath
        {
            get
            {
                ThrowIfDestroyed();
                return _originalPath;
            }
            set
            {
                ThrowIfDestroyed();
                _originalPath = value ?? string.Empty;
            }
        }

        public uint width
        {
            get
            {
                ThrowIfDestroyed();
                return _width;
            }
            internal set => _width = value;
        }

        public uint height
        {
            get
            {
                ThrowIfDestroyed();
                return _height;
            }
            internal set => _height = value;
        }

        public ulong frameCount
        {
            get
            {
                ThrowIfDestroyed();
                return _frameCount;
            }
            internal set => _frameCount = value;
        }

        public double frameRate
        {
            get
            {
                ThrowIfDestroyed();
                return _frameRate;
            }
            internal set => _frameRate = value;
        }

        public double length
        {
            get
            {
                ThrowIfDestroyed();
                return _length;
            }
            internal set => _length = value;
        }
    }
}
