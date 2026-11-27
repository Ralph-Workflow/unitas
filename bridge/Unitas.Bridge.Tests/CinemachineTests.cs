using System;
using Cinemachine;
using Unitas.Bridge;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class CinemachineTests
    {
        public static int Run()
        {
            int failures = Check.FailureCount;
            RuntimeObjectProvider.Current = null;
            TestHighestPriorityCameraBecomesLive();
            TestBlendWeightProgressesAcrossFrames();
            TestFollowTracksTargetWithOffsetAndDamping();
            TestCustomBlendOverridesDefaultBlend();
            CinemachineProvider.Current = null;
            return Check.FailureCount - failures;
        }

        private static void TestHighestPriorityCameraBecomesLive()
        {
            CinemachineBrain brain = CreateBrain();
            brain.DefaultBlend = CinemachineBlendDefinition.Cut;
            CinemachineVirtualCamera low = brain.AddCamera("Low", 10);
            CinemachineVirtualCamera high = brain.AddCamera("High", 20);
            brain.UpdateBrain(0.016);
            Check.That("Cinemachine highest priority camera is the live one", ReferenceEquals(brain.ActiveCamera, high));
            Check.Equal("Cinemachine live camera reports its name", "High", brain.ActiveCamera?.Name ?? string.Empty);
            high.Active = false;
            brain.UpdateBrain(0.016);
            Check.That(
                "Cinemachine deactivating the live camera falls back to the remaining one",
                ReferenceEquals(brain.ActiveCamera, low));
            Check.Equal("Cinemachine fallback camera reports its name", "Low", brain.ActiveCamera?.Name ?? string.Empty);
        }

        private static void TestBlendWeightProgressesAcrossFrames()
        {
            CinemachineBrain brain = CreateBrain();
            brain.DefaultBlend = CinemachineBlendDefinition.Linear(1f);
            brain.AddCamera("From", 10);
            brain.AddCamera("To", 20);
            brain.UpdateBrain(0.25);
            Check.Equal("Cinemachine blend starts at the outgoing camera", "From", brain.ActiveBlendFrom ?? string.Empty);
            Check.Equal("Cinemachine blend ends at the incoming camera", "To", brain.ActiveBlendTo ?? string.Empty);
            Check.That("Cinemachine blend weight advances a quarter of the blend", Math.Abs(brain.ActiveBlendWeight - 0.25f) < 0.0001f);
            brain.UpdateBrain(0.25);
            Check.That("Cinemachine blend weight keeps advancing", Math.Abs(brain.ActiveBlendWeight - 0.5f) < 0.0001f);
            brain.UpdateBrain(0.5);
            Check.That("Cinemachine blend completes at the end of its duration", brain.ActiveBlendWeight == 0f);
            Check.That("Cinemachine finished blend is no longer reported", brain.ActiveBlendFrom == null);
        }

        private static void TestFollowTracksTargetWithOffsetAndDamping()
        {
            GameObject target = new GameObject("CinemachineTarget");
            target.transform.position = new Vector3(10f, 0f, 0f);
            CinemachineBrain brain = CreateBrain();
            brain.DefaultBlend = CinemachineBlendDefinition.Cut;
            CinemachineVirtualCamera camera = brain.AddCamera("Follow", 10);
            camera.Follow = target.transform;
            camera.LookAt = target.transform;
            camera.FollowOffset = new Vector3(0f, 5f, -10f);
            camera.Damping = 0f;
            brain.UpdateBrain(0.016);
            Check.Equal("Cinemachine follow camera snaps to the target plus offset", 10f, brain.ActiveCameraPosition.x);
            Check.Equal("Cinemachine follow camera applies the offset height", 5f, brain.ActiveCameraPosition.y);
            Check.Equal("Cinemachine follow camera applies the offset depth", -10f, brain.ActiveCameraPosition.z);
            Check.That(
                "Cinemachine look at rotates the camera away from its identity rotation",
                Math.Abs(brain.ActiveCameraRotation.w - 1f) > 0.0001f);

            CinemachineBrain damped = CreateBrain();
            damped.DefaultBlend = CinemachineBlendDefinition.Cut;
            CinemachineVirtualCamera dampedCamera = damped.AddCamera("Damped", 10);
            dampedCamera.Follow = target.transform;
            dampedCamera.FollowOffset = new Vector3(0f, 5f, -10f);
            dampedCamera.Damping = 5f;
            damped.UpdateBrain(0.1);
            Check.That(
                "Cinemachine damped follow lags behind the target",
                damped.ActiveCameraPosition.x < 10f);
            Check.That(
                "Cinemachine damped follow moved toward the target",
                damped.ActiveCameraPosition.x > 0f);
            Check.That(
                "Cinemachine damped follow eases along the damping curve",
                Math.Abs(damped.ActiveCameraPosition.x - 3.9346934f) < 0.001f);
        }

        private static void TestCustomBlendOverridesDefaultBlend()
        {
            CinemachineBrain brain = CreateBrain();
            brain.DefaultBlend = CinemachineBlendDefinition.Linear(2f);
            brain.SetCustomBlend("From", "To", CinemachineBlendDefinition.Linear(0.5f));
            brain.AddCamera("From", 10);
            brain.AddCamera("To", 20);
            brain.UpdateBrain(0.25);
            Check.Equal("Cinemachine custom blend runs from the outgoing camera", "From", brain.ActiveBlendFrom ?? string.Empty);
            Check.Equal("Cinemachine custom blend runs to the incoming camera", "To", brain.ActiveBlendTo ?? string.Empty);
            Check.That(
                "Cinemachine custom blend overrides the default blend duration",
                Math.Abs(brain.ActiveBlendWeight - 0.5f) < 0.0001f);
        }

        private static CinemachineBrain CreateBrain()
        {
            CinemachineProvider.Current = new RuntimeCinemachineProvider();
            return new CinemachineBrain();
        }
    }
}
