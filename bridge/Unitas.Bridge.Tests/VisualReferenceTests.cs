using System;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.Json;
using Unitas.Bridge.Playtest;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class VisualReferenceTests
    {
        private const string FixtureDirectory = "bridge/Unitas.Bridge.Tests/Fixtures/playtest/";
        private const string SceneRelativePath = FixtureDirectory + "Assets/Scenes/playtest_scene.unity";
        private const int CaptureSize = 64;
        private const int CapturePixels = CaptureSize * CaptureSize;
        private const string ReferenceFileName = "reference.bmp";
        private const byte FillBlue = 255;

        public static void Run()
        {
            string directory = Path.Combine(Path.GetTempPath(), "unitas-visual-reference");
            Directory.CreateDirectory(directory);
            try
            {
                MatchingReferencePasses(directory);
                MismatchingReferenceFails(directory);
                SizeMismatchFails(directory);
                MissingReferenceFails(directory);
                AbsentReferenceKeepsLegacyBehaviour(directory);
                CombinedChecksBothApply(directory);
                SchemaValidation(directory);
            }
            finally
            {
                DeleteDirectory(directory);
            }
        }

        private static void MatchingReferencePasses(string directory)
        {
            string referencePath = WriteReference(directory, ReferenceFileName, CaptureSize, 0, 0);
            ScenarioDocument document = Parse(ScenarioJson("visual-match", ReferenceFileName, null, null), directory);
            CheckpointResult checkpoint = RunCapture(document, new FixedFrameProvider(0, 0));

            Check.Equal("Matching reference passes", "Passed", checkpoint.Status.ToString());
            Check.Equal("Matching reference reports the tolerance", "tolerance 0.0000", checkpoint.Expected ?? string.Empty);
            Check.Equal("Matching reference reports the differing fraction", "0/" + CapturePixels.ToString(CultureInfo.InvariantCulture) + " (0.0000)", checkpoint.Observed ?? string.Empty);
            Check.Equal("Matching reference has no failure reason", string.Empty, checkpoint.Reason ?? string.Empty);
            Check.That("Matching reference uses the resolved reference path", File.Exists(referencePath));
        }

        private static void MismatchingReferenceFails(string directory)
        {
            WriteReference(directory, ReferenceFileName, CaptureSize, 16, 16);
            ScenarioDocument document = Parse(ScenarioJson("visual-mismatch", ReferenceFileName, "0.01", null), directory);
            CheckpointResult checkpoint = RunCapture(document, new FixedFrameProvider(0, 0));

            Check.Equal("Mismatching reference fails", "Failed", checkpoint.Status.ToString());
            Check.Equal("Mismatching reference reports the allowed tolerance", "tolerance 0.0100", checkpoint.Expected ?? string.Empty);
            Check.Equal(
                "Mismatching reference reports the observed fraction",
                "256/" + CapturePixels.ToString(CultureInfo.InvariantCulture) + " (0.0625)",
                checkpoint.Observed ?? string.Empty);
            string reason = checkpoint.Reason ?? string.Empty;
            Check.That("Mismatching reference reason states the observed fraction", reason.Contains("0.0625", StringComparison.Ordinal));
            Check.That("Mismatching reference reason states the allowed tolerance", reason.Contains("tolerance 0.0100", StringComparison.Ordinal));
        }

        private static void SizeMismatchFails(string directory)
        {
            WriteReference(directory, ReferenceFileName, CaptureSize / 2, 0, 0);
            ScenarioDocument document = Parse(ScenarioJson("visual-size", ReferenceFileName, "0.5", null), directory);
            CheckpointResult checkpoint = RunCapture(document, new FixedFrameProvider(0, 0));

            Check.Equal("Reference size mismatch fails", "Failed", checkpoint.Status.ToString());
            string reason = checkpoint.Reason ?? string.Empty;
            Check.That("Reference size mismatch reason states the capture size", reason.Contains("64x64", StringComparison.Ordinal));
            Check.That("Reference size mismatch reason states the reference size", reason.Contains("32x32", StringComparison.Ordinal));
        }

        private static void MissingReferenceFails(string directory)
        {
            ScenarioDocument document = Parse(ScenarioJson("visual-missing", "missing.bmp", null, null), directory);
            CheckpointResult checkpoint = RunCapture(document, new FixedFrameProvider(0, 0));

            Check.Equal("Missing reference fails", "Failed", checkpoint.Status.ToString());
            Check.That("Missing reference is not unevaluable", checkpoint.Status != CheckpointStatus.Unevaluable);
            string reason = checkpoint.Reason ?? string.Empty;
            Check.That("Missing reference reason states that the file was not found", reason.Contains("not found", StringComparison.Ordinal));
            Check.That(
                "Missing reference reason states the path",
                reason.Contains(Path.Combine(directory, "missing.bmp"), StringComparison.Ordinal));
        }

        private static void AbsentReferenceKeepsLegacyBehaviour(string directory)
        {
            ScenarioDocument document = Parse(ScenarioJson("visual-legacy", null, null, null), directory);
            CheckpointResult checkpoint = RunCapture(document, new FixedFrameProvider(0, 0));
            Check.Equal("Legacy capture without a threshold passes", "Passed", checkpoint.Status.ToString());
            Check.Equal("Legacy capture without a threshold expects any value", "<any>", checkpoint.Expected ?? string.Empty);
            Check.Equal("Legacy capture without a threshold reports the non-background count", CapturePixels.ToString(CultureInfo.InvariantCulture), checkpoint.Observed ?? string.Empty);

            ScenarioDocument passing = Parse(ScenarioJson("visual-legacy-threshold", null, null, CapturePixels), directory);
            CheckpointResult passingCheckpoint = RunCapture(passing, new FixedFrameProvider(0, 0));
            Check.Equal("Legacy capture at the pixel threshold passes", "Passed", passingCheckpoint.Status.ToString());

            ScenarioDocument failing = Parse(ScenarioJson("visual-legacy-threshold-miss", null, null, CapturePixels + 1), directory);
            CheckpointResult failingCheckpoint = RunCapture(failing, new FixedFrameProvider(0, 0));
            Check.Equal("Legacy capture below the pixel threshold fails", "Failed", failingCheckpoint.Status.ToString());
            Check.Equal(
                "Legacy capture below the pixel threshold expects the threshold",
                (CapturePixels + 1).ToString(CultureInfo.InvariantCulture),
                failingCheckpoint.Expected ?? string.Empty);
        }

        private static void CombinedChecksBothApply(string directory)
        {
            WriteReference(directory, ReferenceFileName, CaptureSize, 0, 0);
            ScenarioDocument failing = Parse(ScenarioJson("visual-combined-fail", ReferenceFileName, null, CapturePixels + 1), directory);
            CheckpointResult failingCheckpoint = RunCapture(failing, new FixedFrameProvider(0, 0));
            Check.Equal("Matching reference with a failing pixel threshold fails", "Failed", failingCheckpoint.Status.ToString());
            string reason = failingCheckpoint.Reason ?? string.Empty;
            Check.That("Matching reference with a failing pixel threshold names the pixel threshold", reason.Contains("non-background pixel threshold", StringComparison.Ordinal));

            ScenarioDocument passing = Parse(ScenarioJson("visual-combined-pass", ReferenceFileName, null, CapturePixels), directory);
            CheckpointResult passingCheckpoint = RunCapture(passing, new FixedFrameProvider(0, 0));
            Check.Equal("Matching reference with a passing pixel threshold passes", "Passed", passingCheckpoint.Status.ToString());
        }

        private static void SchemaValidation(string directory)
        {
            Check.That("Tolerance above one is rejected", Rejects(ScenarioJson("visual-schema-high", ReferenceFileName, "1.5", null), directory));
            Check.That("Negative tolerance is rejected", Rejects(ScenarioJson("visual-schema-negative", ReferenceFileName, "-0.1", null), directory));
            Check.That("Tolerance without a reference is rejected", Rejects(ScenarioJson("visual-schema-orphan", null, "0.5", null), directory));
            Check.That("A missing reference file is not a schema error", !Rejects(ScenarioJson("visual-schema-missing-file", "absent.bmp", null, null), directory));

            ScenarioDocument document = Parse(ScenarioJson("visual-schema-bounds", ReferenceFileName, "1", null), directory);
            Check.Equal("Tolerance of one is accepted", 1.0, document.Checkpoints[0].Tolerance);

            ScenarioDocument defaulted = Parse(ScenarioJson("visual-schema-default", ReferenceFileName, null, null), directory);
            Check.Equal("A reference without a tolerance defaults to an exact match", 0.0, defaulted.Checkpoints[0].Tolerance);
            Check.Equal(
                "A relative reference resolves against the scenario directory",
                Path.GetFullPath(Path.Combine(directory, ReferenceFileName)),
                defaulted.Checkpoints[0].ReferencePath ?? string.Empty);

            ScenarioDocument current = Parse(ScenarioJson("visual-schema-current", ReferenceFileName, null, null), null);
            Check.Equal(
                "A relative reference without a base directory resolves against the current directory",
                Path.GetFullPath(ReferenceFileName),
                current.Checkpoints[0].ReferencePath ?? string.Empty);
        }

        private static bool Rejects(string json, string directory)
        {
            try
            {
                ScenarioDocument.Parse(json, directory);
                return false;
            }
            catch (ScenarioFormatException)
            {
                return true;
            }
        }

        private static ScenarioDocument Parse(string json, string? baseDirectory)
        {
            return ScenarioDocument.Parse(json, baseDirectory);
        }

        private static CheckpointResult RunCapture(ScenarioDocument document, FixedFrameProvider provider)
        {
            IRenderProvider? saved = Camera.Provider;
            try
            {
                ScenarioResult result = new ScenarioRunner().Run(document, () =>
                {
                    BridgeHost host = new BridgeHost();
                    Camera.Provider = provider;
                    return host;
                });

                Check.That("The scenario produced one checkpoint", result.Checkpoints.Count == 1);
                return result.Checkpoints[0];
            }
            finally
            {
                Camera.Provider = saved;
            }
        }

        private static string WriteReference(string directory, string fileName, int size, int diffWidth, int diffHeight)
        {
            string path = Path.Combine(directory, fileName);
            File.WriteAllBytes(path, BuildBmp(size, size, 0, 0, FillBlue, diffWidth, diffHeight));
            return path;
        }

        private static string ScenarioJson(string name, string? reference, string? tolerance, int? expectedNonBackgroundPixels)
        {
            StringBuilder builder = new StringBuilder();
            builder.Append("{\"name\":").Append(JsonSerializer.Serialize(name));
            builder.Append(",\"scene\":").Append(JsonSerializer.Serialize(ResolveFixture(SceneRelativePath)));
            builder.Append(",\"checkpoints\":[{\"id\":\"shot\",\"after_step\":0,\"kind\":\"render_capture\",\"camera\":\"Main Camera\"");
            builder.Append(",\"width\":").Append(CaptureSize.ToString(CultureInfo.InvariantCulture));
            builder.Append(",\"height\":").Append(CaptureSize.ToString(CultureInfo.InvariantCulture));
            if (reference != null)
            {
                builder.Append(",\"reference\":").Append(JsonSerializer.Serialize(reference));
            }

            if (tolerance != null)
            {
                builder.Append(",\"tolerance\":").Append(tolerance);
            }

            if (expectedNonBackgroundPixels.HasValue)
            {
                builder.Append(",\"expected_non_background_pixels\":").Append(expectedNonBackgroundPixels.Value.ToString(CultureInfo.InvariantCulture));
            }

            builder.Append("}]}");
            return builder.ToString();
        }

        private static byte[] BuildBmp(int width, int height, byte red, byte green, byte blue, int diffWidth, int diffHeight)
        {
            int stride = ((width * 3) + 3) & ~3;
            int pixelDataSize = stride * height;
            byte[] data = new byte[54 + pixelDataSize];
            data[0] = (byte)'B';
            data[1] = (byte)'M';
            WriteInt32(data, 2, data.Length);
            WriteInt32(data, 10, 54);
            WriteInt32(data, 14, 40);
            WriteInt32(data, 18, width);
            WriteInt32(data, 22, height);
            WriteInt16(data, 26, 1);
            WriteInt16(data, 28, 24);
            WriteInt32(data, 30, 0);
            WriteInt32(data, 34, pixelDataSize);
            for (int row = 0; row < height; row++)
            {
                int rowStart = 54 + (row * stride);
                for (int column = 0; column < width; column++)
                {
                    int offset = rowStart + (column * 3);
                    bool different = row < diffHeight && column < diffWidth;
                    data[offset] = different ? (byte)0 : blue;
                    data[offset + 1] = different ? (byte)0 : green;
                    data[offset + 2] = different ? (byte)255 : red;
                }
            }

            return data;
        }

        private static void WriteInt32(byte[] data, int offset, int value)
        {
            data[offset] = (byte)value;
            data[offset + 1] = (byte)(value >> 8);
            data[offset + 2] = (byte)(value >> 16);
            data[offset + 3] = (byte)(value >> 24);
        }

        private static void WriteInt16(byte[] data, int offset, int value)
        {
            data[offset] = (byte)value;
            data[offset + 1] = (byte)(value >> 8);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private static void DeleteDirectory(string directory)
        {
            try
            {
                if (Directory.Exists(directory))
                {
                    Directory.Delete(directory, true);
                }
            }
            catch (IOException)
            {
            }
        }

        private sealed class FixedFrameProvider : IRenderProvider
        {
            private readonly int _diffWidth;
            private readonly int _diffHeight;

            public FixedFrameProvider(int diffWidth, int diffHeight)
            {
                _diffWidth = diffWidth;
                _diffHeight = diffHeight;
            }

            public ulong CreateMesh(Vector3[] vertices, Vector3[] normals, int[] triangles)
            {
                return 7UL;
            }

            public void DestroyMesh(ulong meshHandle)
            {
            }

            public ulong CreateMaterial()
            {
                return 11UL;
            }

            public void SetMaterialColor(ulong materialHandle, Color color)
            {
            }

            public ulong CreateCamera(ulong entityId)
            {
                return 13UL;
            }

            public void UpdateCamera(ulong cameraHandle, float orthographicSize, float fieldOfView, float nearClipPlane, float farClipPlane, Color backgroundColor)
            {
            }

            public Vector3 WorldToScreenPoint(ulong cameraHandle, Vector3 worldPoint)
            {
                return Vector3.zero;
            }

            public Ray ScreenPointToRay(ulong cameraHandle, Vector3 screenPoint)
            {
                return default;
            }

            public ulong CreateRenderer(ulong entityId, ulong meshHandle, ulong materialHandle)
            {
                return 17UL;
            }

            public void SetRendererEnabled(ulong rendererHandle, bool enabled)
            {
            }

            public ulong CreateLight(ulong entityId)
            {
                return 19UL;
            }

            public void UpdateLight(ulong lightHandle, Vector3 direction, Color color, float intensity)
            {
            }

            public byte[] RenderFrame(ulong cameraHandle, uint width, uint height)
            {
                return BuildBmp((int)width, (int)height, 0, 0, FillBlue, _diffWidth, _diffHeight);
            }
        }
    }
}
