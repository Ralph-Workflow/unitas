# Scripting and Runtime Product Specification

## Scope

This specification defines the observable C# scripting and runtime behavior of
unitas: compilation and assemblies, the Unity object model, lifecycle and
messages, coroutines, asynchronous work and threading, time, runtime scene
management, resource loading, serialized events and delayed invocation, web
requests and file locations, editor-time scripts, and jobs. It owns RUN-001
through RUN-011 in the [Feature catalog](feature-catalog.md). Compatibility
means that supported Unity scripts retain their public meaning, not merely that
similarly named types compile.

Script source is edited in the developer's external code editor, IDE, or agent.
Generated project files, language-server-compatible diagnostics, and debugging
through the Debug Adapter Protocol are owned by DIA-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).
Every capability in this specification is available to authorized agents
through automation, including the agent code workflow owned by AUT-005, as
defined in [Automation and MCP](automation-and-mcp.md), and every capability
runs in headless sessions as well as in the visual editor.

No project code described here (runtime scripts, editor scripts, analyzers,
source generators, importers, or postprocessors) executes before the project is
trusted, as defined by PRJ-008 in [Projects and assets](project-and-assets.md).
Project code runs with the privileges of the user who trusted the project.

## Script authoring experience

As a gameplay programmer, I can add supported C# scripts to an existing project
and use them as components, serialized assets, editor-visible types, and runtime
behavior without changing Unity namespaces or ordinary source patterns.

The product:

* recognizes supported script types after a successful compilation;
* reports errors with file, line, column, severity, diagnostic code, assembly,
  and message;
* keeps the last valid project state inspectable when a later edit fails;
* preserves serialized field values when type and field identities remain
  compatible;
* distinguishes a missing script from a script that compiled but failed at
  runtime;
* reports ambiguous, duplicate, unavailable, or incompatible types explicitly;
* exposes compilation state to both the visual editor and authorized agents.

One script error does not appear as an asset-import, scene, or build error unless
that operation is genuinely affected by the script failure.

## Script compilation and assemblies

### Predefined assemblies and special folders

Scripts outside any assembly definition compile into Unity's predefined
assemblies with Unity's folder rules and compilation order:

* `Assembly-CSharp-firstpass` contains runtime scripts under top-level
  `Plugins`, `Standard Assets`, and `Pro Standard Assets` folders.
* `Assembly-CSharp-Editor-firstpass` contains scripts in `Editor` folders
  beneath those folders.
* `Assembly-CSharp` contains all other runtime scripts and references the
  first-pass assemblies.
* `Assembly-CSharp-Editor` contains scripts in any other `Editor` folder and
  references all of the above.

A script in an `Editor` folder that lies inside an assembly definition's folder
belongs to that assembly definition, not to an editor predefined assembly, as in
Unity. Scripts in a predefined runtime assembly that reference `UnityEditor`
without an editor-only guard compile in the editor and fail the player build
with a diagnostic naming the file and member (BLD-001 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md)).

### Assembly definitions and references

Assembly definition (`.asmdef`) files define named assemblies with Unity's
meaning for:

* name and root namespace;
* assembly references by name or by GUID;
* include platforms and exclude platforms, including editor-only assemblies;
* allow unsafe code;
* auto referenced, which controls whether predefined assemblies reference the
  assembly implicitly;
* no engine references;
* override references with an explicit list of precompiled references;
* define constraints, which exclude the assembly when the expression over
  defined symbols is false;
* version defines, which add a symbol when a named package, module, or the
  Unity version matches a version range.

Assembly definition reference (`.asmref`) files add their folder's scripts to
the referenced assembly. Duplicate assembly names, reference cycles, references
to missing assemblies, scripts claimed by more than one definition, and
invalid version ranges are reported with the defining file and the affected
assemblies. An assembly excluded on the current target by platform or define
constraints is reported as excluded, not as missing, and its dependents report
the unresolved references.

### Defines, compiler options, and language profile

The compilation for each assembly and target defines the symbols Unity defines
for the project's recorded Unity version and the selected target, including:

* `UNITY_EDITOR` and the host-specific editor symbols in editor and headless
  editor sessions, and never in players;
* `UNITY_2020_3_OR_NEWER`, `UNITY_2021_3_OR_NEWER`, `UNITY_2022_3_OR_NEWER`,
  and the corresponding release-specific version symbols;
* platform symbols such as `UNITY_STANDALONE`, `UNITY_STANDALONE_WIN`,
  `UNITY_IOS`, `UNITY_ANDROID`, and `UNITY_WEBGL`;
* `DEVELOPMENT_BUILD` in development players;
* scripting backend and API compatibility symbols such as `ENABLE_MONO`,
  `ENABLE_IL2CPP`, `NET_STANDARD_2_0`, `NET_STANDARD_2_1`, and `NET_4_6`;
* the scripting define symbols set per target in Player settings (PRJ-007 in
  [Projects and assets](project-and-assets.md));
* symbols added by version defines and by response files.

A `csc.rsp` response file contributes defines, warning suppression, warnings
as errors, and other compiler options Unity accepts for that version; an
option unitas does not honor is reported naming the option and file.

