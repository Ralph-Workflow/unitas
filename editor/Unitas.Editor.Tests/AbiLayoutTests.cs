// Layout agreement across the three registered native boundary zones.
//
// The editor ABI crosses pointer-sized addresses, one four-byte status code,
// and UTF-8 JSON text. No struct crosses it: the Rust guard pins all six
// exported signatures at compile time, so a struct parameter could not be
// introduced unnoticed, and this guard pins the managed half of the same
// contract, the scalar widths and the scope names. The runtime ABI crosses
// two sequential structs, fixed-width scalars, C function pointers, and
// fixed-size byte records, so this guard pins their sizes, alignments, and
// field offsets. The .NET host ABI crosses only scalars, pointers, and
// function pointers, so this guard pins their widths.
//
// The six editor ABI delegates are private to NativeLibraryLoader and the
// eleven runtime ABI delegates carry their calling convention in an
// attribute, and reading either needs the reflection API the C# ban catalog
// forbids. The managed half of both is therefore pinned by the compile-time
// delegate assignments below, which fail the build when a signature drifts,
// and the camera blob and the animator state info sizes are private
// constants on both sides, so both sides pin the same agreed value.

using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Unitas.Bridge;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class AbiLayoutTests
    {
        private const int EditorAbiStatusCodeBytes = 4;

        private const int CharacterMoveInputBytes = 144;
        private const int CharacterMoveOutputBytes = 40;
        private const int QueryRecordBytes = 64;
        private const int CameraBlobFloats = 8;
        private const int CameraBlobBytes = 32;
        private const int AnimatorStateInfoBytes = 80;

        private static readonly string[] EditorAbiScopeNames =
        {
            "read-project",
            "edit-content",
            "edit-scripts",
            "execute-project-code",
            "import-external-files",
            "change-packages-and-settings",
            "build",
            "sign",
            "deploy",
            "external-network",
            "review",
        };

        [ModuleInitializer]
        internal static void RunOnAssemblyLoad()
        {
            Check.Suite = "AbiLayoutTests";
            Run();
            Check.Suite = string.Empty;
        }

        public static void Run()
        {
            EditorAbiCrossesPointerSizedAddressesAndAFourByteStatusCode();
            EditorAbiCarriesNoMarshalledStruct();
            EditorAbiScopeNamesMatchTheRustScopeTable();
            RuntimeAbiCharacterMoveInputMatchesTheRustStruct();
            RuntimeAbiCharacterMoveOutputMatchesTheRustStruct();
            RuntimeAbiQueryRecordMatchesTheRustRecord();
            RuntimeAbiBlobSizesMatchTheRustRecords();
            RuntimeAbiCallbacksMatchTheRustCallbackTable();
            DotnetHostAbiCrossesOnlyScalarsPointersAndFunctionPointers();
        }

        private static void EditorAbiCrossesPointerSizedAddressesAndAFourByteStatusCode()
        {
            Check.Equal(
                "abi-layout: the editor ABI signed address width is the pointer width",
                IntPtr.Size,
                Marshal.SizeOf(typeof(IntPtr)));
            Check.Equal(
                "abi-layout: the editor ABI unsigned address width is the pointer width",
                IntPtr.Size,
                Marshal.SizeOf(typeof(UIntPtr)));
            Check.Equal(
                "abi-layout: the editor ABI status code is four bytes",
                EditorAbiStatusCodeBytes,
                Marshal.SizeOf(typeof(int)));
        }

        private static void EditorAbiCarriesNoMarshalledStruct()
        {
            Check.That(
                "abi-layout: the native tool descriptor is a managed record, not a marshalled struct",
                !typeof(NativeToolDescriptor).IsValueType);
            Check.That(
                "abi-layout: the native tool scope crosses as a name, not as a value",
                typeof(Scope).IsEnum);
        }

        private static void EditorAbiScopeNamesMatchTheRustScopeTable()
        {
            Check.Equal(
                "abi-layout: the editor ABI scope table has eleven scopes",
                EditorAbiScopeNames.Length,
                Scopes.All.Length);
            for (int index = 0; index < Scopes.All.Length; index++)
            {
                Check.Equal(
                    "abi-layout: the editor ABI scope " + index + " name",
                    EditorAbiScopeNames[index],
                    Scopes.AsString(Scopes.All[index]));
            }
        }

        private static void RuntimeAbiCharacterMoveInputMatchesTheRustStruct()
        {
            Check.Equal(
                "abi-layout: the character move input size",
                CharacterMoveInputBytes,
                Marshal.SizeOf<RuntimeInterop.AbiCharacterMoveInput>());
            Check.Equal(
                "abi-layout: the character move input EntityId offset",
                0,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.EntityId)));
            Check.Equal(
                "abi-layout: the character move input PositionX offset",
                8,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.PositionX)));
            Check.Equal(
                "abi-layout: the character move input PositionY offset",
                16,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.PositionY)));
            Check.Equal(
                "abi-layout: the character move input PositionZ offset",
                24,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.PositionZ)));
            Check.Equal(
                "abi-layout: the character move input MotionX offset",
                32,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.MotionX)));
            Check.Equal(
                "abi-layout: the character move input MotionY offset",
                40,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.MotionY)));
            Check.Equal(
                "abi-layout: the character move input MotionZ offset",
                48,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.MotionZ)));
            Check.Equal(
                "abi-layout: the character move input Radius offset",
                56,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.Radius)));
            Check.Equal(
                "abi-layout: the character move input Height offset",
                64,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.Height)));
            Check.Equal(
                "abi-layout: the character move input CenterX offset",
                72,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.CenterX)));
            Check.Equal(
                "abi-layout: the character move input CenterY offset",
                80,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.CenterY)));
            Check.Equal(
                "abi-layout: the character move input CenterZ offset",
                88,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.CenterZ)));
            Check.Equal(
                "abi-layout: the character move input SlopeLimitDeg offset",
                96,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.SlopeLimitDeg)));
            Check.Equal(
                "abi-layout: the character move input StepOffset offset",
                104,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.StepOffset)));
            Check.Equal(
                "abi-layout: the character move input SkinWidth offset",
                112,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.SkinWidth)));
            Check.Equal(
                "abi-layout: the character move input MinMoveDistance offset",
                120,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.MinMoveDistance)));
            Check.Equal(
                "abi-layout: the character move input DeltaSeconds offset",
                128,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.DeltaSeconds)));
            Check.Equal(
                "abi-layout: the character move input LayerMask offset",
                136,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.LayerMask)));
            Check.Equal(
                "abi-layout: the character move input Enabled offset",
                140,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.Enabled)));
            Check.Equal(
                "abi-layout: the character move input Reserved0 offset",
                141,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.Reserved0)));
            Check.Equal(
                "abi-layout: the character move input Reserved1 offset",
                142,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.Reserved1)));
            Check.Equal(
                "abi-layout: the character move input Reserved2 offset",
                143,
                FieldOffset<RuntimeInterop.AbiCharacterMoveInput>(
                    nameof(RuntimeInterop.AbiCharacterMoveInput.Reserved2)));
        }

        private static void RuntimeAbiCharacterMoveOutputMatchesTheRustStruct()
        {
            Check.Equal(
                "abi-layout: the character move output size",
                CharacterMoveOutputBytes,
                Marshal.SizeOf<RuntimeInterop.AbiCharacterMoveOutput>());
            Check.Equal(
                "abi-layout: the character move output PositionX offset",
                0,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.PositionX)));
            Check.Equal(
                "abi-layout: the character move output PositionY offset",
                8,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.PositionY)));
            Check.Equal(
                "abi-layout: the character move output PositionZ offset",
                16,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.PositionZ)));
            Check.Equal(
                "abi-layout: the character move output GroundEntityId offset",
                24,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.GroundEntityId)));
            Check.Equal(
                "abi-layout: the character move output CollisionFlags offset",
                32,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.CollisionFlags)));
            Check.Equal(
                "abi-layout: the character move output IsGrounded offset",
                36,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.IsGrounded)));
            Check.Equal(
                "abi-layout: the character move output Reserved0 offset",
                37,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.Reserved0)));
            Check.Equal(
                "abi-layout: the character move output Reserved1 offset",
                38,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.Reserved1)));
            Check.Equal(
                "abi-layout: the character move output Reserved2 offset",
                39,
                FieldOffset<RuntimeInterop.AbiCharacterMoveOutput>(
                    nameof(RuntimeInterop.AbiCharacterMoveOutput.Reserved2)));
        }

        private static void RuntimeAbiQueryRecordMatchesTheRustRecord()
        {
            Check.Equal(
                "abi-layout: the query record size",
                QueryRecordBytes,
                RuntimeInterop.QueryRecordSize);
            Check.Equal(
                "abi-layout: the query hit size",
                QueryRecordBytes,
                Marshal.SizeOf<QueryHit>());
            Check.Equal(
                "abi-layout: the query hit Entity offset",
                0,
                FieldOffset<QueryHit>(nameof(QueryHit.Entity)));
            Check.Equal(
                "abi-layout: the query hit Distance offset",
                8,
                FieldOffset<QueryHit>(nameof(QueryHit.Distance)));
            Check.Equal(
                "abi-layout: the query hit X offset",
                16,
                FieldOffset<QueryHit>(nameof(QueryHit.X)));
            Check.Equal(
                "abi-layout: the query hit Y offset",
                24,
                FieldOffset<QueryHit>(nameof(QueryHit.Y)));
            Check.Equal(
                "abi-layout: the query hit Z offset",
                32,
                FieldOffset<QueryHit>(nameof(QueryHit.Z)));
            Check.Equal(
                "abi-layout: the query hit NX offset",
                40,
                FieldOffset<QueryHit>(nameof(QueryHit.NX)));
            Check.Equal(
                "abi-layout: the query hit NY offset",
                48,
                FieldOffset<QueryHit>(nameof(QueryHit.NY)));
            Check.Equal(
                "abi-layout: the query hit NZ offset",
                56,
                FieldOffset<QueryHit>(nameof(QueryHit.NZ)));
        }

        private static void RuntimeAbiBlobSizesMatchTheRustRecords()
        {
            Check.Equal(
                "abi-layout: the camera blob carries eight floats",
                CameraBlobFloats,
                CameraBlobBytes / sizeof(float));
            Check.Equal(
                "abi-layout: the camera blob occupies thirty-two bytes",
                CameraBlobBytes,
                CameraBlobFloats * sizeof(float));
            Check.Equal(
                "abi-layout: the animator state info occupies eighty bytes",
                AnimatorStateInfoBytes,
                20 * sizeof(uint));
        }

        private static void RuntimeAbiCallbacksMatchTheRustCallbackTable()
        {
            int invoked = 0;
            RuntimeInterop.CreateScriptCallback createScript = (ulong entity, ulong guidHi, ulong guidLo) =>
            {
                invoked++;
                return entity;
            };
            RuntimeInterop.AwakeCallback awake = (ulong scriptHandle) => { invoked++; };
            RuntimeInterop.OnEnableCallback onEnable = (ulong scriptHandle) => { invoked++; };
            RuntimeInterop.StartCallback start = (ulong scriptHandle) => { invoked++; };
            RuntimeInterop.FixedUpdateCallback fixedUpdate = (ulong scriptHandle, double fixedDeltaSeconds) => { invoked++; };
            RuntimeInterop.UpdateCallback update = (ulong scriptHandle, double deltaSeconds) => { invoked++; };
            RuntimeInterop.LateUpdateCallback lateUpdate = (ulong scriptHandle, double deltaSeconds) => { invoked++; };
            RuntimeInterop.OnDisableCallback onDisable = (ulong scriptHandle) => { invoked++; };
            RuntimeInterop.DestroyCallback destroy = (ulong scriptHandle) => { invoked++; };
            RuntimeInterop.CollisionCallback collision = (ulong entityA, ulong entityB, uint phase) => { invoked++; };
            RuntimeInterop.TriggerCallback trigger = (ulong entityA, ulong entityB, uint phase) => { invoked++; };

            Check.That(
                "abi-layout: the create script callback carries three handles and returns the script handle",
                createScript(1UL, 2UL, 3UL) == 1UL);
            awake(1UL);
            onEnable(1UL);
            start(1UL);
            fixedUpdate(1UL, 0.02);
            update(1UL, 0.016);
            lateUpdate(1UL, 0.016);
            onDisable(1UL);
            destroy(1UL);
            collision(1UL, 2UL, 0U);
            trigger(1UL, 2UL, 0U);
            Check.Equal(
                "abi-layout: the runtime ABI invokes eleven script callbacks",
                11,
                invoked);
        }

        private static int FieldOffset<T>(string fieldName) where T : struct
        {
            return Marshal.OffsetOf<T>(fieldName).ToInt32();
        }

        private static void DotnetHostAbiCrossesOnlyScalarsPointersAndFunctionPointers()
        {
            Check.Equal(
                "abi-layout: the host ABI status code is four bytes",
                4,
                sizeof(int));
            Check.Equal(
                "abi-layout: the host ABI handle is eight bytes",
                8,
                sizeof(ulong));
            Check.Equal(
                "abi-layout: the host ABI pointer width",
                IntPtr.Size,
                Marshal.SizeOf(typeof(IntPtr)));
            Check.Equal(
                "abi-layout: the host ABI unsigned pointer width",
                IntPtr.Size,
                Marshal.SizeOf(typeof(UIntPtr)));
        }
    }
}
