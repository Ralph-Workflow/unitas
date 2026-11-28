using System;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class ParticleSystemTests
    {
        private const string ParticleSceneYaml = @"%YAML 1.1
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
  m_Name: EmitterObject
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
--- !u!198 &1002
ParticleSystem:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 1000}
  serializedVersion: 2
  m_Duration: 5
  m_Loop: 1
  m_MaxParticles: 64
  m_StartLifetime: 2
  m_StartSpeed: 0
  m_StartSize: 0.25
  m_StartColor:
    r: 1
    g: 0.5
    b: 0
    a: 1
  m_EmissionRate: 10
  m_Bursts: []
";

        public static void Run()
        {
            TestNullProviderBehavior();
            TestLiveRuntimeProviderAndSceneParticleSystem();
            TestRuntimeAttachedParticleSystemAndSimulate();
        }

        private static void TestNullProviderBehavior()
        {
            IParticleSystemProvider? previous = ParticleSystem.Provider;
            ParticleSystem.Provider = null;
            try
            {
                GameObject go = new GameObject("OfflineParticles");
                ParticleSystem ps = go.AddComponent<ParticleSystem>();
                Check.Equal("Offline initial isPlaying", false, ps.isPlaying);
                Check.Equal("Offline initial isPaused", false, ps.isPaused);
                Check.Equal("Offline initial isStopped", true, ps.isStopped);
                Check.Equal("Offline initial particleCount", 0, ps.particleCount);

                ps.Play();
                Check.Equal("Offline Play sets isPlaying", true, ps.isPlaying);
                Check.Equal("Offline Play clears isPaused", false, ps.isPaused);
                Check.Equal("Offline Play clears isStopped", false, ps.isStopped);

                ps.Emit(5);
                Check.Equal("Offline Emit(5) raises particleCount", 5, ps.particleCount);

                ps.Pause();
                Check.Equal("Offline Pause clears isPlaying", false, ps.isPlaying);
                Check.Equal("Offline Pause sets isPaused", true, ps.isPaused);

                ps.Stop(ParticleSystemStopBehavior.StopEmitting);
                Check.Equal("Offline Stop(StopEmitting) keeps particleCount", 5, ps.particleCount);
                Check.Equal("Offline Stop sets isStopped", true, ps.isStopped);

                ps.Stop(ParticleSystemStopBehavior.StopEmittingAndClear);
                Check.Equal("Offline Stop(StopEmittingAndClear) clears particleCount", 0, ps.particleCount);

                ps.Emit(3);
                ps.Clear();
                Check.Equal("Offline Clear resets particleCount", 0, ps.particleCount);

                ps.Simulate(0.5f);
                Check.Equal("Offline Simulate sets isPaused", true, ps.isPaused);
                UnityEngine.Object.Destroy(go);
            }
            finally
            {
                ParticleSystem.Provider = previous;
            }
        }

        private static void TestLiveRuntimeProviderAndSceneParticleSystem()
        {
            PurgeStaleEmitters();
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_particle_test_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "ParticleTest.unity");
            File.WriteAllText(scenePath, ParticleSceneYaml);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    if (ParticleSystem.Provider == null)
                    {
                        ParticleSystem.Provider = new RuntimeParticleSystemProvider(host.Handle);
                    }
                    host.LoadScene(scenePath);

                    GameObject? emitterObj = GameObject.Find("EmitterObject");
                    Check.That("EmitterObject found in loaded scene", emitterObj != null);
                    if (emitterObj == null)
                    {
                        return;
                    }

                    ParticleSystem ps = emitterObj.GetComponent<ParticleSystem>() ?? emitterObj.AddComponent<ParticleSystem>();
                    Check.Equal("Scene ParticleSystem starts playing", true, ps.isPlaying);
                    Check.Equal("Scene ParticleSystem not paused initially", false, ps.isPaused);
                    Check.Equal("Scene ParticleSystem initial particleCount", 0, ps.particleCount);

                    ps.Emit(5);
                    Check.Equal("Emit(5) raises particleCount to 5", 5, ps.particleCount);

                    ps.Pause();
                    Check.Equal("Pause sets isPaused", true, ps.isPaused);
                    Check.Equal("Pause clears isPlaying", false, ps.isPlaying);

                    ps.Play();
                    Check.Equal("Play resumes isPlaying", true, ps.isPlaying);
                    Check.Equal("Play clears isPaused", false, ps.isPaused);

                    ps.Stop(ParticleSystemStopBehavior.StopEmitting);
                    Check.Equal("Stop(StopEmitting) stops playing", false, ps.isPlaying);
                    Check.Equal("Stop(StopEmitting) keeps 5 emitted particles", 5, ps.particleCount);

                    ps.Stop(ParticleSystemStopBehavior.StopEmittingAndClear);
                    Check.Equal("Stop(StopEmittingAndClear) clears particles", 0, ps.particleCount);

                    ps.Simulate(0.5f, true, true);
                    Check.Equal("Simulate(0.5f) at 10/s spawns 5 particles", 5, ps.particleCount);
                    Check.Equal("Simulate leaves system paused", true, ps.isPaused);

                    ps.Clear();
                    Check.Equal("Clear drops all particles", 0, ps.particleCount);
                }
            }
            finally
            {
                ParticleSystem.Provider = null;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestRuntimeAttachedParticleSystemAndSimulate()
        {
            using (BridgeHost host = new BridgeHost())
            {
                if (ParticleSystem.Provider == null)
                {
                    ParticleSystem.Provider = new RuntimeParticleSystemProvider(host.Handle);
                }
                try
                {
                    GameObject go = new GameObject("RuntimeEmitter");
                    ParticleSystem ps = go.AddComponent<ParticleSystem>();
                    ps.Emit(4);
                    Check.Equal("Runtime-attached ParticleSystem Emit(4)", 4, ps.particleCount);
                    ps.Simulate(0.5f, false, false);
                    Check.Equal("Runtime-attached Simulate(0.5f, restart:false) keeps 4 particles alive (1s lifetime)", 4, ps.particleCount);
                    ps.Simulate(0.6f, false, false);
                    Check.Equal("Runtime-attached Simulate past 1s lifetime expires particles", 0, ps.particleCount);
                    UnityEngine.Object.Destroy(go);
                }
                finally
                {
                    ParticleSystem.Provider = null;
                }
            }
        }

        private static void PurgeStaleEmitters()
        {
            for (int guard = 0; guard < 50; guard++)
            {
                GameObject? stale = GameObject.Find("EmitterObject");
                if (stale == null)
                {
                    return;
                }
                UnityEngine.Object.Destroy(stale);
            }
        }
    }
}
