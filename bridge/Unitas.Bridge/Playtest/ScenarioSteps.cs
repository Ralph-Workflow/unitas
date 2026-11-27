using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Playtest
{
    public enum ScenarioStepKind
    {
        LoadScene,
        Tick,
        InjectKey,
        InjectMouseButton,
        InjectPointer,
        InjectScroll,
        SetTimeScale,
        SetQuality,
        InjectGamepadAxis,
    }

    public abstract class ScenarioStep
    {
        public abstract ScenarioStepKind Kind { get; }
    }

    public sealed class LoadSceneStep : ScenarioStep
    {
        public LoadSceneStep(string path, LoadSceneMode mode)
        {
            Path = path;
            Mode = mode;
        }

        public string Path { get; }

        public LoadSceneMode Mode { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.LoadScene;
    }

    public sealed class TickStep : ScenarioStep
    {
        public TickStep(int frames, double deltaSeconds)
        {
            Frames = frames;
            DeltaSeconds = deltaSeconds;
        }

        public int Frames { get; }

        public double DeltaSeconds { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.Tick;
    }

    public sealed class InjectKeyStep : ScenarioStep
    {
        public InjectKeyStep(string key, bool pressed)
        {
            Key = key;
            Pressed = pressed;
        }

        public string Key { get; }

        public bool Pressed { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.InjectKey;
    }

    public sealed class InjectMouseButtonStep : ScenarioStep
    {
        public InjectMouseButtonStep(int button, bool pressed)
        {
            Button = button;
            Pressed = pressed;
        }

        public int Button { get; }

        public bool Pressed { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.InjectMouseButton;
    }

    public sealed class InjectPointerStep : ScenarioStep
    {
        public InjectPointerStep(double x, double y)
        {
            X = x;
            Y = y;
        }

        public double X { get; }

        public double Y { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.InjectPointer;
    }

    public sealed class InjectScrollStep : ScenarioStep
    {
        public InjectScrollStep(double x, double y)
        {
            X = x;
            Y = y;
        }

        public double X { get; }

        public double Y { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.InjectScroll;
    }

    public sealed class SetTimeScaleStep : ScenarioStep
    {
        public SetTimeScaleStep(double value)
        {
            Value = value;
        }

        public double Value { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.SetTimeScale;
    }

    public sealed class SetQualityStep : ScenarioStep
    {
        public SetQualityStep(int level)
        {
            Level = level;
        }

        public int Level { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.SetQuality;
    }

    public sealed class InjectGamepadAxisStep : ScenarioStep
    {
        public InjectGamepadAxisStep(string axis, double value)
        {
            Axis = axis;
            Value = value;
        }

        public string Axis { get; }

        public double Value { get; }

        public override ScenarioStepKind Kind => ScenarioStepKind.InjectGamepadAxis;
    }

    public enum CheckpointKind
    {
        TransformPosition,
        Log,
        FrameCount,
        RenderCapture,
    }

    public sealed class ScenarioCheckpoint
    {
        public ScenarioCheckpoint(
            string id,
            int afterStep,
            CheckpointKind kind,
            string? objectName,
            double[]? expected,
            double tolerance,
            string? severity,
            string? contains,
            int? expectedFrameCount,
            string? cameraName,
            int width,
            int height,
            int? expectedNonBackgroundPixels,
            string? referencePath = null)
        {
            Id = id;
            AfterStep = afterStep;
            Kind = kind;
            ObjectName = objectName;
            Expected = expected;
            Tolerance = tolerance;
            Severity = severity;
            Contains = contains;
            ExpectedFrameCount = expectedFrameCount;
            CameraName = cameraName;
            Width = width;
            Height = height;
            ExpectedNonBackgroundPixels = expectedNonBackgroundPixels;
            ReferencePath = referencePath;
        }

        public string Id { get; }

        public int AfterStep { get; }

        public CheckpointKind Kind { get; }

        public string? ObjectName { get; }

        public double[]? Expected { get; }

        public double Tolerance { get; }

        public string? Severity { get; }

        public string? Contains { get; }

        public int? ExpectedFrameCount { get; }

        public string? CameraName { get; }

        public int Width { get; }

        public int Height { get; }

        public int? ExpectedNonBackgroundPixels { get; }

        public string? ReferencePath { get; }
    }
}