The C# language version and API compatibility level follow the project's
Unity version: C# 8 for Unity 2020.3 and C# 9 for Unity 2021.3 and 2022.3,
with the .NET Standard and .NET Framework API compatibility levels that the
project's Player settings select. Language features that Unity's compiler does
not support for that version, such as covariant return types, are diagnosed as
in Unity. The unitas profile is never wider than the project's version
without being reported.

### Analyzers and source generators

Roslyn analyzers and source generators labeled for use as analyzers run against
the assemblies that Unity's rules for the project's version apply them to, and
only in a trusted project. Analyzer diagnostics carry their own identifiers and
severities, honor rule set files, and appear with compiler diagnostics.
Generated sources are inspectable and navigable. An analyzer or generator that
fails to load or throws is reported naming the analyzer and assembly, and does
not prevent compilation of assemblies it does not apply to. In an untrusted
project, compilation is not performed and the report states that analyzers and
generators were not run.

### Precompiled and native plugins

Precompiled managed assemblies (`.dll`) are referenced according to their
plugin import settings: compatible platforms and editor, CPU architecture, auto
reference, validate references, and define constraints. A managed plugin
excluded from a target is absent from that target's compilation and player.

Native plugins (`.dll`, `.so`, `.dylib`, `.bundle`, `.a`, `.framework`, `.aar`,
`.jar`, and browser `.jslib` and `.jspre` files) are included per platform and
architecture according to their import settings. `DllImport` resolution,
including `__Internal` for statically linked targets, follows Unity's rules. A
native library that is absent, built for another architecture, or excluded on
the running target produces a diagnostic naming the library, the target, the
architecture, and the calling member; it does not appear as a generic load
failure. A native plugin that cannot run in the editor host is reported as
Platform-limited for the editor rather than as a project error.

### Recompilation and the last valid state

A script, assembly definition, define, or plugin change recompiles the affected
assemblies and their dependents and leaves unaffected assemblies untouched.
Compilation is requested automatically after a change and on demand through
automation, without a manual refresh.

When compilation fails, the last successfully compiled assemblies remain in
effect for inspection, existing serialized values remain untouched, and
components whose types are absent from the failed compilation are not
converted into missing scripts. A change made during play mode follows the
project's script-changes-while-playing preference: recompile and continue,
recompile after play ends, or stop play and recompile.

## Runtime type behavior

The end-state scripting profile includes the C# 9 and .NET Standard 2.1 surface
declared by unitas. Compatibility reports distinguish this unitas profile from
the language and API profiles actually available in each supported Unity
project version. A supported type or member has compatible
effects, return values, exceptions, null behavior, timing, object lifetime, and
platform limits.

Core compatibility covers Object, GameObject, Component, Behaviour,
MonoBehaviour, Transform, ScriptableObject, instantiation, destruction,
discovery, and component access, together with:

* value types `Vector2`, `Vector3`, `Vector4`, `Vector2Int`, `Vector3Int`,
  `Quaternion`, `Matrix4x4`, `Color`, `Color32`, `Rect`, `RectInt`, `Bounds`,
  `BoundsInt`, `Ray`, `Ray2D`, `Plane`, and `LayerMask`, including Unity's
  approximate equality operators, zero-vector normalization, default
  quaternion values, and conversions between them;
* `Mathf`, including `Approximately`, angle, interpolation, smoothing, and
  rounding functions;
* `Time`, `Debug`, `Application`, `SystemInfo`, `Screen` (dimensions, DPI,
  orientation, safe area, and cutouts), and `Cursor` (visibility, lock state,
  and custom cursors);
