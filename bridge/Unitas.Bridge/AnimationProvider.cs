using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed class RuntimeAnimationProvider : IAnimationProvider
    {
        private const int StateInfoSize = 80;
        private const int StateNameCapacity = 256;
        private const int ValueSize = 4;
        private const int Vector3Size = 12;
        private const int QuaternionSize = 16;
        private const int RootMotionSize = 28;

        private readonly ulong _runtimeHandle;

        public RuntimeAnimationProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public ulong AttachAnimator(ulong entityId)
        {
            if (_runtimeHandle == 0UL)
            {
                return 0UL;
            }

            return RuntimeInterop.unitas_runtime_animator_attach(_runtimeHandle, entityId);
        }

        public ulong CreateController(AnimatorController controller)
        {
            if (_runtimeHandle == 0UL || controller == null)
            {
                return 0UL;
            }

            byte[] encoded = AnimatorControllerEncoder.Encode(controller);
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)encoded.Length);
            if (buffer == IntPtr.Zero)
            {
                return 0UL;
            }

            Marshal.Copy(encoded, 0, buffer, encoded.Length);
            return RuntimeInterop.unitas_runtime_animator_controller_create(_runtimeHandle, buffer, (ulong)encoded.Length);
        }

        public void DestroyController(ulong controllerHandle)
        {
            if (_runtimeHandle == 0UL || controllerHandle == 0UL)
            {
                return;
            }

            RuntimeInterop.unitas_runtime_animator_controller_destroy(_runtimeHandle, controllerHandle);
        }

        public bool SetController(ulong entityId, ulong controllerHandle)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_controller(_runtimeHandle, entityId, controllerHandle) == 0;
        }

        public bool SetFloat(ulong entityId, string name, float value)
        {
            if (!TryWriteName(name, ValueSize, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_float(_runtimeHandle, entityId, buffer, length, value) == 0;
        }

        public bool GetFloat(ulong entityId, string name, out float value)
        {
            value = 0f;
            if (!TryWriteName(name, ValueSize, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_float(_runtimeHandle, entityId, buffer, length, buffer) != 0)
            {
                return false;
            }

            value = ReadSingle(buffer, 0);
            return true;
        }

        public bool SetInteger(ulong entityId, string name, int value)
        {
            if (!TryWriteName(name, ValueSize, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_int(_runtimeHandle, entityId, buffer, length, value) == 0;
        }

        public bool GetInteger(ulong entityId, string name, out int value)
        {
            value = 0;
            if (!TryWriteName(name, ValueSize, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_int(_runtimeHandle, entityId, buffer, length, buffer) != 0)
            {
                return false;
            }

            value = BitConverter.ToInt32(ReadBytes(buffer, ValueSize), 0);
            return true;
        }

        public bool SetBool(ulong entityId, string name, bool value)
        {
            if (!TryWriteName(name, ValueSize, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_bool(_runtimeHandle, entityId, buffer, length, value ? 1U : 0U) == 0;
        }

        public bool GetBool(ulong entityId, string name, out bool value)
        {
            value = false;
            if (!TryWriteName(name, ValueSize, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_bool(_runtimeHandle, entityId, buffer, length, buffer) != 0)
            {
                return false;
            }

            value = BitConverter.ToUInt32(ReadBytes(buffer, ValueSize), 0) != 0U;
            return true;
        }

        public bool SetTrigger(ulong entityId, string name)
        {
            if (!TryWriteName(name, 0, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_trigger(_runtimeHandle, entityId, buffer, length) == 0;
        }

        public bool ResetTrigger(ulong entityId, string name)
        {
            if (!TryWriteName(name, 0, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_reset_trigger(_runtimeHandle, entityId, buffer, length) == 0;
        }

        public bool SetSpeed(ulong entityId, float speed)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_speed(_runtimeHandle, entityId, speed) == 0;
        }

        public bool GetSpeed(ulong entityId, out float speed)
        {
            speed = 0f;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, ValueSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_speed(_runtimeHandle, entityId, buffer) != 0)
            {
                return false;
            }

            speed = ReadSingle(buffer, 0);
            return true;
        }

        public bool GetStateInfo(ulong entityId, out AnimatorStateInfo stateInfo)
        {
            stateInfo = AnimatorStateInfo.Empty;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, StateNameCapacity);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            string stateName = string.Empty;
            int nameLength = RuntimeInterop.unitas_runtime_animator_state_name(_runtimeHandle, entityId);
            if (nameLength < 0)
            {
                return false;
            }

            if (nameLength > 0)
            {
                stateName = Marshal.PtrToStringUTF8(buffer, nameLength) ?? string.Empty;
            }

            IntPtr infoBuffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, StateInfoSize);
            if (infoBuffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_state_info(_runtimeHandle, entityId, infoBuffer) != 0)
            {
                return false;
            }

            byte[] bytes = ReadBytes(infoBuffer, StateInfoSize);
            stateInfo = new AnimatorStateInfo(
                stateName,
                (int)BitConverter.ToUInt32(bytes, 0),
                BitConverter.ToSingle(bytes, 4),
                BitConverter.ToSingle(bytes, 8),
                BitConverter.ToSingle(bytes, 12),
                BitConverter.ToSingle(bytes, 16),
                BitConverter.ToUInt32(bytes, 20) != 0U,
                new Vector3(
                    BitConverter.ToSingle(bytes, 32),
                    BitConverter.ToSingle(bytes, 36),
                    BitConverter.ToSingle(bytes, 40)));
            return true;
        }

        public bool Play(ulong entityId, string stateName)
        {
            if (!TryWriteName(stateName, 0, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_play(_runtimeHandle, entityId, buffer, length) == 0;
        }

        public bool PlayHash(ulong entityId, int stateNameHash)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_play_hash(_runtimeHandle, entityId, stateNameHash) == 0;
        }

        public bool PlayQueued(ulong entityId, string stateName)
        {
            if (!TryWriteName(stateName, 0, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_play_queued(_runtimeHandle, entityId, buffer, length) == 0;
        }

        public bool CrossFade(ulong entityId, string stateName, float normalizedTransitionDuration)
        {
            if (!TryWriteName(stateName, 0, out IntPtr buffer, out ulong length))
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_cross_fade(_runtimeHandle, entityId, buffer, length, normalizedTransitionDuration) == 0;
        }

        public bool CrossFadeHash(ulong entityId, int stateNameHash, float normalizedTransitionDuration)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_cross_fade_hash(_runtimeHandle, entityId, stateNameHash, normalizedTransitionDuration) == 0;
        }

        public bool SetApplyRootMotion(ulong entityId, bool apply)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_apply_root_motion(_runtimeHandle, entityId, apply ? 1U : 0U) == 0;
        }

        public bool GetApplyRootMotion(ulong entityId, out bool apply)
        {
            apply = false;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, ValueSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_apply_root_motion(_runtimeHandle, entityId, buffer) != 0)
            {
                return false;
            }

            apply = BitConverter.ToUInt32(ReadBytes(buffer, ValueSize), 0) != 0U;
            return true;
        }

        public bool GetRootMotion(ulong entityId, out Vector3 deltaPosition, out Quaternion deltaRotation)
        {
            deltaPosition = Vector3.zero;
            deltaRotation = Quaternion.identity;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, RootMotionSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_root_motion(_runtimeHandle, entityId, buffer) != 0)
            {
                return false;
            }

            byte[] bytes = ReadBytes(buffer, RootMotionSize);
            deltaPosition = new Vector3(
                BitConverter.ToSingle(bytes, 0),
                BitConverter.ToSingle(bytes, 4),
                BitConverter.ToSingle(bytes, 8));
            deltaRotation = new Quaternion(
                BitConverter.ToSingle(bytes, 12),
                BitConverter.ToSingle(bytes, 16),
                BitConverter.ToSingle(bytes, 20),
                BitConverter.ToSingle(bytes, 24));
            return true;
        }

        public bool SetIKPosition(ulong entityId, AvatarIKGoal goal, Vector3 goalPosition)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_ik_position(
                _runtimeHandle,
                entityId,
                (uint)goal,
                goalPosition.x,
                goalPosition.y,
                goalPosition.z) == 0;
        }

        public bool GetIKPosition(ulong entityId, AvatarIKGoal goal, out Vector3 goalPosition)
        {
            goalPosition = Vector3.zero;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, Vector3Size);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_ik_position(_runtimeHandle, entityId, (uint)goal, buffer) != 0)
            {
                return false;
            }

            byte[] bytes = ReadBytes(buffer, Vector3Size);
            goalPosition = new Vector3(
                BitConverter.ToSingle(bytes, 0),
                BitConverter.ToSingle(bytes, 4),
                BitConverter.ToSingle(bytes, 8));
            return true;
        }

        public bool SetIKPositionWeight(ulong entityId, AvatarIKGoal goal, float value)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_ik_position_weight(_runtimeHandle, entityId, (uint)goal, value) == 0;
        }

        public bool GetIKPositionWeight(ulong entityId, AvatarIKGoal goal, out float value)
        {
            value = 0f;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, ValueSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_ik_position_weight(_runtimeHandle, entityId, (uint)goal, buffer) != 0)
            {
                return false;
            }

            value = ReadSingle(buffer, 0);
            return true;
        }

        public bool SetIKRotation(ulong entityId, AvatarIKGoal goal, Quaternion goalRotation)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_ik_rotation(
                _runtimeHandle,
                entityId,
                (uint)goal,
                goalRotation.x,
                goalRotation.y,
                goalRotation.z,
                goalRotation.w) == 0;
        }

        public bool GetIKRotation(ulong entityId, AvatarIKGoal goal, out Quaternion goalRotation)
        {
            goalRotation = Quaternion.identity;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, QuaternionSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_ik_rotation(_runtimeHandle, entityId, (uint)goal, buffer) != 0)
            {
                return false;
            }

            byte[] bytes = ReadBytes(buffer, QuaternionSize);
            goalRotation = new Quaternion(
                BitConverter.ToSingle(bytes, 0),
                BitConverter.ToSingle(bytes, 4),
                BitConverter.ToSingle(bytes, 8),
                BitConverter.ToSingle(bytes, 12));
            return true;
        }

        public bool SetIKRotationWeight(ulong entityId, AvatarIKGoal goal, float value)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_ik_rotation_weight(_runtimeHandle, entityId, (uint)goal, value) == 0;
        }

        public bool GetIKRotationWeight(ulong entityId, AvatarIKGoal goal, out float value)
        {
            value = 0f;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, ValueSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_ik_rotation_weight(_runtimeHandle, entityId, (uint)goal, buffer) != 0)
            {
                return false;
            }

            value = ReadSingle(buffer, 0);
            return true;
        }

        public bool SetIKHintPosition(ulong entityId, AvatarIKHint hint, Vector3 hintPosition)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_ik_hint_position(
                _runtimeHandle,
                entityId,
                (uint)hint,
                hintPosition.x,
                hintPosition.y,
                hintPosition.z) == 0;
        }

        public bool GetIKHintPosition(ulong entityId, AvatarIKHint hint, out Vector3 hintPosition)
        {
            hintPosition = Vector3.zero;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, Vector3Size);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_ik_hint_position(_runtimeHandle, entityId, (uint)hint, buffer) != 0)
            {
                return false;
            }

            byte[] bytes = ReadBytes(buffer, Vector3Size);
            hintPosition = new Vector3(
                BitConverter.ToSingle(bytes, 0),
                BitConverter.ToSingle(bytes, 4),
                BitConverter.ToSingle(bytes, 8));
            return true;
        }

        public bool SetIKHintPositionWeight(ulong entityId, AvatarIKHint hint, float value)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_ik_hint_position_weight(_runtimeHandle, entityId, (uint)hint, value) == 0;
        }

        public bool GetIKHintPositionWeight(ulong entityId, AvatarIKHint hint, out float value)
        {
            value = 0f;
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, ValueSize);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }

            if (RuntimeInterop.unitas_runtime_animator_get_ik_hint_position_weight(_runtimeHandle, entityId, (uint)hint, buffer) != 0)
            {
                return false;
            }

            value = ReadSingle(buffer, 0);
            return true;
        }

        public bool SetLookAtPosition(ulong entityId, Vector3 lookAtPosition)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_look_at_position(
                _runtimeHandle,
                entityId,
                lookAtPosition.x,
                lookAtPosition.y,
                lookAtPosition.z) == 0;
        }

        public bool SetLookAtWeight(ulong entityId, float weight, float bodyWeight, float headWeight, float eyesWeight, float clampWeight)
        {
            if (_runtimeHandle == 0UL || entityId == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_animator_set_look_at_weight(
                _runtimeHandle,
                entityId,
                weight,
                bodyWeight,
                headWeight,
                eyesWeight,
                clampWeight) == 0;
        }

        public readonly struct FiredAnimatorEvent
        {
            public FiredAnimatorEvent(ulong entityId, string functionName, float floatParameter, int intParameter, string stringParameter)
            {
                EntityId = entityId;
                FunctionName = functionName;
                FloatParameter = floatParameter;
                IntParameter = intParameter;
                StringParameter = stringParameter;
            }

            public ulong EntityId { get; }

            public string FunctionName { get; }

            public float FloatParameter { get; }

            public int IntParameter { get; }

            public string StringParameter { get; }
        }

        public List<FiredAnimatorEvent> DrainFiredEvents()
        {
            List<FiredAnimatorEvent> events = new List<FiredAnimatorEvent>();
            if (_runtimeHandle == 0UL)
            {
                return events;
            }

            int byteLength = RuntimeInterop.unitas_runtime_animator_drain_fired_events(_runtimeHandle);
            if (byteLength < 4)
            {
                return events;
            }

            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)byteLength);
            if (buffer == IntPtr.Zero)
            {
                return events;
            }

            byte[] payload = ReadBytes(buffer, byteLength);
            uint count = BitConverter.ToUInt32(payload, 0);
            int offset = 4;
            for (uint index = 0; index < count; index++)
            {
                if (offset + 20 > payload.Length)
                {
                    break;
                }

                ulong entityId = BitConverter.ToUInt64(payload, offset);
                offset += 8;
                float floatParam = BitConverter.ToSingle(payload, offset);
                offset += 4;
                int intParam = BitConverter.ToInt32(payload, offset);
                offset += 4;

                int fnLen = (int)BitConverter.ToUInt32(payload, offset);
                offset += 4;
                if (fnLen < 0 || offset + fnLen > payload.Length)
                {
                    break;
                }

                string functionName = fnLen > 0 ? Encoding.UTF8.GetString(payload, offset, fnLen) : string.Empty;
                offset += fnLen;

                if (offset + 4 > payload.Length)
                {
                    break;
                }

                int strLen = (int)BitConverter.ToUInt32(payload, offset);
                offset += 4;
                if (strLen < 0 || offset + strLen > payload.Length)
                {
                    break;
                }

                string stringParam = strLen > 0 ? Encoding.UTF8.GetString(payload, offset, strLen) : string.Empty;
                offset += strLen;

                if (!string.IsNullOrEmpty(functionName))
                {
                    events.Add(new FiredAnimatorEvent(entityId, functionName, floatParam, intParam, stringParam));
                }
            }

            return events;
        }

        private bool TryWriteName(string name, int minimumCapacity, out IntPtr buffer, out ulong length)
        {
            byte[] utf8 = Encoding.UTF8.GetBytes(name ?? string.Empty);
            int capacity = Math.Max(minimumCapacity, utf8.Length);
            buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)capacity);
            if (buffer == IntPtr.Zero)
            {
                length = 0UL;
                return false;
            }

            Marshal.Copy(utf8, 0, buffer, utf8.Length);
            length = (ulong)utf8.Length;
            return true;
        }

        private static byte[] ReadBytes(IntPtr buffer, int count)
        {
            byte[] bytes = new byte[count];
            Marshal.Copy(buffer, bytes, 0, count);
            return bytes;
        }

        private static float ReadSingle(IntPtr buffer, int offset)
        {
            byte[] bytes = ReadBytes(buffer, offset + 4);
            return BitConverter.ToSingle(bytes, offset);
        }
    }

    public static class AnimatorControllerEncoder
    {
        private const uint WireVersion = 1;

        public static byte[] Encode(AnimatorController controller)
        {
            List<byte> bytes = new List<byte>();
            bytes.AddRange(Encoding.ASCII.GetBytes("UANC"));
            WriteUInt32(bytes, WireVersion);
            WriteString(bytes, controller.name);
            WriteParameters(bytes, controller.parameters);
            WriteStates(bytes, controller.states);
            int entryIndex = controller.entryState.Length > 0 ? controller.IndexOfState(controller.entryState) : 0;
            WriteUInt32(bytes, (uint)Math.Max(entryIndex, 0));
            WriteTransitions(bytes, controller);
            return bytes.ToArray();
        }

        private static void WriteParameters(List<byte> bytes, IReadOnlyList<AnimatorControllerParameter> parameters)
        {
            WriteUInt32(bytes, (uint)parameters.Count);
            for (int index = 0; index < parameters.Count; index++)
            {
                AnimatorControllerParameter parameter = parameters[index];
                WriteString(bytes, parameter.name);
                switch (parameter.type)
                {
                    case AnimatorParameterType.Float:
                        bytes.Add(0);
                        WriteSingle(bytes, parameter.floatValue);
                        break;
                    case AnimatorParameterType.Int:
                        bytes.Add(1);
                        WriteInt32(bytes, parameter.intValue);
                        break;
                    case AnimatorParameterType.Bool:
                        bytes.Add(2);
                        WriteBool(bytes, parameter.boolValue);
                        break;
                    default:
                        bytes.Add(3);
                        WriteBool(bytes, parameter.boolValue);
                        break;
                }
            }
        }

        private static void WriteStates(List<byte> bytes, IReadOnlyList<AnimatorControllerState> states)
        {
            WriteUInt32(bytes, (uint)states.Count);
            for (int index = 0; index < states.Count; index++)
            {
                AnimatorControllerState state = states[index];
                WriteString(bytes, state.name);
                WriteSingle(bytes, state.speed);
                WriteBool(bytes, state.isExit);
                if (state.clip == null)
                {
                    bytes.Add(0);
                }
                else
                {
                    bytes.Add(1);
                    WriteClip(bytes, state.clip);
                }
            }
        }

        private static void WriteTransitions(List<byte> bytes, AnimatorController controller)
        {
            WriteUInt32(bytes, (uint)controller.transitions.Count);
            for (int index = 0; index < controller.transitions.Count; index++)
            {
                AnimatorControllerTransition transition = controller.transitions[index];
                WriteUInt32(bytes, (uint)controller.IndexOfState(transition.sourceState));
                bytes.Add(0);
                WriteUInt32(bytes, (uint)controller.IndexOfState(transition.targetState));
                WriteBool(bytes, transition.hasExitTime);
                WriteSingle(bytes, transition.exitTime);
                WriteSingle(bytes, transition.duration);
                WriteUInt32(bytes, (uint)transition.conditions.Count);
                for (int conditionIndex = 0; conditionIndex < transition.conditions.Count; conditionIndex++)
                {
                    AnimatorCondition condition = transition.conditions[conditionIndex];
                    WriteString(bytes, condition.parameter);
                    bytes.Add((byte)condition.mode);
                    switch (condition.mode)
                    {
                        case AnimatorConditionMode.Greater:
                        case AnimatorConditionMode.Less:
                            WriteSingle(bytes, condition.floatValue);
                            break;
                        case AnimatorConditionMode.Equals:
                        case AnimatorConditionMode.NotEqual:
                            WriteInt32(bytes, condition.intValue);
                            break;
                    }
                }
            }
        }

        private static void WriteClip(List<byte> bytes, AnimationClip clip)
        {
            WriteString(bytes, clip.name);
            WriteSingle(bytes, clip.duration);
            WriteSingle(bytes, clip.sampleRate);
            bytes.Add((byte)clip.wrapMode);

            WriteUInt32(bytes, (uint)clip.curves.Count);
            for (int index = 0; index < clip.curves.Count; index++)
            {
                AnimationCurveBinding binding = clip.curves[index];
                WriteString(bytes, binding.objectPath);
                WriteString(bytes, binding.component);
                WriteString(bytes, binding.property);
                bytes.Add((byte)binding.propertyType);
                int channels = ChannelCount(binding.propertyType);
                for (int channel = 0; channel < channels; channel++)
                {
                    WriteFloatCurve(bytes, binding.curve);
                }
            }

            WriteUInt32(bytes, (uint)clip.events.Count);
            for (int index = 0; index < clip.events.Count; index++)
            {
                AnimationEvent animationEvent = clip.events[index];
                WriteSingle(bytes, animationEvent.time);
                WriteString(bytes, animationEvent.functionName);
                WriteSingle(bytes, animationEvent.floatParameter);
                WriteInt32(bytes, animationEvent.intParameter);
                WriteString(bytes, animationEvent.stringParameter);
            }
        }

        private static void WriteFloatCurve(List<byte> bytes, AnimationCurve curve)
        {
            Keyframe[] keys = curve.keys;
            WriteUInt32(bytes, (uint)keys.Length);
            for (int index = 0; index < keys.Length; index++)
            {
                Keyframe key = keys[index];
                WriteSingle(bytes, key.time);
                WriteSingle(bytes, key.value);
                WriteSingle(bytes, key.inTangent);
                WriteSingle(bytes, key.outTangent);
                bytes.Add((byte)key.interpolation);
            }
        }

        private static int ChannelCount(AnimationPropertyType propertyType)
        {
            switch (propertyType)
            {
                case AnimationPropertyType.Vector3:
                    return 3;
                case AnimationPropertyType.Quaternion:
                case AnimationPropertyType.Color:
                    return 4;
                default:
                    return 1;
            }
        }

        private static void WriteString(List<byte> bytes, string value)
        {
            byte[] utf8 = Encoding.UTF8.GetBytes(value ?? string.Empty);
            WriteUInt32(bytes, (uint)utf8.Length);
            bytes.AddRange(utf8);
        }

        private static void WriteUInt32(List<byte> bytes, uint value)
        {
            bytes.AddRange(BitConverter.GetBytes(value));
        }

        private static void WriteInt32(List<byte> bytes, int value)
        {
            bytes.AddRange(BitConverter.GetBytes(value));
        }

        private static void WriteSingle(List<byte> bytes, float value)
        {
            bytes.AddRange(BitConverter.GetBytes(value));
        }

        private static void WriteBool(List<byte> bytes, bool value)
        {
            bytes.Add(value ? (byte)1 : (byte)0);
        }
    }
}
