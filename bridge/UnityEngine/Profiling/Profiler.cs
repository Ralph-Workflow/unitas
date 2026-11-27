using System;
using System.Collections.Generic;

namespace UnityEngine.Profiling
{
    public interface IProfilerHost
    {
        bool IsRecording { get; }

        void StartCapture();

        void StopCapture();

        void BeginSample(string name);

        void EndSample();

        ProfilerCapture GetCapture();
    }

    public sealed class ProfilerMarkerNode
    {
        private readonly List<ProfilerMarkerNode> _children;

        public ProfilerMarkerNode(string name, ulong durationNanoseconds, int depth)
            : this(name, durationNanoseconds, depth, new List<ProfilerMarkerNode>())
        {
        }

        public ProfilerMarkerNode(
            string name,
            ulong durationNanoseconds,
            int depth,
            List<ProfilerMarkerNode> children)
        {
            Name = name ?? string.Empty;
            DurationNanoseconds = durationNanoseconds;
            Depth = depth;
            _children = children ?? new List<ProfilerMarkerNode>();
        }

        public string Name { get; }

        public ulong DurationNanoseconds { get; }

        public int Depth { get; }

        public IReadOnlyList<ProfilerMarkerNode> Children => _children;

        public void AddChild(ProfilerMarkerNode child)
        {
            if (child != null)
            {
                _children.Add(child);
            }
        }

        public ProfilerMarkerNode? FindChild(string name)
        {
            for (int index = 0; index < _children.Count; index++)
            {
                if (_children[index].Name == name)
                {
                    return _children[index];
                }
            }

            return null;
        }

        public ProfilerMarkerNode? FindDescendant(string name)
        {
            for (int index = 0; index < _children.Count; index++)
            {
                ProfilerMarkerNode child = _children[index];
                if (child.Name == name)
                {
                    return child;
                }

                ProfilerMarkerNode? nested = child.FindDescendant(name);
                if (nested != null)
                {
                    return nested;
                }
            }

            return null;
        }
    }

    public sealed class ProfilerCapture
    {
        public static readonly ProfilerCapture Empty = new ProfilerCapture(
            new ProfilerMarkerNode("Capture", 0, 0),
            Array.Empty<ProfilerMarkerNode>());

        public ProfilerCapture(ProfilerMarkerNode root, IReadOnlyList<ProfilerMarkerNode> nodes)
        {
            Root = root ?? new ProfilerMarkerNode("Capture", 0, 0);
            Nodes = nodes ?? Array.Empty<ProfilerMarkerNode>();
        }

        public ProfilerMarkerNode Root { get; }

        public IReadOnlyList<ProfilerMarkerNode> Nodes { get; }

        public IReadOnlyList<ProfilerMarkerNode> Frames => Root.Children;
    }

    public static class Profiler
    {
        private static bool _enabled;

        public static IProfilerHost? ProfilerHost { get; set; }

        public static bool enabled
        {
            get
            {
                IProfilerHost? host = ProfilerHost;
                return host != null ? host.IsRecording : _enabled;
            }
            set
            {
                _enabled = value;
                IProfilerHost? host = ProfilerHost;
                if (host == null)
                {
                    return;
                }

                if (value)
                {
                    host.StartCapture();
                }
                else
                {
                    host.StopCapture();
                }
            }
        }

        public static void StartCapture()
        {
            _enabled = true;
            ProfilerHost?.StartCapture();
        }

        public static void StopCapture()
        {
            _enabled = false;
            ProfilerHost?.StopCapture();
        }

        public static ProfilerCapture GetCapture()
        {
            IProfilerHost? host = ProfilerHost;
            return host != null ? host.GetCapture() : ProfilerCapture.Empty;
        }

        public static void BeginSample(string name)
        {
            if (string.IsNullOrEmpty(name))
            {
                return;
            }

            ProfilerHost?.BeginSample(name);
        }

        public static void BeginSample(string name, Object? targetObject)
        {
            _ = targetObject;
            BeginSample(name);
        }

        public static void EndSample()
        {
            ProfilerHost?.EndSample();
        }
    }
}
