using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using Unitas.Editor.Scene;

namespace Unitas.Editor.Tests
{
    public static class SceneTests
    {
        private const string RollABallScene = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";

        public static void Run()
        {
            RoundTripIsByteExact();
            SetPropertyInverseRestoresBytes();
            CreateThenInverseRestoresBytes();
            DestroyThenInverseRestoresBytes();
            ReparentThenInverseRestoresBytes();
            ReparentRefusesACycle();
            SetActiveThenInverseRestoresBytes();
            DuplicateThenInverseRestoresBytes();
            InspectReportsTheComponentsAndProperties();
            MalformedHeaderReportsFileAndLine();
            HierarchyMatchesTheSampleScene();
        }

        private static void RoundTripIsByteExact()
        {
            List<string> paths = FindSceneAssets();
            Check.That("scene: scene and prefab files were found", paths.Count >= 4);
            foreach (string path in paths)
            {
                byte[] expected = File.ReadAllBytes(path);
                SceneDocument document = SceneDocument.Open(path);
                byte[] actual = Encoding.UTF8.GetBytes(document.Save());
                Check.That(
                    "scene: round-trip is byte-exact for " + RelativePath(path),
                    expected.SequenceEqual(actual));
            }
        }

        private static void SetPropertyInverseRestoresBytes()
        {
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));
            SceneObject pickup = FindObject(document, "Pickup_1");
            long transformFileId = pickup.ComponentFileIds[0];

            SceneOp op = SceneOp.SetProperty(transformFileId, "m_LocalPosition.x", "3.5");
            SceneOp inverse = document.Apply(op);
            Check.That(
                "scene: the edited scene holds the new value",
                document.Save().Contains("m_LocalPosition: {x: 3.5, y: 0.75, z: 3}", StringComparison.Ordinal));
            Check.That(
                "scene: the inverse op carries the document from before the edit",
                inverse.DocumentSnapshot.Length > 0);

