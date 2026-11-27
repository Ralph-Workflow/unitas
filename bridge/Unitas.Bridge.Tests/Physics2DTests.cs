using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class Physics2DRecorderScript : MonoBehaviour
    {
        private readonly List<string> _events = new List<string>();
        private int _frame;

        public IReadOnlyList<string> Events => _events;

        public string ObservedOrder => string.Join(", ", _events);

        public int CollisionContactCount { get; private set; }

        public string CollisionOtherName { get; private set; } = string.Empty;

        public ContactPoint2D CollisionContact { get; private set; }

        public override void Update()
        {
            _frame++;
            _events.Add("Update@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        public override void OnCollisionEnter(Collision collision)
        {
            _events.Add("OnCollisionEnter@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        public override void OnCollisionEnter2D(Collision2D collision)
        {
            _events.Add("OnCollisionEnter2D@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
            CollisionContactCount = collision.contactCount;
            CollisionOtherName = collision.gameObject?.name ?? string.Empty;
            if (collision.contactCount > 0)
            {
                CollisionContact = collision.GetContact(0);
            }
        }

        public override void OnTriggerEnter(Collider? other)
        {
            _events.Add("OnTriggerEnter@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        public override void OnTriggerEnter2D(Collider2D? other)
        {
            _events.Add("OnTriggerEnter2D@" + _frame.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }
    }

    public static class Physics2DTests
    {
        public static void Run()
        {
            TestTwoDimensionalContactsReachTheTwoDimensionalHandlers();
            TestBoxCastReachesTheCollidersItSweeps();
            TestRigidbodyConstraintsReachTheNativeBody();
            TestCapsuleColliderCreatesANativeCapsuleShape();
            TestSceneParsesExtendedCollidersAndMaterials();
            TestSceneParsesInlineTilemapCollider();
            TestLayerCollisionMatrixRoundTripsThroughProvider();
        }

        private static void TestTwoDimensionalContactsReachTheTwoDimensionalHandlers()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");
            Check.That("Physics2D scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            Physics2DRecorderScript recorded = new Physics2DRecorderScript();
            ScriptGlueRegistry.Register(0x2d2d2d2d2d2d2d2dUL, 0x2d2d2d2d2d2d2d2dUL, () => recorded);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            IReadOnlyList<string> events = recorded.Events;
            Console.WriteLine("Observed 2D order: " + recorded.ObservedOrder);

            int triggerIndex = IndexOfEvent(events, "OnTriggerEnter2D");
            int collisionIndex = IndexOfEvent(events, "OnCollisionEnter2D");

            Check.That("2D trigger reaches the OnTriggerEnter2D handler", triggerIndex >= 0);
            Check.That("2D collision reaches the OnCollisionEnter2D handler", collisionIndex >= 0);
            Check.That("2D trigger never reaches the 3D OnTriggerEnter handler", IndexOfEvent(events, "OnTriggerEnter") < 0);
            Check.That("2D collision never reaches the 3D OnCollisionEnter handler", IndexOfEvent(events, "OnCollisionEnter") < 0);
            Check.That(
                "2D trigger reaches the handler before the 2D collision",
                triggerIndex >= 0 && collisionIndex > triggerIndex);
            Check.That(
                "2D trigger is dispatched after the physics step and before the frame update",
                IsDispatchedBeforeTheNextUpdate(events, triggerIndex));
            Check.That(
                "2D collision is dispatched after the physics step and before the frame update",
                IsDispatchedBeforeTheNextUpdate(events, collisionIndex));
            Check.That(
                "2D collision carries the contact points of the 2D world",
                recorded.CollisionContactCount > 0);
            Check.That(
                "2D collision resolves the other game object",
                recorded.CollisionOtherName == "Crate2D");
            Check.That(
                "2D collision contact point sits on the crate surface",
                Math.Abs(recorded.CollisionContact.point.y - 0.5f) < 0.25f);
        }

        private static void TestBoxCastReachesTheCollidersItSweeps()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);

                RaycastHit2D[] crateHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D());
                Check.Equal("the box cast reports the crate it reaches", 1, crateHits.Length);
                Check.Equal("the box cast hit resolves the crate collider", "Crate2D", HitName(crateHits, 0));
                Check.That(
                    "the box cast stops when the box reaches the crate face",
                    crateHits.Length > 0 && Math.Abs(crateHits[0].distance - 2.75f) < 0.05f);
                Check.That(
                    "the box cast normal points back at the caster",
                    crateHits.Length > 0 && crateHits[0].normal.x < -0.99f && Math.Abs(crateHits[0].normal.y) < 0.01f);

                ContactFilter2D sideFilter = new ContactFilter2D
                {
                    useOutsideNormalAngle = true,
                    minNormalAngle = 90f,
                    maxNormalAngle = 270f,
                };
                RaycastHit2D[] sideHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    sideFilter);
                Check.Equal("a normal-angle filter that holds the hit normal keeps the hit", 1, sideHits.Length);

                ContactFilter2D upwardFilter = new ContactFilter2D
                {
                    useOutsideNormalAngle = true,
                    minNormalAngle = 0f,
                    maxNormalAngle = 90f,
                };
                RaycastHit2D[] upwardHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    upwardFilter);
                Check.Equal("a normal-angle filter that excludes the hit normal drops the hit", 0, upwardHits.Length);

                RaycastHit2D[] unfilteredHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D());
                Check.Equal("a filter without a normal angle keeps the hit", 1, unfilteredHits.Length);

                RaycastHit2D[] maskedHits = Physics2D.BoxCast(
                    new Vector2(-5f, 0f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D(),
                    0);
                Check.Equal("a layer mask that excludes the crate layer drops the hit", 0, maskedHits.Length);

                ContactFilter2D triggerFilter = new ContactFilter2D
                {
                    useTriggers = true,
                };
                RaycastHit2D[] triggerHits = Physics2D.BoxCast(
                    new Vector2(-5f, 1.5f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    triggerFilter);
                Check.Equal("a trigger-inclusive filter reports the trigger collider", 1, triggerHits.Length);
                Check.Equal("the trigger hit resolves the trigger collider", "Trigger2D", HitName(triggerHits, 0));

                RaycastHit2D[] solidHits = Physics2D.BoxCast(
                    new Vector2(-5f, 1.5f),
                    new Vector2(0.5f, 0.5f),
                    0f,
                    new Vector2(1f, 0f),
                    20f,
                    new ContactFilter2D());
                Check.Equal("a trigger-exclusive filter drops the trigger collider", 0, solidHits.Length);
            }
        }

        private static void TestRigidbodyConstraintsReachTheNativeBody()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                Rigidbody2D? body = FindSceneRigidbody2D();
                Check.That("the scene rigidbody is materialized", body != null);
                if (body == null)
                {
                    return;
                }

                body.freezeRotation = true;
                Check.That("freezeRotation reads back as frozen", body.freezeRotation);
                Check.Equal(
                    "the freeze-rotation constraint carries the Unity bit",
                    4,
                    (int)(body.constraints & RigidbodyConstraints2D.FreezeRotation));
                Check.That(
                    "the native body accepts the freeze-rotation constraint",
                    Physics2D.Provider != null
                    && Physics2D.Provider.TrySetBodyConstraints2D(
                        body.gameObject.NativeEntityId,
                        RigidbodyConstraints2D.FreezeRotation));

                body.constraints = RigidbodyConstraints2D.None;
                Check.That("clearing the constraints clears freezeRotation", !body.freezeRotation);

                body.constraints = RigidbodyConstraints2D.FreezeAll;
                Check.That("the freeze-all constraints freeze the rotation", body.freezeRotation);
                Check.That(
                    "the freeze-all constraints keep the position bits",
                    (body.constraints & RigidbodyConstraints2D.FreezePosition) == RigidbodyConstraints2D.FreezePosition);
                Check.That(
                    "the freeze-all constraints reach the native body",
                    Physics2D.Provider != null
                    && Physics2D.Provider.TrySetBodyConstraints2D(
                        body.gameObject.NativeEntityId,
                        RigidbodyConstraints2D.FreezeAll));
            }
        }

        private static void TestCapsuleColliderCreatesANativeCapsuleShape()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                Rigidbody2D? body = FindSceneRigidbody2D();
                Check.That("the scene rigidbody is materialized", body != null);
                if (body == null)
                {
                    return;
                }

                Check.Equal(
                    "the box collider of the ball does not reach the capsule cap",
                    0,
                    Physics2D.OverlapPoint(new Vector2(0f, 3.8f)).Length);

                CapsuleCollider2D? capsule = body.gameObject.AddComponent<CapsuleCollider2D>();
                Check.That("the capsule collider is attached to the ball", capsule != null);
                if (capsule == null)
                {
                    return;
                }
                capsule.size = new Vector2(0.5f, 2f);

                Check.Equal(
                    "the native capsule collider reaches the capsule cap",
                    1,
                    Physics2D.OverlapPoint(new Vector2(0f, 3.8f)).Length);
                Check.Equal(
                    "the native capsule collider does not widen the ball",
                    0,
                    Physics2D.OverlapPoint(new Vector2(0.4f, 3.8f)).Length);
            }
        }

        private static void TestSceneParsesExtendedCollidersAndMaterials()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_u2_scene_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            Directory.CreateDirectory(Path.Combine(tempDir, "ProjectSettings"));
            string assetsDir = Path.Combine(tempDir, "Assets");
            Directory.CreateDirectory(assetsDir);

            try
            {
                string matGuid = "11111111111111111111111111111111";
                string matPath = Path.Combine(assetsDir, "Bouncy2D.physicsMaterial2D");
                File.WriteAllText(
                    matPath,
                    "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!62 &6200000\nPhysicsMaterial2D:\n  m_Name: Bouncy2D\n  friction: 0.25\n  bounciness: 0.85\n");
                File.WriteAllText(
                    matPath + ".meta",
                    "fileFormatVersion: 2\nguid: " + matGuid + "\n");

                string tileGuid = "22222222222222222222222222222222";
                string tilePath = Path.Combine(assetsDir, "GroundTile.asset");
                File.WriteAllText(
                    tilePath,
                    "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nTile:\n  m_Name: GroundTile\n  m_Sprite: {fileID: 21300000, guid: 33333333333333333333333333333333, type: 3}\n  m_ColliderType: 2\n");
                File.WriteAllText(
                    tilePath + ".meta",
                    "fileFormatVersion: 2\nguid: " + tileGuid + "\n");

                string scenePath = Path.Combine(assetsDir, "Extended2D.unity");
                string sceneYaml = string.Join(
                    "\n",
                    new string[]
                    {
                        "%YAML 1.1",
                        "%TAG !u! tag:unity3d.com,2011:",
                        "--- !u!1 &100",
                        "GameObject:",
                        "  m_Name: CapsuleObj",
                        "  m_Layer: 0",
                        "  m_IsActive: 1",
                        "--- !u!4 &101",
                        "Transform:",
                        "  m_GameObject: {fileID: 100}",
                        "  m_LocalPosition: {x: -10, y: 0, z: 0}",
                        "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                        "  m_LocalScale: {x: 1, y: 1, z: 1}",
                        "--- !u!70 &102",
                        "CapsuleCollider2D:",
                        "  m_GameObject: {fileID: 100}",
                        "  m_Material: {fileID: 6200000, guid: " + matGuid + ", type: 2}",
                        "  m_IsTrigger: 0",
                        "  m_UsedByEffector: 1",
                        "  m_Offset: {x: 0, y: 0}",
                        "  m_Size: {x: 1, y: 3}",
                        "  m_Direction: 0",
                        "--- !u!1 &200",
                        "GameObject:",
                        "  m_Name: PolyObj",
                        "  m_Layer: 0",
                        "  m_IsActive: 1",
                        "--- !u!4 &201",
                        "Transform:",
                        "  m_GameObject: {fileID: 200}",
                        "  m_LocalPosition: {x: 0, y: 0, z: 0}",
                        "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                        "  m_LocalScale: {x: 1, y: 1, z: 1}",
                        "--- !u!60 &202",
                        "PolygonCollider2D:",
                        "  m_GameObject: {fileID: 200}",
                        "  m_IsTrigger: 0",
                        "  m_Friction: 0.6",
                        "  m_Bounciness: 0.3",
                        "  m_Offset: {x: 0, y: 0}",
                        "  m_Points:",
                        "    m_Paths:",
                        "    - - {x: -1, y: -1}",
                        "      - {x: 1, y: -1}",
                        "      - {x: 0, y: 1}",
                        "--- !u!1 &300",
                        "GameObject:",
                        "  m_Name: EdgeObj",
                        "  m_Layer: 0",
                        "  m_IsActive: 1",
                        "--- !u!4 &301",
                        "Transform:",
                        "  m_GameObject: {fileID: 300}",
                        "  m_LocalPosition: {x: 10, y: 0, z: 0}",
                        "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                        "  m_LocalScale: {x: 1, y: 1, z: 1}",
                        "--- !u!68 &302",
                        "EdgeCollider2D:",
                        "  m_GameObject: {fileID: 300}",
                        "  m_IsTrigger: 0",
                        "  m_EdgeRadius: 0.15",
                        "  m_Offset: {x: 0, y: 0}",
                        "  m_Points:",
                        "  - {x: -2, y: 0}",
                        "  - {x: 0, y: 1}",
                        "  - {x: 2, y: 0}",
                        "--- !u!1 &400",
                        "GameObject:",
                        "  m_Name: CompositeObj",
                        "  m_Layer: 0",
                        "  m_IsActive: 1",
                        "--- !u!4 &401",
                        "Transform:",
                        "  m_GameObject: {fileID: 400}",
                        "  m_LocalPosition: {x: 20, y: 0, z: 0}",
                        "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                        "  m_LocalScale: {x: 1, y: 1, z: 1}",
                        "--- !u!50 &402",
                        "Rigidbody2D:",
                        "  m_GameObject: {fileID: 400}",
                        "  m_BodyType: 2",
                        "  m_Simulated: 1",
                        "--- !u!66 &403",
                        "CompositeCollider2D:",
                        "  m_GameObject: {fileID: 400}",
                        "  m_GeometryType: 1",
                        "  m_GenerationType: 0",
                        "  m_VertexDistance: 0.001",
                        "  m_OffsetDistance: 0.002",
                        "  m_EdgeRadius: 0.05",
                        "--- !u!61 &404",
                        "BoxCollider2D:",
                        "  m_GameObject: {fileID: 400}",
                        "  m_UsedByComposite: 1",
                        "  m_Offset: {x: 0, y: 0}",
                        "  m_Size: {x: 2, y: 2}",
                        "--- !u!1 &500",
                        "GameObject:",
                        "  m_Name: TilemapObj",
                        "  m_Layer: 0",
                        "  m_IsActive: 1",
                        "--- !u!4 &501",
                        "Transform:",
                        "  m_GameObject: {fileID: 500}",
                        "  m_LocalPosition: {x: 30, y: 0, z: 0}",
                        "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                        "  m_LocalScale: {x: 1, y: 1, z: 1}",
                        "--- !u!19719996 &502",
                        "TilemapCollider2D:",
                        "  m_GameObject: {fileID: 500}",
                        "  m_MaximumTileChangeCount: 500",
                        "  m_ExtrusionFactor: 0.25",
                        "  m_TilemapAsset: {fileID: 11400000, guid: " + tileGuid + ", type: 2}",
                        string.Empty,
                    });
                File.WriteAllText(scenePath, sceneYaml);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    AssertExtendedSceneCollidersMaterialized();
                }
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void AssertExtendedSceneCollidersMaterialized()
        {
            GameObject? capsuleGo = GameObject.Find("CapsuleObj");
            Check.That("CapsuleObj is materialized from the scene", capsuleGo != null);
            CapsuleCollider2D? capsule = capsuleGo?.GetComponent<CapsuleCollider2D>();
            Check.That("CapsuleCollider2D is attached to CapsuleObj", capsule != null);
            if (capsule != null)
            {
                Check.Equal("capsule size X parsed", 1f, capsule.size.x);
                Check.Equal("capsule size Y parsed", 3f, capsule.size.y);
                Check.Equal("capsule direction parsed", 0, capsule.direction);
                Check.That("capsule usedByEffector parsed", capsule.usedByEffector);
                Check.That("capsule sharedMaterial resolved from GUID", capsule.sharedMaterial != null);
                if (capsule.sharedMaterial != null)
                {
                    Check.Equal("material name parsed", "Bouncy2D", capsule.sharedMaterial.name);
                    Check.That("material friction parsed", Math.Abs(capsule.sharedMaterial.friction - 0.25f) < 0.001f);
                    Check.That("material bounciness parsed", Math.Abs(capsule.sharedMaterial.bounciness - 0.85f) < 0.001f);
                }
            }

            GameObject? polyGo = GameObject.Find("PolyObj");
            Check.That("PolyObj is materialized from the scene", polyGo != null);
            PolygonCollider2D? poly = polyGo?.GetComponent<PolygonCollider2D>();
            Check.That("PolygonCollider2D is attached to PolyObj", poly != null);
            if (poly != null)
            {
                Check.Equal("polygon vertex count parsed", 3, poly.GetTotalPointCount());
                Check.That("polygon inline material parsed", poly.sharedMaterial != null);
                if (poly.sharedMaterial != null)
                {
                    Check.That("polygon inline friction parsed", Math.Abs(poly.sharedMaterial.friction - 0.6f) < 0.001f);
                    Check.That("polygon inline bounciness parsed", Math.Abs(poly.sharedMaterial.bounciness - 0.3f) < 0.001f);
                }
                Collider2D[] polyHits = Physics2D.OverlapPoint(new Vector2(0f, 0f));
                Check.Equal("native polygon collider overlaps interior point", 1, polyHits.Length);
            }

            GameObject? edgeGo = GameObject.Find("EdgeObj");
            Check.That("EdgeObj is materialized from the scene", edgeGo != null);
            EdgeCollider2D? edge = edgeGo?.GetComponent<EdgeCollider2D>();
            Check.That("EdgeCollider2D is attached to EdgeObj", edge != null);
            if (edge != null)
            {
                Check.Equal("edge point count parsed", 3, edge.pointCount);
                Check.Equal("edge segment count parsed", 2, edge.edgeCount);
                Check.That("edge radius parsed", Math.Abs(edge.edgeRadius - 0.15f) < 0.001f);
            }

            GameObject? compGo = GameObject.Find("CompositeObj");
            Check.That("CompositeObj is materialized from the scene", compGo != null);
            CompositeCollider2D? composite = compGo?.GetComponent<CompositeCollider2D>();
            BoxCollider2D? compBox = compGo?.GetComponent<BoxCollider2D>();
            Check.That("CompositeCollider2D is attached to CompositeObj", composite != null);
            Check.That("BoxCollider2D is attached to CompositeObj", compBox != null);
            if (composite != null && compBox != null)
            {
                Check.Equal("composite geometryType parsed", (int)CompositeCollider2D.GeometryType.Polygons, (int)composite.geometryType);
                Check.Equal("composite generationType parsed", (int)CompositeCollider2D.GenerationType.Synchronous, (int)composite.generationType);
                Check.That("composite vertexDistance parsed", Math.Abs(composite.vertexDistance - 0.001f) < 0.0001f);
                Check.That("composite offsetDistance parsed", Math.Abs(composite.offsetDistance - 0.002f) < 0.0001f);
                Check.That("composite edgeRadius parsed", Math.Abs(composite.edgeRadius - 0.05f) < 0.0001f);
                Check.That("part collider usedByComposite parsed", compBox.usedByComposite);
                Check.That("part collider composite links to CompositeCollider2D", ReferenceEquals(compBox.composite, composite));
            }

            GameObject? tilemapGo = GameObject.Find("TilemapObj");
            Check.That("TilemapObj is materialized from the scene", tilemapGo != null);
            TilemapCollider2D? tilemap = tilemapGo?.GetComponent<TilemapCollider2D>();
            Check.That("TilemapCollider2D is attached to TilemapObj", tilemap != null);
            if (tilemap != null)
            {
                Check.Equal("tilemap maximumTileChangeCount parsed", 500u, tilemap.maximumTileChangeCount);
                Check.That("tilemap extrusionFactor parsed", Math.Abs(tilemap.extrusionFactor - 0.25f) < 0.001f);
                Check.That("tilemap asset path resolved from GUID", tilemap.tilemapAssetPath.EndsWith("GroundTile.asset", StringComparison.Ordinal));
            }
        }

        private static void TestSceneParsesInlineTilemapCollider()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_u2_tilemap_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);

            try
            {
                string scenePath = Path.Combine(tempDir, "InlineTilemap.unity");
                string sceneYaml = string.Join(
                    "\n",
                    new string[]
                    {
                        "%YAML 1.1",
                        "%TAG !u! tag:unity3d.com,2011:",
                        "--- !u!1 &600",
                        "GameObject:",
                        "  m_Name: InlineTilemapObj",
                        "  m_Layer: 0",
                        "  m_IsActive: 1",
                        "--- !u!4 &601",
                        "Transform:",
                        "  m_GameObject: {fileID: 600}",
                        "  m_LocalPosition: {x: 0, y: 0, z: 0}",
                        "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                        "  m_LocalScale: {x: 1, y: 1, z: 1}",
                        "--- !u!1839735485 &602",
                        "Tilemap:",
                        "  m_GameObject: {fileID: 600}",
                        "  m_Tiles:",
                        "  - m_X: 0",
                        "    m_Y: 0",
                        "    m_Z: 0",
                        "    m_TileIndex: 0",
                        "  m_TileAssetArray: []",
                        "--- !u!19719996 &603",
                        "TilemapCollider2D:",
                        "  m_GameObject: {fileID: 600}",
                        "  m_MaximumTileChangeCount: 1000",
                        "  m_ExtrusionFactor: 0",
                        string.Empty,
                    });
                File.WriteAllText(scenePath, sceneYaml);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    GameObject? go = GameObject.Find("InlineTilemapObj");
                    Check.That("InlineTilemapObj is materialized", go != null);
                    TilemapCollider2D? tilemap = go?.GetComponent<TilemapCollider2D>();
                    Check.That("TilemapCollider2D is attached to InlineTilemapObj", tilemap != null);
                    if (tilemap != null)
                    {
                        Check.That("inline Tilemap YAML is captured on TilemapCollider2D", tilemap.tilemapAssetPath.Contains("Tilemap:", StringComparison.Ordinal));
                    }
                    Collider2D[] hits = Physics2D.OverlapPoint(new Vector2(0.5f, 0.5f));
                    Check.Equal("native tilemap collider built from inline Tilemap YAML overlaps cell (0,0)", 1, hits.Length);
                }
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestLayerCollisionMatrixRoundTripsThroughProvider()
        {
            string scenePath = ResolveFixture("bridge/Unitas.Bridge.Tests/Fixtures/scene_physics_2d.unity");

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);

                Check.That("layers 1 and 2 collide by default", !Physics2D.GetIgnoreLayerCollision(1, 2));
                Physics2D.IgnoreLayerCollision(1, 2, true);
                Check.That("ignoring collision between layers 1 and 2 reads back as ignored", Physics2D.GetIgnoreLayerCollision(1, 2));
                Check.That("ignoring collision is symmetric for layers 2 and 1", Physics2D.GetIgnoreLayerCollision(2, 1));
                Check.Equal("layer 1 collision mask clears bit 2", 0, Physics2D.GetLayerCollisionMask(1) & (1 << 2));

                Physics2D.IgnoreLayerCollision(1, 2, false);
                Check.That("re-enabling collision between layers 1 and 2 clears ignore flag", !Physics2D.GetIgnoreLayerCollision(1, 2));
                Check.That("layer 1 collision mask sets bit 2", (Physics2D.GetLayerCollisionMask(1) & (1 << 2)) != 0);

                int originalMask = Physics2D.GetLayerCollisionMask(3);
                int customMask = originalMask & ~(1 << 4);
                Physics2D.SetLayerCollisionMask(3, customMask);
                Check.That("SetLayerCollisionMask disables collision with cleared layer bit", Physics2D.GetIgnoreLayerCollision(3, 4));
                Physics2D.SetLayerCollisionMask(3, originalMask);
                Check.That("restoring layer collision mask re-enables collision", !Physics2D.GetIgnoreLayerCollision(3, 4));
            }
        }

        private static Rigidbody2D? FindSceneRigidbody2D()
        {
            Collider2D[] colliders = Physics2D.OverlapPoint(new Vector2(0f, 3f));
            for (int index = 0; index < colliders.Length; index++)
            {
                Rigidbody2D? body = colliders[index].gameObject.GetComponent<Rigidbody2D>();
                if (body != null)
                {
                    return body;
                }
            }
            return null;
        }

        private static string HitName(RaycastHit2D[] hits, int index)
        {
            if (index >= hits.Length)
            {
                return string.Empty;
            }
            Collider2D? collider = hits[index].collider;
            return collider == null ? string.Empty : collider.gameObject.name;
        }

        private static int IndexOfEvent(IReadOnlyList<string> events, string name)
        {
            for (int index = 0; index < events.Count; index++)
            {
                if (events[index].StartsWith(name + "@", StringComparison.Ordinal))
                {
                    return index;
                }
            }
            return -1;
        }

        private static bool IsDispatchedBeforeTheNextUpdate(IReadOnlyList<string> events, int eventIndex)
        {
            if (eventIndex < 0 || eventIndex + 1 >= events.Count)
            {
                return false;
            }
            int frame = FrameOf(events[eventIndex]);
            if (frame < 0)
            {
                return false;
            }
            string next = events[eventIndex + 1];
            if (!next.StartsWith("Update@", StringComparison.Ordinal))
            {
                return false;
            }
            return FrameOf(next) == frame + 1;
        }

        private static int FrameOf(string entry)
        {
            int separator = entry.LastIndexOf('@');
            if (separator < 0 || separator + 1 >= entry.Length)
            {
                return -1;
            }
            return int.TryParse(
                entry.AsSpan(separator + 1),
                System.Globalization.NumberStyles.Integer,
                System.Globalization.CultureInfo.InvariantCulture,
                out int frame)
                ? frame
                : -1;
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