* `Resources` (see [Resource loading](#resource-loading-and-content-catalogs))
  and `PlayerPrefs`, whose storage behavior is owned by SAV-001 in
  [Accessibility, localization, and persistence](accessibility-localization-persistence.md).

Results of floating-point operations are equivalent within the variation the
[Compatibility contract](compatibility-contract.md) allows; exact bitwise
parity is not claimed. Cursor lock, cursor images, and screen orientation are
Platform-limited where the target restricts them, for example browser cursor
lock requiring a user gesture.

Where an API is unavailable, the product identifies the exact type or member
and its consequence. An inert placeholder is not reported as support.

## Object identity and lifetime

As a programmer, I can rely on stable runtime identity for live objects.
Instantiation produces a distinct object graph with supported serialized values
and internal references preserved. Destruction makes the object unavailable at
the compatible frame boundary and produces compatible comparison and null-like
behavior for retained managed references.

A destroyed object compares equal to `null` through Unity's equality operators
and implicit boolean conversion, while the C# null-conditional, null-coalescing,
and `is null` forms observe the managed reference exactly as in Unity. Member
access on a destroyed object raises `MissingReferenceException` or
`NullReferenceException` under Unity's conditions, naming the type and, where
known, the original object. `Destroy` with and without a delay,
`DestroyImmediate`, and destruction of assets versus scene objects have
compatible effects and errors.

Object discovery:

* respects type compatibility and requested active-state rules;
* distinguishes scene objects from project assets;
* returns components from the correct owner, parents, or descendants;
* never returns a different object merely because a referenced object is gone;
* produces deterministic results where the Unity contract defines ordering.

## Activation and lifecycle

A GameObject has a local active choice and an effective hierarchy state. An
enabled Behaviour participates only while its object and ancestors allow it.
Changing a parent or ancestor does not erase the descendant's local choice.

Supported MonoBehaviours receive lifecycle events in compatible circumstances.
Ordering guarantees apply only where Unity defines them; the product does not
invent a total order among unrelated behaviours:

1. `Awake` when the loaded instance becomes initialized.
2. `OnEnable` whenever an enabled behaviour becomes effectively active.
3. `Start` once, immediately before its first eligible frame activity.
4. `FixedUpdate` for eligible fixed simulation steps.
5. `Update` once for each eligible rendered frame.
6. `LateUpdate` after eligible `Update` calls for that frame.
7. `OnDisable` when effective activity ends.
8. `OnDestroy` when destruction or unloading satisfies the compatible Unity
   conditions for that instance; an instance that never became eligible does
   not receive a callback merely because the type declares one.

`Start` and the other messages that Unity permits to return `IEnumerator` run
as coroutines when declared that way. Instantiation, enablement changes,
hierarchy activation, scene loading, scene unloading, destruction, application
focus, and shutdown produce inspectable event sequences. Exceptions in one
callback identify the script, method, object, scene, and frame and do not
masquerade as successful execution.

### Script execution order

The Script Execution Order setting and `[DefaultExecutionOrder]` order
`Awake`, `OnEnable`, `Start`, `FixedUpdate`, `Update`, and `LateUpdate` across
script types as in Unity; a value assigned in the setting takes precedence over
the attribute. Types with equal order have no guaranteed relative order. The
effective order of every script type is inspectable, and changing it through
the editor or automation is stored in the Unity-readable location
(PRJ-007 in [Projects and assets](project-and-assets.md)).

### Script attributes

The following attributes have Unity's observable effect in the editor, in
headless sessions, through automation, and in players:

* `[RequireComponent]` adds the required components when the component is
  added, and refuses removal of a required component with a reason naming the
  dependent component.
* `[DisallowMultipleComponent]` refuses a second instance on the same
  GameObject with a reason; the refusal applies equally to agent edits.
* `[ExecuteAlways]` and `[ExecuteInEditMode]` run the behaviour's callbacks
  outside play mode under Unity's conditions, including the difference between
  them in prefab editing mode, and only in a trusted project.
* `[RuntimeInitializeOnLoadMethod]` invokes static methods once per play
  session or player start at the requested load type:
  `SubsystemRegistration`, `AfterAssembliesLoaded`, `BeforeSplashScreen`,
  `BeforeSceneLoad`, or `AfterSceneLoad` (the default). Load types run in that
  order; methods within one load type have no guaranteed order.
* `[SerializeField]`, `[HideInInspector]`, `[NonSerialized]`,
  `[SerializeReference]`, and `[FormerlySerializedAs]` affect persistence and
  presentation as described in
  [Serialization-facing script behavior](#serialization-facing-script-behavior).
* `[AddComponentMenu]` sets the path and order under which the component is
  offered for adding, in the editor and in automation component discovery.

### Additional messages

MonoBehaviours also receive these messages under Unity's conditions:

* `OnApplicationPause(bool)`, `OnApplicationFocus(bool)`, and
  `OnApplicationQuit`, with Unity's ordering relative to `OnDisable` and
  `OnDestroy` at shutdown, and `Application.wantsToQuit` able to cancel a quit
  request where the target allows cancellation;
* `OnValidate` and `Reset` at edit time, as defined in
  [Editor-time scripts](#editor-time-scripts-and-content-hooks);
* `OnBecameVisible` and `OnBecameInvisible` for renderers, counting every
  camera that Unity counts, including the scene view camera in the editor;
* `OnTransformParentChanged` and `OnTransformChildrenChanged`;
* `OnMouseDown`, `OnMouseUp`, `OnMouseUpAsButton`, `OnMouseEnter`,
  `OnMouseOver`, `OnMouseExit`, and `OnMouseDrag` for objects with colliders,
  delivered only when the legacy input manager is active, as in Unity
  (INP-001 in [UI, audio, and input](ui-audio-input.md));
* `OnRenderObject`, `OnWillRenderObject`, and camera messages, delivered under
  the conditions the active render pipeline defines in Unity (REN-006 in
  [Rendering and animation](rendering-and-animation.md));
* `OnAnimatorMove` and `OnAnimatorIK`, owned by ANI-002 in
  [Rendering and animation](rendering-and-animation.md).

Collision and trigger messages are owned by PHY-001 and PHY-002 in
[Physics, navigation, and terrain](physics-navigation-terrain.md), `OnGUI` by
UI-002 and `OnAudioFilterRead` by AUD-001 in
[UI, audio, and input](ui-audio-input.md). A message method whose parameters
do not match the signature Unity requires is reported with the script and
method, and is invoked or ignored exactly as Unity does.

### Domain reload and Enter Play Mode Options

Entering play mode honors the project's Enter Play Mode Options:

* With domain reload enabled, static state is reset and `[InitializeOnLoad]`
  and runtime initialization methods run again.
* With domain reload disabled, static fields, static event subscriptions, and
  running background work retain their values from the previous session, and
  only explicit resets such as `SubsystemRegistration` initialization methods
  clear them.
* With scene reload disabled, the open scenes are not reloaded from disk and
  object lifecycle callbacks follow Unity's rules for that option.

The play session reports which options were in effect. Exiting play mode
restores edit-time state as defined by EDT-004 in
[Editor and authoring](editor-and-authoring.md).

## Time and frame behavior

Time exposes frame delta, unscaled delta, fixed delta, elapsed time, frame count,
time scale, real time, maximum delta, and supported smoothed values with
compatible relationships. Pausing gameplay, changing time scale, stepping a
frame, and running multiple fixed steps in one rendered frame have defined
effects. Target frame rate and frame pacing are owned by REN-007 in
[Rendering and animation](rendering-and-animation.md).

As a developer, I can distinguish:

* rendered frames from fixed simulation steps;
* scaled from unscaled elapsed time;
* editor pause from a game's own pause state;
* runtime time from edit-time preview time;
* a slow frame from an intentional time-scale change.

## Coroutines and asynchronous operations

A MonoBehaviour can start, enumerate, and stop supported coroutines. A
coroutine can wait for the next frame, a fixed update, end of frame, a scaled or
unscaled duration, a predicate, another coroutine, or a supported asynchronous
operation, including custom `CustomYieldInstruction` subclasses.

The coroutine ends when it completes, is stopped through a compatible API, its
owner is destroyed or made inactive under the compatible rule, or another
compatible lifetime rule applies. Disabling a MonoBehaviour and deactivating
its GameObject remain distinct cases and do not implicitly share stop behavior.
Yielded nulls, already-completed operations, nested enumerators, destroyed
objects, predicate exceptions, and scene unloading all have explicit behavior.

`AsyncOperation` and its subclasses expose `progress`, `isDone`, `priority`,
and the `completed` event with Unity's timing, including a `completed`
handler registered after completion.

### Threading and async/await

Unity APIs that Unity restricts to the main thread raise a compatible
exception naming the API when called from another thread, including from jobs
and thread-pool tasks; the thread-safe members Unity permits, such as `Debug`
logging and value-type math, remain callable. A non-main-thread call never
succeeds silently and never corrupts state.

An `await` continuation captured on the main thread resumes on the main thread
through Unity's synchronization context at the compatible point of the frame
loop, and does not run while the editor is paused. Continuations configured
not to capture the context, `Task.Run` work, and explicitly created threads run
on background threads under .NET rules.

Asynchronous work, threads, and timers that outlive the play session in which
they started are reported at the end of that session with their origin, and
their later attempts to touch destroyed objects or main-thread APIs are
reported as such. Ending a session does not silently cancel such work, as in
Unity.

On targets without threads, such as the browser, thread creation and blocking
waits are Platform-limited: the build report and runtime diagnostics name the
affected API and calling member (PLT-001 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md)).

## Runtime scene management

Scene assets, their authoring, and edit-time scene sets are owned by SER-002 in
[Serialization and prefabs](serialization-and-prefabs.md). This section owns
`SceneManager` behavior at runtime, in play mode, headless play sessions, and
players.

### Loading and activation

* `SceneManager.LoadScene` with `LoadSceneMode.Single` completes the load in
  the next frame, unloads every loaded scene except the `DontDestroyOnLoad`
  scene, destroys their objects with the compatible callbacks, makes the new
  scene active, and releases unused assets as Unity does for a single load.
* `LoadSceneMode.Additive` adds the scene alongside loaded scenes without
  changing the active scene. `LoadSceneParameters` with a local physics mode
  is owned by PHY-004 in
  [Physics, navigation, and terrain](physics-navigation-terrain.md).
* `SceneManager.LoadSceneAsync` returns an operation whose `progress` advances
  toward completion. While `allowSceneActivation` is false, the load holds at
  a progress of 0.9 with `isDone` false, and later asynchronous scene
  operations queued behind it wait as in Unity. Setting it to true activates
  the scene.
* Activation runs `Awake` and `OnEnable` for the loaded scene's active objects,
  then `sceneLoaded`, then `Start` before those objects' first frame activity.
  `sceneLoaded`, `sceneUnloaded`, and `activeSceneChanged` fire in Unity's
  order for each operation, and the recorded sequence is inspectable.

### Active scene, unloading, and persistent objects

* The active scene receives newly created and instantiated root objects and
  supplies the scene-level lighting and render settings (REN-003 in
  [Rendering and animation](rendering-and-animation.md)).
  `SetActiveScene` fails for a scene that is not loaded, with the reason.
* `SceneManager.UnloadSceneAsync` unloads the named scene with compatible
  callbacks and options; unloading the only loaded scene is refused with
  Unity's error. A `Scene` value for an unloaded scene reports itself invalid.
* `SceneManager.CreateScene` creates an empty, unsaved runtime scene with a
  unique name; a duplicate name is rejected.
* `SceneManager.MoveGameObjectToScene` moves root objects only into a loaded
  scene, with Unity's errors otherwise.
* `Object.DontDestroyOnLoad` moves a root object into the `DontDestroyOnLoad`
  scene, where it survives single loads until the session ends; applied to a
  child object it warns and has no effect, as in Unity.

### Resolution and failure

A scene is resolved by name, by project path, or by build index according to
the build scene list (BLD-001 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md)).
When a name matches more than one scene, the scene Unity would choose is
loaded and a diagnostic names every match and the path that disambiguates. A
scene that is absent from the build list or disabled in it fails to load in
players and in play mode with Unity's error, except through the editor-only
load paths Unity provides. An invalid index or unknown name fails without
changing loaded scenes.

Scene operations requested from inside lifecycle callbacks, from
`sceneLoaded` or `sceneUnloaded` handlers, or while another load is pending are
queued and applied with Unity's ordering, and the resulting sequence is
inspectable. Loaded scenes, the active scene, pending operations, and their
progress are visible in the editor and through automation.

## Resource loading and content catalogs

### Resources folders

`Resources.Load`, `Resources.LoadAll`, and `Resources.LoadAsync` find assets in
any `Resources` folder, including those inside packages, by path relative to
that folder without extension, with optional type filtering. A missing path
returns null or an empty result without an exception, as in Unity, and a
type mismatch returns null. `Resources.UnloadAsset` releases a loaded asset and
rejects GameObjects and components with Unity's error;
`Resources.UnloadUnusedAssets` releases unreferenced assets asynchronously.
Every asset in a `Resources` folder is included in players (BLD-001).

### Streaming assets

Files under `StreamingAssets` are copied unprocessed into players and are
reachable through `Application.streamingAssetsPath`. Direct file access works
where the target permits it; on Android packages and the browser, access goes
through `UnityWebRequest`, and direct file access there is reported as
Platform-limited naming the calling member.

### AssetBundles

Asset bundle names and variants assigned to assets are preserved and
editable. `BuildPipeline.BuildAssetBundles` produces bundles and a manifest for
the selected target. Bundles load from file, memory, stream, and web request,
synchronously and asynchronously, and expose their assets and scenes.

* Loading an asset whose dependency bundles are not loaded produces missing
  references and a diagnostic naming the absent bundle.
* Loading the same bundle twice fails with Unity's error.
* `AssetBundle.Unload(true)` destroys assets loaded from the bundle;
  `Unload(false)` leaves them alive and detached, so reloading the bundle
  produces new copies, as in Unity.
* A bundle built for another target, or with a serialized form unitas cannot
  read (SER-005 in [Serialization and prefabs](serialization-and-prefabs.md)),
  is reported Unsupported naming the bundle and reason rather than partially
  loaded.

### Addressables

With the Addressables package (scoped by CMP-002 in the
[Compatibility contract](compatibility-contract.md)), assets load by address
key, label, `AssetReference`, and resource location. Load, instantiate, and
download operations return handles with status, progress, result, and
exception. Each load increments a reference count; `Release` and
`ReleaseInstance` decrement it, and content unloads when its count reaches zero.
Releasing an invalid or already-released handle is reported. Local and remote
catalogs, catalog update checks, content updates, download size queries, and
dependency downloads behave as the package defines, and an invalid key fails
with the package's exception. In the editor, the selected Addressables play
mode script (asset database, simulated groups, or existing build) is honored
and reported.

### Residency

Loaded assets, bundles, Addressables handles and reference counts, and the
origin of each load are inspectable in the editor and through automation, with
memory attribution owned by PRF-001 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

## Serialized events, messages, and delayed invocation

### UnityEvent

`UnityEvent` and generic `UnityEvent<T0>` through `UnityEvent<T0, T1, T2, T3>`
fields store persistent listeners with target object, method name, argument
mode (dynamic, void, object, int, float, string, or bool), static argument
value, and call state (off, editor and runtime, or runtime only). Persistent
listeners are authored in the inspector (EDT-002 in
[Editor and authoring](editor-and-authoring.md)) and through automation.

`Invoke` calls enabled persistent listeners in serialized order and then
dynamic listeners added with `AddListener` in the order added.
`RemoveListener` and `RemoveAllListeners` affect dynamic listeners only.
Listeners added or removed during an invocation and exceptions thrown by a
listener affect the remaining listeners as in Unity. A persistent listener
whose target is missing or whose method or argument type no longer exists is
reported on load and on invocation, naming the owner object, event field,
target, and method; it is not invoked and is not silently removed. UI controls
use this behavior for their events (UI-001 in
[UI, audio, and input](ui-audio-input.md)).

### SendMessage family

`SendMessage` calls the named method on every component of the GameObject,
`BroadcastMessage` on the GameObject and its descendants, and
`SendMessageUpwards` on the GameObject and its ancestors, with zero or one
argument. Inactive objects and disabled behaviours are included or skipped by
Unity's rules. With `SendMessageOptions.RequireReceiver`, a message with no
receiver logs Unity's error; with `DontRequireReceiver` it is silent. An
argument type that does not match the receiver's parameter is reported naming
the method.

### Invoke family

`Invoke` calls a method by name after a delay in scaled time.
`InvokeRepeating` calls it first after the delay and then at the repeat rate.
`CancelInvoke` with and without a method name cancels pending invocations,
and `IsInvoking` reports them. Pending invocations use scaled time, so a time
scale of zero holds them; destroying the owner cancels them; disabling the
behaviour or deactivating its object has the effect Unity defines. A method
name that does not exist on the behaviour logs Unity's error naming the type
and method.

## Web requests, file locations, JSON, and randomness

### UnityWebRequest

`UnityWebRequest` supports GET, POST, PUT, DELETE, HEAD, and custom methods;
form and multipart bodies; raw and file upload handlers; buffer, file,
texture, asset bundle, and audio clip download handlers; request and response
headers; timeout; redirect limit; upload and download progress; `Abort`; and
disposal of handlers. Results distinguish success, connection error, protocol
error with the response code, and data processing error, using the result
model of the project's Unity version.

Server certificates are validated by the platform; a `CertificateHandler`
can accept or reject certificates where the target allows it. Target
restrictions are reported as Platform-limited with the request and reason,
including browser cross-origin rules and forbidden headers, cleartext HTTP
blocked on mobile targets unless the project setting or platform policy
allows it, and missing network permission. Requests made by editor scripts
run only in a trusted project. Request URL, method, status, timing, and
result are recorded in diagnostics, with credential-bearing headers redacted.

### File locations

`Application.dataPath`, `Application.persistentDataPath`,
`Application.streamingAssetsPath`, and `Application.temporaryCachePath` resolve
per target to the locations Unity uses for that target and the project's
company and product names. In editor and headless sessions they resolve to the
locations Unity uses on that host, so data the project wrote while running
under Unity is found. Paths that are not writable or not directly accessible
on a target are reported with the target and path.

### JsonUtility

`JsonUtility.ToJson`, `FromJson`, and `FromJsonOverwrite` apply the same field
rules as Unity serialization (SER-001 in
[Serialization and prefabs](serialization-and-prefabs.md)): public and
`[SerializeField]` fields are included, properties, dictionaries, and top-level
arrays are not, and engine object references follow Unity's rules for the
calling context. Number formatting is culture-invariant. Malformed input
raises Unity's exception with the parse position. `EditorJsonUtility` is
available to editor scripts.

### Randomness

`UnityEngine.Random.InitState` and `Random.state` seed, save, and restore the
generator. The same seed produces the same sequence as Unity for `value` and
for integer and floating-point `Range`, and derived values such as
`insideUnitSphere` and `rotation` are equivalent within allowed floating-point
variation. Integer `Range` excludes its maximum and floating-point `Range`
includes it, as in Unity. `System.Random` follows .NET behavior.

## Serialization-facing script behavior

Supported public fields and fields marked for serialization appear in the
inspector and retain compatible values. Hidden serialized fields remain stored
without being presented as ordinary editable fields. Static, constant,
unsupported, or explicitly nonserialized values do not appear as persisted
instance state.

Supported values include primitives, enumerations, strings, engine object
references, compatible structs and classes, arrays, lists, and declared managed
references. The editor distinguishes null, missing object reference, invalid
type, renamed or absent field, and value conversion failure.

`[SerializeField]` persists a non-public field; `[HideInInspector]` hides a
persisted field from presentation; `[NonSerialized]` excludes a public field;
`[SerializeReference]` persists managed references with polymorphism and
shared identity; and `[FormerlySerializedAs]` loads a value stored under an old
field name. The stored form and preservation rules are owned by SER-001 in
[Serialization and prefabs](serialization-and-prefabs.md).

## ScriptableObject behavior

As a developer, I can create ScriptableObject assets, reference them from scenes
and other assets, inspect and edit their supported fields, instantiate temporary
runtime copies, and preserve asset identity across moves and renames.
`[CreateAssetMenu]` offers the type for creation in the editor and through
automation.

Changing a runtime copy does not silently mutate the project asset. Shared asset
references continue to refer to the same asset unless explicitly replaced.
Changes made to a project asset during play mode persist after play mode ends,
as in Unity, and are shown as unsaved asset changes.

## Editor-time scripts and content hooks

Editor scripts run in the editor and in headless sessions of a trusted project.
Assemblies that reference `UnityEditor` as editor-only, and the predefined
editor assemblies, are never included in players.

### Content hooks

* `AssetPostprocessor` callbacks for textures, models, audio, animations,
  materials, and all assets run during import with Unity's ordering, including
  postprocessor order and version; changing a postprocessor's version
  reimports the affected assets.
* `ScriptedImporter` types handle their declared extensions, add objects and
  set the main object through the import context, declare dependencies, and
  reimport when their version changes. Import failures name the importer and
  asset.
* `OnValidate` runs after a component or asset loads in the editor and after
  each edit of its values, including agent edits; `Reset` runs when a
  component is added or reset.
* `[InitializeOnLoad]` static constructors and `[InitializeOnLoadMethod]`
  methods run after each editor domain reload.
* `EditorApplication` callbacks such as `update`, `delayCall`,
  `playModeStateChanged`, `hierarchyChanged`, `projectChanged`, and `quitting`
  fire under Unity's conditions.

Asset import itself is owned by PRJ-004 in
[Projects and assets](project-and-assets.md).

### Commands from editor scripts

Methods marked `[MenuItem]`, including validation functions, priority, and
shortcuts, and methods marked `[ContextMenu]` or referenced by
`[ContextMenuItem]` appear as named, invokable commands in the editor command
palette (EDT-006 in [Editor and authoring](editor-and-authoring.md)) and as
automation tools. A validation function that returns false appears as a
disabled command with its reason. Commands from custom editor extensions
remain available even though their custom drawing is not.

### Editor APIs

`AssetDatabase`, `EditorUtility`, `SerializedObject`, `SerializedProperty`,
`Undo`, `EditorPrefs`, `EditorSceneManager`, and `PrefabUtility` are supported
for the members listed as Supported in the API reference owned by AUT-006 in
[Automation and MCP](automation-and-mcp.md); any other editor member is
reported Unsupported naming the member when called. Edits made through these
APIs join undo history (EDT-005) and are attributed to the person or agent
session that invoked the script (EDT-008), as defined in
[Editor and authoring](editor-and-authoring.md). Modal dialogs and progress
bars raised in headless or agent-driven sessions never block: they are
reported as structured events and return the default answer that the session
declares.

### Custom editor drawing and gizmos

Drawing custom inspectors, property drawers, and editor windows is
Unsupported, as declared in the [Index](INDEX.md); affected objects use the
default inspector presentation, and the report names each custom drawer that
is not drawn. `OnDrawGizmos`, `OnDrawGizmosSelected`, and `Gizmos` drawing
calls are drawn in the scene view for review (EDT-003) and captured as
structured data (DIA-001 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)).

## Jobs, Burst-compiled code, and native collections

### Jobs

`IJob`, `IJobFor` (`Run`, `Schedule`, and `ScheduleParallel`),
`IJobParallelFor` with batch counts, and `IJobParallelForTransform` with
`TransformAccessArray` execute with Unity's semantics. `JobHandle`
dependencies, `CombineDependencies`, `Complete`, `IsCompleted`, and
`ScheduleBatchedJobs` behave as in Unity; completing a handle completes its
dependencies first. Unity APIs restricted to the main thread are unavailable
inside jobs as defined in [Threading and async/await](#threading-and-asyncawait).

### Native containers

`NativeArray` with `Temp`, `TempJob`, and `Persistent` allocators and the
Collections package containers (scoped by CMP-002) provide Unity's semantics
for allocation, disposal, and access attributes such as `[ReadOnly]`,
`[WriteOnly]`, `[NativeDisableParallelForRestriction]`, and
`[DeallocateOnJobCompletion]`.

In the editor and in development players, safety checks raise Unity's
exceptions for conflicting access between scheduled jobs and the main thread,
writes outside a parallel job's permitted range, access after disposal, and
writes to read-only containers, naming the job and container; undisposed
allocations are reported as leaks with their allocation site where available.
Release players omit these checks, as in Unity.

### Burst and Unity.Mathematics

Code marked `[BurstCompile]`, including Burst function pointers, produces
results equivalent to Unity within the floating-point mode and precision the
attribute declares. Constructs that Burst rejects, such as managed object
access, are reported as compile diagnostics naming the method. Whether native
acceleration is available on a target is reported as a performance property
(PRF-001), not as a compatibility failure; without it, the code runs with the
same results. `Unity.Mathematics` types and functions have the same semantics
in managed and Burst-compiled code.

On targets without worker threads, such as the browser, jobs run on the main
thread with the same results, and the lack of parallelism is reported as
Platform-limited.

## Logging and exception behavior

Log entries expose severity, message, time, frame, stack information, runtime
instance, originating object, and player connection where available. Repeated
messages may be grouped without losing their count or distinct context.
Presentation and navigation of logs are owned by DIA-001 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

Unhandled exceptions remain visible. Development players preserve enough
context to identify the responsible script and execution point. Release-player
diagnostics accurately reflect the selected product settings and never imply
that an exception did not occur merely because extended details were omitted.

## Acceptance stories

1. A supported Unity MonoBehaviour compiles, attaches to the intended object,
   retains inspector values, and receives its expected lifecycle callbacks.
   (RUN-002, RUN-003, RUN-007)
2. Cloning and destroying an object produces compatible identity, reference,
   callback, and frame-boundary behavior, and a retained reference to the
   destroyed object compares equal to null through Unity's operators while
   `?.` observes the managed reference. (RUN-001)
3. Scaled and unscaled coroutines remain distinguishable through pause and
   time-scale changes. (RUN-004)
4. A failed script edit identifies its source location and does not destroy the
   last inspectable valid project state; fixing it recompiles only the affected
   assemblies and their dependents. (RUN-007)
5. Unsupported APIs and values are reported precisely instead of behaving as
   successful no-ops. (RUN-002)
6. A project with scripts under `Plugins`, `Editor`, and assembly definitions
   using references, platform exclusion, define constraints, version defines,
   and an `.asmref` compiles into the same assemblies Unity produces, and a
   reference cycle is reported naming both definitions. (RUN-007)
7. Code guarded by `UNITY_EDITOR`, `UNITY_2021_3_OR_NEWER`, a platform symbol,
   `DEVELOPMENT_BUILD`, and a `csc.rsp` define compiles in or out exactly as in
   Unity for the editor and for a release and a development player of two
   targets. (RUN-007)
8. A C# 9 feature in a Unity 2020.3 project is diagnosed as Unity diagnoses it,
   and the same source compiles in a Unity 2022.3 project. (RUN-002, RUN-007)
9. A trusted project's source generator adds types that gameplay scripts use,
   and its analyzer reports a warning with the analyzer's identifier; in the
   same project untrusted, neither runs and the report says so. (RUN-007)
10. A native plugin excluded for the running architecture causes a
    `DllImport` call to fail with a diagnostic naming the library, target,
    architecture, and calling member. (RUN-007)
11. Two script types ordered through the Script Execution Order setting and
    `[DefaultExecutionOrder]` receive `Awake`, `Start`, and `Update` in the
    configured order, and the setting overrides the attribute. (RUN-003)
12. Adding a component with `[RequireComponent]` adds its dependency, removing
    that dependency is refused with a reason, and a second
    `[DisallowMultipleComponent]` component is refused for a person and for an
    agent alike. (RUN-003)
13. `[RuntimeInitializeOnLoadMethod]` methods with each load type run once in
    load-type order, and with domain reload disabled a static counter retains
    its value across play sessions until a `SubsystemRegistration` method
    resets it. (RUN-003)
14. Application pause, focus, and quit messages, `OnBecameVisible`, and
    `OnMouseDown` with the legacy input manager arrive in Unity's
    circumstances and order, and `OnMouseDown` does not arrive when only the
    Input System is active. (RUN-003)
15. Calling a main-thread-only Unity API from a task raises an exception
    naming the API; an awaited delay started on the main thread resumes on the
    main thread in the frame loop; a task still running when play ends is
    reported with its origin. (RUN-004)
16. A browser build of a project that creates a thread reports the API and
    calling member as Platform-limited. (RUN-004)
17. `LoadSceneAsync` with `allowSceneActivation` false holds at 0.9 progress
    until released, after which `Awake`, `OnEnable`, `sceneLoaded`, and `Start`
    occur in Unity's order, and a single load preserves only
    `DontDestroyOnLoad` objects. (RUN-005)
18. Loading an ambiguous scene name loads Unity's choice and lists every match;
    loading a scene absent from the build list fails with Unity's error and
    leaves loaded scenes unchanged; unloading the only scene is refused.
    (RUN-005)
19. `Resources.Load` of a missing path returns null without an exception, and
    `Resources.UnloadUnusedAssets` releases an asset once no reference
    remains, with residency visible through automation. (RUN-006)
20. An AssetBundle with a dependency loads correctly once its dependency is
    loaded, reports the absent bundle otherwise, and `Unload(false)` followed
    by reload yields duplicate assets as in Unity. (RUN-006)
21. Addressables loads by key and label update reference counts, content
    unloads when the last handle is released, and a remote catalog update is
    detected and applied. (RUN-006)
22. A button's persistent listener with a string argument and a dynamic
    listener run in the defined order, and a persistent listener whose method
    was renamed is reported naming the owner, field, target, and method.
    (RUN-008)
23. `SendMessage` with `RequireReceiver` and no receiver logs Unity's error,
    and `InvokeRepeating` stops firing while time scale is zero and stops
    permanently when its owner is destroyed. (RUN-008)
24. A `UnityWebRequest` GET with a timeout reports success, a 404 as a
    protocol error with its code, an unreachable host as a connection error,
    and a cross-origin denial in a browser build as Platform-limited.
    (RUN-009)
25. `JsonUtility` round-trips a class with private `[SerializeField]` fields
    and skips its properties, and `Random.InitState` with a fixed seed yields
    Unity's sequence for integer and float `Range`. (RUN-009)
26. `Application.persistentDataPath` resolves per target to Unity's location
    for the project's company and product names. (RUN-009)
27. A `[MenuItem]` command with a validation function appears in the command
    palette and as an automation tool, is disabled with its reason when
    validation fails, and its `AssetDatabase` edits are undoable and
    attributed to the invoking agent session. (RUN-010)
28. An `AssetPostprocessor` and a `ScriptedImporter` run during import in a
    trusted project and not in an untrusted one, and a custom inspector is
    reported Unsupported while the object's default inspector remains
    editable. (RUN-010)
29. No `UnityEditor` assembly is present in a built player, and a runtime
    script that references `UnityEditor` unguarded fails the build with the
    file and member named. (RUN-007, RUN-010)
30. An `IJobParallelFor` chain with dependencies produces the same results as
    Unity; reading its `NativeArray` on the main thread before completion
    raises the safety exception in a development player; an undisposed
    allocation is reported as a leak. (RUN-011)
31. A `[BurstCompile]` job produces results equivalent to its managed
    execution within its declared floating-point mode, and on a target
    without native acceleration the report lists that as a performance
    property only. (RUN-011)
