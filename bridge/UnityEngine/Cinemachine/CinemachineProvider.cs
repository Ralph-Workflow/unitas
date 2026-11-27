using UnityEngine;

namespace Cinemachine
{
    public interface ICinemachineProvider
    {
        ulong CreateWorld();

        void DestroyWorld(ulong world);

        ulong AddCamera(ulong world, string name, int priority);

        void SetCameraPriority(ulong world, ulong camera, int priority);

        void SetCameraActive(ulong world, ulong camera, bool active);

        void SetCameraFollow(
            ulong world,
            ulong camera,
            bool hasFollow,
            ulong followTarget,
            Vector3 offset,
            float damping);

        void SetCameraLookAt(ulong world, ulong camera, bool hasLookAt, ulong lookAtTarget, float damping);

        void SetTargetPosition(ulong world, ulong target, Vector3 position);

        void SetDefaultBlend(ulong world, CinemachineBlendStyle style, float duration);

        void SetCustomBlend(
            ulong world,
            string from,
            string to,
            CinemachineBlendStyle style,
            float duration);

        void UpdateWorld(ulong world, double deltaSeconds);

        bool TryReadState(ulong world, out CinemachineBrainState state);
    }

    public static class CinemachineProvider
    {
        public static ICinemachineProvider? Current { get; set; }
    }
}
