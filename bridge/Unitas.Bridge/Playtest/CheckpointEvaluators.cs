using System;
using System.Globalization;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Playtest
{
    internal static class CheckpointEvaluators
    {
        internal static CheckpointResult EvaluateTransformPosition(ScenarioCheckpoint checkpoint)
        {
            string? objectName = checkpoint.ObjectName;
            if (string.IsNullOrEmpty(objectName))
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "object name missing");
            }

            GameObject? target = GameObject.Find(objectName);
            if (target == null)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "object not found: " + objectName);
            }

            double[]? expected = checkpoint.Expected;
            if (expected == null || expected.Length != 3)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "expected position missing");
            }

            Vector3 position = target.transform.position;
            bool within = Math.Abs(position.x - expected[0]) <= checkpoint.Tolerance
                && Math.Abs(position.y - expected[1]) <= checkpoint.Tolerance
                && Math.Abs(position.z - expected[2]) <= checkpoint.Tolerance;
            CheckpointStatus status = within ? CheckpointStatus.Passed : CheckpointStatus.Failed;
            return new CheckpointResult(checkpoint.Id, status, FormatVector(expected[0], expected[1], expected[2]), FormatVector(position.x, position.y, position.z), null);
        }

        internal static CheckpointResult EvaluateLog(ScenarioCheckpoint checkpoint)
        {
            string contains = checkpoint.Contains ?? string.Empty;
            LogSeverity? required = RequiredSeverity(checkpoint.Severity);
            string expected = contains.Length > 0 ? contains : "<any>";
            foreach (LogEntry entry in LogRecorder.Entries)
            {
                if (required.HasValue && entry.Severity != required.Value)
                {
                    continue;
                }

                if (contains.Length > 0 && entry.Message.IndexOf(contains, StringComparison.Ordinal) < 0)
                {
                    continue;
                }

                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Passed, expected, entry.Message, null);
            }

            return new CheckpointResult(checkpoint.Id, CheckpointStatus.Failed, expected, null, "no matching log message");
        }

        internal static CheckpointResult EvaluateFrameCount(ScenarioCheckpoint checkpoint, int frameCount)
        {
            int? expected = checkpoint.ExpectedFrameCount;
            if (!expected.HasValue)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "expected frame count missing");
            }

            CheckpointStatus status = frameCount == expected.Value ? CheckpointStatus.Passed : CheckpointStatus.Failed;
            return new CheckpointResult(checkpoint.Id, status, expected.Value.ToString(CultureInfo.InvariantCulture), frameCount.ToString(CultureInfo.InvariantCulture), null);
        }

        internal static CheckpointResult EvaluateRenderCapture(ScenarioCheckpoint checkpoint, string capturePath, out string? writtenPath)
        {
            writtenPath = null;
            string? cameraName = checkpoint.CameraName;
            if (string.IsNullOrEmpty(cameraName))
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "camera not found: " + cameraName);
            }

            GameObject? cameraObject = GameObject.Find(cameraName);
            Camera? camera = cameraObject != null ? cameraObject.GetComponent<Camera>() : null;
            if (camera == null)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "camera not found: " + cameraName);
            }

            camera.pixelWidth = checkpoint.Width;
            camera.pixelHeight = checkpoint.Height;
            try
            {
                string? directory = Path.GetDirectoryName(capturePath);
                if (!string.IsNullOrEmpty(directory))
                {
                    Directory.CreateDirectory(directory);
                }

                if (File.Exists(capturePath))
                {
                    File.Delete(capturePath);
                }

                camera.CaptureToFile(capturePath);
            }
            catch (Exception ex)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "capture failed: " + ex.Message);
            }

            if (!File.Exists(capturePath))
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "capture failed");
            }

            writtenPath = capturePath;
            BmpImage image;
            try
            {
                image = BmpImage.Load(capturePath);
            }
            catch (InvalidDataException ex)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "capture failed: " + ex.Message);
            }

            int different = image.CountPixelsDifferentFrom(camera.backgroundColor);
            string observed = different.ToString(CultureInfo.InvariantCulture);
            int? threshold = checkpoint.ExpectedNonBackgroundPixels;
            string? referencePath = checkpoint.ReferencePath;
            if (string.IsNullOrEmpty(referencePath))
            {
                if (!threshold.HasValue)
                {
                    return new CheckpointResult(checkpoint.Id, CheckpointStatus.Passed, "<any>", observed, null);
                }

                CheckpointStatus status = different >= threshold.Value ? CheckpointStatus.Passed : CheckpointStatus.Failed;
                return new CheckpointResult(checkpoint.Id, status, threshold.Value.ToString(CultureInfo.InvariantCulture), observed, null);
            }

            if (threshold.HasValue && different < threshold.Value)
            {
                return new CheckpointResult(
                    checkpoint.Id,
                    CheckpointStatus.Failed,
                    threshold.Value.ToString(CultureInfo.InvariantCulture),
                    observed,
                    "non-background pixel threshold not met: " + observed + " is below " + threshold.Value.ToString(CultureInfo.InvariantCulture));
            }

            if (!File.Exists(referencePath))
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Failed, null, null, "reference image not found: " + referencePath);
            }

            BmpImage reference;
            try
            {
                reference = BmpImage.Load(referencePath);
            }
            catch (Exception ex)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Failed, null, null, "reference image could not be read: " + referencePath + " (" + ex.Message + ")");
            }

            if (reference.Width != image.Width || reference.Height != image.Height)
            {
                return new CheckpointResult(
                    checkpoint.Id,
                    CheckpointStatus.Failed,
                    null,
                    null,
                    "reference image size " + reference.Width.ToString(CultureInfo.InvariantCulture) + "x" + reference.Height.ToString(CultureInfo.InvariantCulture)
                    + " does not match capture size " + image.Width.ToString(CultureInfo.InvariantCulture) + "x" + image.Height.ToString(CultureInfo.InvariantCulture));
            }

            int differing = image.CountPixelsDifferentFrom(reference);
            int total = image.Width * image.Height;
            double fraction = total > 0 ? (double)differing / total : 0.0;
            double tolerance = checkpoint.Tolerance;
            string referenceExpected = "tolerance " + FormatFraction(tolerance);
            string referenceObserved = differing.ToString(CultureInfo.InvariantCulture) + "/" + total.ToString(CultureInfo.InvariantCulture) + " (" + FormatFraction(fraction) + ")";
            if (fraction > tolerance)
            {
                return new CheckpointResult(
                    checkpoint.Id,
                    CheckpointStatus.Failed,
                    referenceExpected,
                    referenceObserved,
                    "reference comparison failed: " + referenceObserved + " exceeds " + referenceExpected);
            }

            return new CheckpointResult(checkpoint.Id, CheckpointStatus.Passed, referenceExpected, referenceObserved, null);
        }

        private static LogSeverity? RequiredSeverity(string? severity)
        {
            if (string.IsNullOrEmpty(severity))
            {
                return null;
            }

            if (string.Equals(severity, "any", StringComparison.OrdinalIgnoreCase))
            {
                return null;
            }

            if (string.Equals(severity, "log", StringComparison.OrdinalIgnoreCase) || string.Equals(severity, "info", StringComparison.OrdinalIgnoreCase))
            {
                return LogSeverity.Info;
            }

            if (string.Equals(severity, "warning", StringComparison.OrdinalIgnoreCase))
            {
                return LogSeverity.Warning;
            }

            if (string.Equals(severity, "error", StringComparison.OrdinalIgnoreCase))
            {
                return LogSeverity.Error;
            }

            return null;
        }

        private static string FormatVector(double x, double y, double z)
        {
            return "(" + FormatNumber(x) + ", " + FormatNumber(y) + ", " + FormatNumber(z) + ")";
        }

        private static string FormatNumber(double value)
        {
            return value.ToString("F6", CultureInfo.InvariantCulture);
        }

        private static string FormatFraction(double value)
        {
            return value.ToString("F4", CultureInfo.InvariantCulture);
        }
    }
}
