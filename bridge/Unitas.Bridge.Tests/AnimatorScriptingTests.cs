using System;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class AnimatorScriptingTests
    {
        public static void Run()
        {
            TestStringToHashAndStateInfoHashes();
            TestNullProviderDefaults();
            TestPlayPlayQueuedCrossFade();
            TestRootMotionAndDeltas();
            TestIkGoalsHintsAndLookAt();
        }

        private static void TestStringToHashAndStateInfoHashes()
        {
            int idleHash = Animator.StringToHash("Idle");
            int runHash = Animator.StringToHash("Run");
            Check.That("StringToHash returns non-zero for Idle", idleHash != 0);
            Check.Equal("StringToHash is deterministic", idleHash, Animator.StringToHash("Idle"));
            Check.That("StringToHash distinguishes states", idleHash != runHash);

            AnimatorStateInfo info = new AnimatorStateInfo("Idle", 0, 0.5f, 1f, 1f, 1f, false, Vector3.zero);
            Check.Equal("AnimatorStateInfo shortNameHash matches StringToHash", idleHash, info.shortNameHash);
            Check.Equal("AnimatorStateInfo fullPathHash matches StringToHash(Base Layer.Idle)", Animator.StringToHash("Base Layer.Idle"), info.fullPathHash);
            Check.That("AnimatorStateInfo IsName matches short name", info.IsName("Idle"));
            Check.That("AnimatorStateInfo IsName matches Base Layer.Idle", info.IsName("Base Layer.Idle"));
            Check.That("AnimatorStateInfo IsName rejects other state", !info.IsName("Run"));
        }

        private static void TestNullProviderDefaults()
        {
            IAnimationProvider? saved = Animator.Provider;
            Animator.Provider = null;
            try
            {
                GameObject go = new GameObject("NullAnimatorObj");
                Animator animator = go.AddComponent<Animator>();
                Check.Equal("Default applyRootMotion is false without provider", false, animator.applyRootMotion);
                animator.applyRootMotion = true;
                Check.Equal("applyRootMotion stores true without provider", true, animator.applyRootMotion);
                Check.That("deltaPosition is zero without provider", animator.deltaPosition == Vector3.zero);
                Check.That("deltaRotation is identity without provider", animator.deltaRotation == Quaternion.identity);

                animator.Play("Idle");
                animator.Play(Animator.StringToHash("Idle"));
                animator.PlayQueued("Run");
                animator.CrossFade("Run", 0.25f);
                animator.SetIKPosition(AvatarIKGoal.LeftHand, new Vector3(1f, 2f, 3f));
                animator.SetIKPositionWeight(AvatarIKGoal.LeftHand, 0.5f);
                animator.SetIKRotation(AvatarIKGoal.LeftHand, Quaternion.identity);
                animator.SetIKRotationWeight(AvatarIKGoal.LeftHand, 0.5f);
                animator.SetIKHintPosition(AvatarIKHint.LeftElbow, new Vector3(0f, 1f, 0f));
                animator.SetIKHintPositionWeight(AvatarIKHint.LeftElbow, 0.5f);
                animator.SetLookAtPosition(new Vector3(0f, 1f, 5f));
                animator.SetLookAtWeight(0.8f);

                Check.That("GetIKPosition is zero without provider", animator.GetIKPosition(AvatarIKGoal.LeftHand) == Vector3.zero);
                Check.Equal("GetIKPositionWeight is 0 without provider", 0f, animator.GetIKPositionWeight(AvatarIKGoal.LeftHand));
                Check.That("GetIKRotation is identity without provider", animator.GetIKRotation(AvatarIKGoal.LeftHand) == Quaternion.identity);
                Check.Equal("GetIKRotationWeight is 0 without provider", 0f, animator.GetIKRotationWeight(AvatarIKGoal.LeftHand));
                Check.That("GetIKHintPosition is zero without provider", animator.GetIKHintPosition(AvatarIKHint.LeftElbow) == Vector3.zero);
                Check.Equal("GetIKHintPositionWeight is 0 without provider", 0f, animator.GetIKHintPositionWeight(AvatarIKHint.LeftElbow));
            }
            finally
            {
                Animator.Provider = saved;
            }
        }

        private static void TestPlayPlayQueuedCrossFade()
        {
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    GameObject go = new GameObject("Actor");
                    Animator animator = go.AddComponent<Animator>();
                    animator.runtimeAnimatorController = BuildTwoStateController();

                    host.Tick(0.0);
                    Check.Equal("Starts in Idle", "Idle", animator.GetCurrentAnimatorStateInfo().name);

                    animator.Play("Run");
                    host.Tick(0.1);
                    AnimatorStateInfo runInfo = animator.GetCurrentAnimatorStateInfo();
                    Check.Equal("Play(string) switches to Run", "Run", runInfo.name);
                    Check.That("Play(string) IsName(Run) is true", runInfo.IsName("Run"));

                    animator.Play(Animator.StringToHash("Idle"));
                    host.Tick(0.0);
                    Check.Equal("Play(int hash) switches to Idle", "Idle", animator.GetCurrentAnimatorStateInfo().name);

                    animator.PlayQueued("Run");
                    host.Tick(0.5);
                    Check.Equal("PlayQueued keeps Idle before clip ends", "Idle", animator.GetCurrentAnimatorStateInfo().name);
                    host.Tick(0.6);
                    Check.Equal("PlayQueued advances to Run after clip finishes", "Run", animator.GetCurrentAnimatorStateInfo().name);

                    animator.CrossFade("Idle", 0.5f);
                    host.Tick(0.1);
                    AnimatorStateInfo fadeInfo = animator.GetCurrentAnimatorStateInfo();
                    Check.Equal("CrossFade enters transition", true, fadeInfo.isInTransition);
                    host.Tick(0.5);
                    Check.Equal("CrossFade completes to Idle", "Idle", animator.GetCurrentAnimatorStateInfo().name);

                    host.Shutdown();
                }
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TestRootMotionAndDeltas()
        {
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    GameObject go = new GameObject("RootMotionActor");
                    Animator animator = go.AddComponent<Animator>();
                    animator.runtimeAnimatorController = BuildTwoStateController();

                    Check.Equal("applyRootMotion defaults to true on native animator", true, animator.applyRootMotion);
                    animator.applyRootMotion = false;
                    Check.Equal("applyRootMotion round-trips false", false, animator.applyRootMotion);
                    animator.applyRootMotion = true;
                    Check.Equal("applyRootMotion round-trips true", true, animator.applyRootMotion);

                    host.Tick(0.0);
                    host.Tick(0.2);
                    Vector3 deltaPos = animator.deltaPosition;
                    Quaternion deltaRot = animator.deltaRotation;
                    Check.That("deltaPosition.x reflects root motion step (~2.0)", Math.Abs(deltaPos.x - 2f) < 0.01f);
                    Check.That("deltaRotation.w is near 1 for translation-only clip", Math.Abs(deltaRot.w - 1f) < 0.01f);

                    host.Shutdown();
                }
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TestIkGoalsHintsAndLookAt()
        {
            using (BridgeHost host = new BridgeHost())
            {
                GameObject go = new GameObject("IkActor");
                Animator animator = go.AddComponent<Animator>();
                animator.runtimeAnimatorController = BuildTwoStateController();

                Vector3 goalPos = new Vector3(1.5f, 2.5f, -0.5f);
                Quaternion goalRot = new Quaternion(0f, 0.7071068f, 0f, 0.7071068f);
                Vector3 hintPos = new Vector3(0.5f, 1.25f, -0.25f);
                Vector3 lookPos = new Vector3(0f, 1.7f, 10f);

                animator.SetIKPosition(AvatarIKGoal.RightHand, goalPos);
                animator.SetIKPositionWeight(AvatarIKGoal.RightHand, 0.85f);
                animator.SetIKRotation(AvatarIKGoal.RightHand, goalRot);
                animator.SetIKRotationWeight(AvatarIKGoal.RightHand, 0.65f);
                animator.SetIKHintPosition(AvatarIKHint.RightElbow, hintPos);
                animator.SetIKHintPositionWeight(AvatarIKHint.RightElbow, 0.45f);
                animator.SetLookAtPosition(lookPos);
                animator.SetLookAtWeight(0.9f, 0.25f, 0.75f, 0.1f, 0.5f);

                Vector3 readGoalPos = animator.GetIKPosition(AvatarIKGoal.RightHand);
                Check.Equal("GetIKPosition x", goalPos.x, readGoalPos.x);
                Check.Equal("GetIKPosition y", goalPos.y, readGoalPos.y);
                Check.Equal("GetIKPosition z", goalPos.z, readGoalPos.z);
                Check.Equal("GetIKPositionWeight", 0.85f, animator.GetIKPositionWeight(AvatarIKGoal.RightHand));

                Quaternion readGoalRot = animator.GetIKRotation(AvatarIKGoal.RightHand);
                Check.That("GetIKRotation y matches", Math.Abs(readGoalRot.y - goalRot.y) < 1e-4f);
                Check.That("GetIKRotation w matches", Math.Abs(readGoalRot.w - goalRot.w) < 1e-4f);
                Check.Equal("GetIKRotationWeight", 0.65f, animator.GetIKRotationWeight(AvatarIKGoal.RightHand));

                Vector3 readHintPos = animator.GetIKHintPosition(AvatarIKHint.RightElbow);
                Check.Equal("GetIKHintPosition x", hintPos.x, readHintPos.x);
                Check.Equal("GetIKHintPosition y", hintPos.y, readHintPos.y);
                Check.Equal("GetIKHintPosition z", hintPos.z, readHintPos.z);
                Check.Equal("GetIKHintPositionWeight", 0.45f, animator.GetIKHintPositionWeight(AvatarIKHint.RightElbow));

                host.Shutdown();
            }
        }

        private static AnimatorController BuildTwoStateController()
        {
            AnimationClip idleClip = new AnimationClip("IdleClip");
            idleClip.duration = 1f;
            idleClip.sampleRate = 60f;
            idleClip.wrapMode = AnimationWrapMode.Once;
            idleClip.SetCurve(
                string.Empty,
                AnimationPropertyType.Float,
                "localPosition.x",
                new AnimationCurve(
                    new Keyframe(0f, 0f, 0f, 0f, KeyframeInterpolationMode.Linear),
                    new Keyframe(1f, 10f, 0f, 0f, KeyframeInterpolationMode.Linear)));

            AnimationClip runClip = new AnimationClip("RunClip");
            runClip.duration = 1f;
            runClip.sampleRate = 60f;
            runClip.wrapMode = AnimationWrapMode.Once;
            runClip.SetCurve(
                string.Empty,
                AnimationPropertyType.Float,
                "localPosition.x",
                new AnimationCurve(
                    new Keyframe(0f, 100f, 0f, 0f, KeyframeInterpolationMode.Linear),
                    new Keyframe(1f, 110f, 0f, 0f, KeyframeInterpolationMode.Linear)));

            AnimatorController controller = new AnimatorController("Locomotion");
            controller.AddState("Idle", idleClip);
            controller.AddState("Run", runClip);
            controller.SetEntryState("Idle");
            return controller;
        }
    }
}
