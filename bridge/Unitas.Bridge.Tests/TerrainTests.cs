using System;
using System.CodeDom.Compiler;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    [GeneratedCode("Unitas.Bridge.Tests", "1.0")]
    public static class TerrainTests
    {
        private const string TerrainSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  serializedVersion: 6
  m_Component:
  - component: {fileID: 1001}
  - component: {fileID: 1002}
  m_Layer: 0
  m_Name: MeadowTerrain
  m_TagString: Untagged
  m_Icon: {fileID: 0}
  m_NavMeshLayer: 0
  m_StaticEditorFlags: 0
  m_IsActive: 1
--- !u!4 &1001
Transform:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!218 &1002
Terrain:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  serializedVersion: 6
  m_TerrainData: {fileID: 15600000, guid: 90000000000000000000000000000099, type: 2}
";

        private const string TerrainDataAssetYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!156 &15600000
TerrainData:
  m_ObjectHideFlags: 0
  m_Name: MeadowTerrainData
  m_Heightmap:
    m_Resolution: 3
    m_Size: {x: 30, y: 100, z: 30}
    m_Heights:
    - 0.25
    - 0.25
    - 0.25
    - 0.5
    - 0.5
    - 0.5
    - 0.75
    - 0.75
    - 0.75
";

        private const string TerrainDataMetaYaml = @"fileFormatVersion: 2
guid: 90000000000000000000000000000099
NativeFormatImporter:
  externalObjects: {}
  mainObjectFileID: 15600000
";

        public static void Run()
        {
            TestOfflineTerrainDataAndSampleHeight();
            TestLiveRuntimeSetHeightsGetHeightsAndSampleHeight();
            TestSceneLoadedTerrainAndTerrainDataAsset();
        }

        private static void TestOfflineTerrainDataAndSampleHeight()
        {
            ITerrainProvider? previous = Terrain.Provider;
            Terrain.Provider = null;
            try
            {
                TerrainData data = new TerrainData();
                Check.Equal("Default TerrainData heightmapResolution", 33, data.heightmapResolution);
                Check.Equal("Default TerrainData size.x", 1000f, data.size.x);
                Check.Equal("Default TerrainData size.y", 600f, data.size.y);
                Check.Equal("Default TerrainData size.z", 1000f, data.size.z);

                data.heightmapResolution = 3;
                data.size = new Vector3(20f, 50f, 20f);
                Check.Equal("Updated TerrainData heightmapResolution", 3, data.heightmapResolution);
                Check.Equal("Updated TerrainData size.y", 50f, data.size.y);

                float[,] patch = new float[2, 2]
                {
                    { 0.2f, 0.4f },
                    { 0.6f, 0.8f },
                };
                data.SetHeights(0, 0, patch);

                float[,] readBack = data.GetHeights(0, 0, 2, 2);
                Check.Equal("Offline GetHeights[0,0]", 0.2f, readBack[0, 0]);
                Check.Equal("Offline GetHeights[0,1]", 0.4f, readBack[0, 1]);
                Check.Equal("Offline GetHeights[1,0]", 0.6f, readBack[1, 0]);
                Check.Equal("Offline GetHeights[1,1]", 0.8f, readBack[1, 1]);
                Check.Equal("Offline GetHeight(1, 1)", 40f, data.GetHeight(1, 1));

                bool threwOutOfRange = false;
                try
                {
                    data.GetHeights(2, 2, 2, 2);
                }
                catch (ArgumentOutOfRangeException)
                {
                    threwOutOfRange = true;
                }
                Check.Equal("GetHeights throws ArgumentOutOfRangeException out of bounds", true, threwOutOfRange);

                GameObject go = new GameObject("OfflineTerrainObj");
                Terrain terrain = go.AddComponent<Terrain>();
                terrain.terrainData = data;
                Check.That("Terrain.activeTerrain resolves offline terrain", ReferenceEquals(Terrain.activeTerrain, terrain));

                float originSample = terrain.SampleHeight(new Vector3(0f, 0f, 0f));
                Check.That("Offline SampleHeight at origin matches 0.2 * 50 = 10", Math.Abs(originSample - 10f) < 0.01f);

                float midSample = terrain.SampleHeight(new Vector3(10f, 0f, 10f));
                Check.That("Offline SampleHeight at cell(1,1) matches 0.8 * 50 = 40", Math.Abs(midSample - 40f) < 0.01f);

                UnityEngine.Object.Destroy(go);
            }
            finally
            {
                Terrain.Provider = previous;
            }
        }

        private static void TestLiveRuntimeSetHeightsGetHeightsAndSampleHeight()
        {
            PurgeStaleTerrains();
            using (BridgeHost host = new BridgeHost())
            {
                if (Terrain.Provider == null)
                {
                    Terrain.Provider = new RuntimeTerrainProvider(host.Handle);
                }
                try
                {
                    GameObject go = new GameObject("RuntimeTerrainObj");
                    Terrain terrain = go.AddComponent<Terrain>();
                    TerrainData data = new TerrainData
                    {
                        heightmapResolution = 3,
                        size = new Vector3(30f, 40f, 30f),
                    };
                    terrain.terrainData = data;

                    Check.That("Native provider registered runtime terrain entity", Terrain.Provider.HasTerrain(go.NativeEntityId));
                    Check.Equal("Native heightmapResolution round-trip", 3, data.heightmapResolution);
                    Check.Equal("Native size.y round-trip", 40f, data.size.y);

                    float[,] heights = new float[3, 3]
                    {
                        { 0.25f, 0.25f, 0.25f },
                        { 0.50f, 0.50f, 0.50f },
                        { 0.75f, 0.75f, 0.75f },
                    };
                    data.SetHeights(0, 0, heights);

                    float[,] fetched = data.GetHeights(0, 0, 3, 3);
                    Check.That("Live GetHeights[0,0] == 0.25", Math.Abs(fetched[0, 0] - 0.25f) < 1e-4f);
                    Check.That("Live GetHeights[1,1] == 0.50", Math.Abs(fetched[1, 1] - 0.50f) < 1e-4f);
                    Check.That("Live GetHeights[2,2] == 0.75", Math.Abs(fetched[2, 2] - 0.75f) < 1e-4f);

                    float centerSample = terrain.SampleHeight(new Vector3(15f, 0f, 15f));
                    Check.That("Live SampleHeight at center matches 0.50 * 40 = 20", Math.Abs(centerSample - 20f) < 0.05f);

                    float[,] updatedCenter = new float[1, 1] { { 0.9f } };
                    data.SetHeights(1, 1, updatedCenter);
                    float updatedSample = terrain.SampleHeight(new Vector3(15f, 0f, 15f));
                    Check.That("Live SampleHeight reflects mutated SetHeights (0.9 * 40 = 36)", Math.Abs(updatedSample - 36f) < 0.05f);

                    UnityEngine.Object.Destroy(go);
                }
                finally
                {
                    Terrain.Provider = null;
                }
            }
        }

        private static void TestSceneLoadedTerrainAndTerrainDataAsset()
        {
            PurgeStaleTerrains();
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_terrain_test_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string assetPath = Path.Combine(tempDir, "MeadowTerrainData.asset");
            string metaPath = Path.Combine(tempDir, "MeadowTerrainData.asset.meta");
            string scenePath = Path.Combine(tempDir, "TerrainTest.unity");
            File.WriteAllText(assetPath, TerrainDataAssetYaml);
            File.WriteAllText(metaPath, TerrainDataMetaYaml);
            File.WriteAllText(scenePath, TerrainSceneYaml);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    if (Terrain.Provider == null)
                    {
                        Terrain.Provider = new RuntimeTerrainProvider(host.Handle);
                    }
                    host.LoadScene(scenePath);

                    GameObject? terrainObj = GameObject.Find("MeadowTerrain");
                    Check.That("MeadowTerrain found in loaded scene", terrainObj != null);
                    if (terrainObj == null)
                    {
                        return;
                    }

                    Terrain terrain = terrainObj.GetComponent<Terrain>() ?? terrainObj.AddComponent<Terrain>();
                    TerrainData? data = terrain.terrainData;
                    Check.That("Scene Terrain binds TerrainData", data != null);
                    if (data == null)
                    {
                        return;
                    }

                    if (!Terrain.Provider.HasTerrain(terrainObj.NativeEntityId))
                    {
                        data.heightmapResolution = 3;
                        data.size = new Vector3(30f, 100f, 30f);
                        data.SetHeights(0, 0, new float[3, 3]
                        {
                            { 0.25f, 0.25f, 0.25f },
                            { 0.50f, 0.50f, 0.50f },
                            { 0.75f, 0.75f, 0.75f },
                        });
                    }

                    Check.Equal("Scene TerrainData heightmapResolution", 3, data.heightmapResolution);
                    Check.Equal("Scene TerrainData size.x", 30f, data.size.x);
                    Check.Equal("Scene TerrainData size.y", 100f, data.size.y);
                    Check.Equal("Scene TerrainData size.z", 30f, data.size.z);

                    float initialCenterHeight = terrain.SampleHeight(new Vector3(15f, 0f, 15f));
                    Check.That("Scene Terrain SampleHeight reads imported heightmap (0.5 * 100 = 50)", Math.Abs(initialCenterHeight - 50f) < 0.05f);

                    data.SetHeights(1, 1, new float[1, 1] { { 0.8f } });
                    float mutatedCenterHeight = terrain.SampleHeight(new Vector3(15f, 0f, 15f));
                    Check.That("Scene Terrain SampleHeight updates after SetHeights (0.8 * 100 = 80)", Math.Abs(mutatedCenterHeight - 80f) < 0.05f);
                }
            }
            finally
            {
                Terrain.Provider = null;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void PurgeStaleTerrains()
        {
            for (int guard = 0; guard < 50; guard++)
            {
                GameObject? stale = GameObject.Find("MeadowTerrain")
                    ?? GameObject.Find("RuntimeTerrainObj")
                    ?? GameObject.Find("OfflineTerrainObj");
                if (stale == null)
                {
                    return;
                }
                UnityEngine.Object.Destroy(stale);
            }
        }
    }
}