            document.Apply(inverse);
            Check.That(
                "scene: undo of a property edit restores the original bytes",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));
        }

        private static void CreateThenInverseRestoresBytes()
        {
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));
            int before = document.Objects.Count;

            SceneOp op = SceneOp.CreateGameObject("Unit Test Cube", 0);
            SceneOp inverse = document.Apply(op);
            string createdText = document.Save();
            Check.Equal("scene: create returns a destroy inverse", "DestroyGameObject", inverse.Kind.ToString());
            Check.Equal("scene: the created object is in the scene", before + 1, document.Objects.Count);

            SceneObject created = FindObject(document, "Unit Test Cube");
            Check.Equal("scene: the created object is a scene root", "0", Format(created.ParentFileId));
            Check.Equal("scene: the created object has one component", 1, created.ComponentFileIds.Count);

            document.Apply(inverse);
            Check.That(
                "scene: undo of a create restores the original bytes",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));

            document.Apply(op);
            Check.That("scene: redo of a create is byte-identical", createdText == document.Save());
        }

        private static void DestroyThenInverseRestoresBytes()
        {
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));
            int before = document.Objects.Count;

            SceneOp op = SceneOp.DestroyGameObject(1000000091);
            SceneOp inverse = document.Apply(op);
            string destroyedText = document.Save();
            Check.Equal("scene: destroy returns a create inverse", "CreateGameObject", inverse.Kind.ToString());
            Check.Equal("scene: the destroyed object is gone", before - 1, document.Objects.Count);
            Check.Equal(
                "scene: the parent keeps the other children",
                3,
                document.Find(1000000081).Children.Count);

            document.Apply(inverse);
            Check.That(
                "scene: undo of a destroy restores the original bytes",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));

            document.Apply(op);
            Check.That("scene: redo of a destroy is byte-identical", destroyedText == document.Save());
        }

        private static void ReparentThenInverseRestoresBytes()
        {
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));

            SceneOp op = SceneOp.Reparent(1000000091, 1000000021);
            SceneOp inverse = document.Apply(op);
            Check.Equal("scene: reparent returns a reparent inverse", "Reparent", inverse.Kind.ToString());
            Check.Equal(
                "scene: the reparented object knows its new parent",
                "1000000021",
                Format(document.Find(1000000091).ParentFileId));

            document.Apply(inverse);
            Check.That(
                "scene: undo of a reparent restores the original bytes",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));
        }

        private static void ReparentRefusesACycle()
        {
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));

            bool cycleRefused = false;
            try
            {
                document.Apply(SceneOp.Reparent(1000000081, 1000000091));
            }
            catch (SceneParseException error)
            {
                cycleRefused = true;
                Check.That("scene: the cycle refusal gives a reason", error.Reason.Contains("cycle", StringComparison.Ordinal));
            }

            Check.That("scene: a reparent that would create a cycle is refused", cycleRefused);

            bool selfRefused = false;
            try
            {
                document.Apply(SceneOp.Reparent(1000000091, 1000000091));
            }
            catch (SceneParseException error)
            {
                selfRefused = true;
                Check.That("scene: the self-parent refusal gives a reason", error.Reason.Length > 0);
            }

            Check.That("scene: an object cannot be its own parent", selfRefused);
            Check.That(
                "scene: a refused reparent leaves the scene untouched",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));
        }

        private static void SetActiveThenInverseRestoresBytes()
        {
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));

            SceneOp op = SceneOp.SetActive(1000000091, false);
            SceneOp inverse = document.Apply(op);
            Check.Equal("scene: set active returns a set active inverse", "SetActive", inverse.Kind.ToString());
            Check.That(
                "scene: the deactivated object is not active",
                !document.Inspect(1000000091).Active);

            document.Apply(inverse);
            Check.That(
                "scene: undo of an activation restores the original bytes",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));
        }

        private static void DuplicateThenInverseRestoresBytes()
        {
            byte[] original = File.ReadAllBytes(ResolveRepoPath(RollABallScene));
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));
            int before = document.Objects.Count;

            SceneOp op = SceneOp.DuplicateGameObject(1000000091, "Pickup_1 Copy", -1);
            SceneOp inverse = document.Apply(op);
            Check.Equal("scene: duplicate returns a destroy inverse", "DestroyGameObject", inverse.Kind.ToString());
            Check.Equal("scene: the copy is in the scene", before + 1, document.Objects.Count);

            SceneObject copy = FindObject(document, "Pickup_1 Copy");
            Check.Equal(
                "scene: the copy keeps the parent of the object it copies",
                "1000000081",
                Format(copy.ParentFileId));
            Check.Equal(
                "scene: the copy keeps the position of the object it copies",
                "{x: 3, y: 0.75, z: 3}",
                document.Inspect(copy.FileId).Position);

            document.Apply(inverse);
            Check.That(
                "scene: undo of a duplicate restores the original bytes",
                original.SequenceEqual(Encoding.UTF8.GetBytes(document.Save())));
        }

        private static void InspectReportsTheComponentsAndProperties()
        {
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));
            SceneInspection inspection = document.Inspect(1000000091);

            Check.Equal("scene: the inspection names the object", "Pickup_1", inspection.Name);
            Check.Equal("scene: the inspection names the parent", "1000000081", Format(inspection.ParentFileId));
            Check.Equal("scene: the inspection reports the object as active", true, inspection.Active);
            Check.Equal("scene: the inspection reports the position", "{x: 3, y: 0.75, z: 3}", inspection.Position);
            Check.Equal("scene: the inspection reports the scale", "{x: 0.75, y: 0.75, z: 0.75}", inspection.Scale);
            Check.Equal("scene: the inspection lists the components", 5, inspection.Components.Count);
            Check.Equal("scene: the first component is the transform", "Transform", inspection.Components[0].Type);

            bool foundPosition = false;
            foreach (SceneProperty property in inspection.Components[0].Properties)
            {
                if (string.Equals(property.Name, "m_LocalPosition", StringComparison.Ordinal))
                {
                    foundPosition = true;
                    Check.Equal(
                        "scene: the transform reports its local position",
                        "{x: 3, y: 0.75, z: 3}",
                        property.Value);
                }
            }

            Check.That("scene: the transform carries the local position", foundPosition);

            bool unknownRefused = false;
            try
            {
                document.Inspect(999999999);
            }
            catch (SceneParseException error)
            {
                unknownRefused = true;
                Check.That("scene: the inspection of an unknown object gives a reason", error.Reason.Length > 0);
            }

            Check.That("scene: the inspection of an unknown object is refused", unknownRefused);
        }

        private static void MalformedHeaderReportsFileAndLine()
        {
            string text = "%YAML 1.1\n"
                + "%TAG !u! tag:unity3d.com,2011:\n"
                + "--- !u!1 &1000000001\n"
                + "GameObject:\n"
                + "  m_Name: Broken\n"
                + "--- !u!1 &not-a-number\n"
                + "GameObject:\n"
                + "  m_Name: Nope\n";
            bool raised = false;
            try
            {
                SceneDocument.Parse("samples/roll-a-ball/Broken.unity", text);
            }
            catch (SceneParseException error)
            {
                raised = true;
                Check.Equal("scene: the parse error names the file", "samples/roll-a-ball/Broken.unity", error.FilePath);
                Check.Equal("scene: the parse error names the line", 6, error.Line);
                Check.That("scene: the parse error gives a reason", error.Reason.Length > 0);
            }
            catch (Exception error)
            {
                Check.That("scene: malformed input raises SceneParseException", error is SceneParseException);
            }

            Check.That("scene: malformed input raises SceneParseException", raised);
        }

        private static void HierarchyMatchesTheSampleScene()
        {
            SceneDocument document = SceneDocument.Open(ResolveRepoPath(RollABallScene));

            SceneObject arena = document.Find(1000000021);
            Check.Equal("scene: the arena keeps its name", "Arena", arena.Name);
            Check.Equal("scene: the arena is a scene root", "0", Format(arena.ParentFileId));
            Check.Equal("scene: the arena has five children", 5, arena.Children.Count);
            Check.Equal("scene: the first arena child is the floor", "ArenaFloor", arena.Children[0].Name);
            Check.Equal("scene: the arena has one component", 1, arena.ComponentFileIds.Count);
            Check.Equal("scene: the arena component is its transform", "1000000022", Format(arena.ComponentFileIds[0]));

            SceneObject pickups = document.Find(1000000081);
            Check.Equal("scene: the pickup parent keeps its name", "Pickups", pickups.Name);
            Check.Equal("scene: the pickup parent is a scene root", "0", Format(pickups.ParentFileId));
            Check.Equal("scene: the pickup parent has four children", 4, pickups.Children.Count);

            SceneObject pickup = document.Find(1000000091);
            Check.Equal("scene: the pickup keeps its name", "Pickup_1", pickup.Name);
            Check.Equal("scene: the pickup knows its parent", "1000000081", Format(pickup.ParentFileId));
            Check.Equal("scene: the pickup has five components", 5, pickup.ComponentFileIds.Count);

            SceneObject ball = document.Find(1000000131);
            Check.Equal("scene: the ball has one child", 1, ball.Children.Count);
            Check.Equal("scene: the ball child is the sensor", "PickupSensor", ball.Children[0].Name);
        }

        private static SceneObject FindObject(SceneDocument document, string name)
        {
            foreach (SceneObject sceneObject in document.Objects)
            {
                if (string.Equals(sceneObject.Name, name, StringComparison.Ordinal))
                {
                    return sceneObject;
                }
            }

            throw new InvalidOperationException("no scene object named " + name);
        }

        private static List<string> FindSceneAssets()
        {
            List<string> paths = new List<string>();
            foreach (string path in Directory.GetFiles(ResolveRepoRoot(), "*", SearchOption.AllDirectories))
            {
                if (IsBuildOutput(path))
                {
                    continue;
                }

                if (path.EndsWith(".unity", StringComparison.Ordinal) || path.EndsWith(".prefab", StringComparison.Ordinal))
                {
                    paths.Add(path);
                }
            }

            paths.Sort(StringComparer.Ordinal);
            return paths;
        }

        private static bool IsBuildOutput(string path)
        {
            foreach (string part in path.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar))
            {
                if (part == "obj" || part == "bin" || part == "target" || part == ".git")
                {
                    return true;
                }
            }

            return false;
        }

        private static string RelativePath(string path)
        {
            string root = ResolveRepoRoot();
            return path.StartsWith(root, StringComparison.Ordinal) ? path.Substring(root.Length + 1) : path;
        }

        private static string Format(long value)
        {
            return value.ToString(CultureInfo.InvariantCulture);
        }

        private static string ResolveRepoRoot()
        {
            DirectoryInfo? directory = new DirectoryInfo(Directory.GetCurrentDirectory());
            for (int depth = 0; depth < 12 && directory != null; depth++)
            {
                if (Directory.Exists(Path.Combine(directory.FullName, "samples", "roll-a-ball")))
                {
                    return directory.FullName;
                }

                directory = directory.Parent;
            }

            throw new FileNotFoundException("repo root not found from current directory");
        }

        private static string ResolveRepoPath(string relativePath)
        {
            return Path.Combine(ResolveRepoRoot(), relativePath);
        }
    }
}
