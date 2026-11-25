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
Project code runs with the privileges of the user account running unitas and is
not sandboxed (PRJ-008).

Play, agent, and headless sessions are defined by AUT-004 in
[Automation and MCP](automation-and-mcp.md).

### Compatibility tiers

RUN-001, RUN-002, RUN-003, RUN-004, RUN-005, RUN-006, RUN-007, RUN-008,
RUN-009, RUN-010, and RUN-011 are all in the Semantic tier, as recorded in the
[Feature catalog](feature-catalog.md) and defined in the
[Compatibility contract](compatibility-contract.md). Throughout this specification, "as in Unity" and "Unity's"
behavior, rules, errors, and ordering mean the documented, observable behavior
of Unity 2022.3 LTS in ordinary use. Undocumented quirks are not promised; a
found difference is reported as Partially supported naming the difference.
Projects saved by earlier Unity releases are compiled and run with the 2022.3
LTS behavior after their serialized forms migrate (SER-005 in
[Serialization and prefabs](serialization-and-prefabs.md)).

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
  Unity version matches a version range; the Unity version evaluated is the
  project's Unity 2022.3 LTS version.

Assembly definition reference (`.asmref`) files add their folder's scripts to
the referenced assembly. Duplicate assembly names, reference cycles, references
to missing assemblies, scripts claimed by more than one definition, and
invalid version ranges are reported with the defining file and the affected
assemblies. An assembly excluded on the current target by platform or define
constraints is reported as excluded, not as missing, and its dependents report
the unresolved references.

### Defines, compiler options, and language profile

The compilation for each assembly and target defines the symbols Unity 2022.3
LTS defines for the selected target, including:

* `UNITY_EDITOR` and the host-specific editor symbols in the editor and in
  headless sessions, and never in players;
* the Unity 2022.3 version symbols, including `UNITY_2022_3`,
  `UNITY_2022_3_OR_NEWER`, and every earlier `_OR_NEWER` symbol Unity 2022.3
  defines, such as `UNITY_2020_3_OR_NEWER` and `UNITY_2021_3_OR_NEWER`;
* platform symbols such as `UNITY_STANDALONE`, `UNITY_STANDALONE_WIN`,
  `UNITY_SERVER` for the Dedicated Server target, `UNITY_IOS`,
  `UNITY_ANDROID`, and `UNITY_WEBGL`;
* `DEVELOPMENT_BUILD` in development players;
* scripting backend and API compatibility symbols such as `ENABLE_MONO`,
  `ENABLE_IL2CPP`, `NET_STANDARD_2_1`, `NET_STANDARD`, and `NET_4_6`;
* the scripting define symbols set per target in Player settings (PRJ-007 in
  [Projects and assets](project-and-assets.md));
* symbols added by version defines and by response files.

A `csc.rsp` response file contributes defines, warning suppression, warnings
as errors, and other compiler options Unity 2022.3 accepts; an option unitas
does not honor is reported naming the option and file.

The C# language version is C# 9. The API compatibility level is .NET Standard
2.1 or .NET Framework, as the project's Player settings choose per target.
C# 9 features that Unity 2022.3's compiler does not support, such as covariant
return types, are diagnosed as in Unity. Language versions or APIs beyond this
profile are reported, never silently accepted.

### Native interop

On the targets that provide them, `AndroidJavaObject`, `AndroidJavaClass`,
`AndroidJavaProxy`, `AndroidJNI`, and `AndroidJNIHelper`; `UnitySendMessage`
calls from native code; reverse callbacks marked `[MonoPInvokeCallback]` under
ahead-of-time compilation (BLD-003 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md)); and
browser JavaScript calls into managed code through `SendMessage` behave as in
Unity. On other targets, including the editor host where Unity also lacks them,
these APIs raise Unity's exception, and the build report lists each use as
Platform-limited naming the calling member.

### Analyzers and source generators

Roslyn analyzers and source generators labeled for use as analyzers run against
the assemblies that Unity 2022.3's analyzer scoping rules apply them to, and
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

The scripting profile for a project is the C# language version and the API
compatibility level defined in
[Defines, compiler options, and language profile](#defines-compiler-options-and-language-profile),
with the API compatibility level that the project's Player settings choose.
Compatibility reports state the profile in effect for the project. A supported
type or member has compatible
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
  orientation, safe area, cutouts, sleep timeout, and autorotation), and
  `Cursor` (visibility, lock state, and custom cursors);
* `Resources` (see [Resource loading](#resource-loading-and-content-catalogs))
  and `PlayerPrefs`, whose storage behavior is owned by SAV-001 in
  [Accessibility, localization, and persistence](accessibility-localization-persistence.md).

Results of floating-point operations are equivalent within the variation the
[Compatibility contract](compatibility-contract.md) allows; exact bitwise
parity is not claimed. Cursor lock, cursor images, and screen orientation are
Platform-limited where the target restricts them, for example browser cursor
lock requiring a user gesture.

### Application, system, and device information

`Application.isPlaying`, `isEditor`, `platform`, `unityVersion`, `version`,
`identifier`, `productName`, `runInBackground`, `internetReachability`,
`systemLanguage`, `isMobilePlatform`, `absoluteURL`, and `deepLinkActivated`
return Unity's values for the running context:

* In the editor and in headless sessions, `platform` reports the host editor
  value (`WindowsEditor`, `OSXEditor`, or `LinuxEditor`).
* `unityVersion` reports the project's Unity 2022.3 LTS version, including
  for a project migrated from an earlier release, never a unitas version.
* `Application.Quit` is ignored during an editor play session and on targets
  that forbid an application from quitting itself, as in Unity, and otherwise
  ends the player with the quit messages defined in
  [Additional messages](#additional-messages).
* `Application.OpenURL` opens the URL through the platform where the target
  permits it.

`SystemInfo` reports the actual host or device; with no graphics device, such
as in a headless session without rendering, `graphicsDeviceType` is `Null`.
`Screen.sleepTimeout`, autorotation settings, and `Handheld.Vibrate` on a
target or host that lacks the capability return or report an explicit
unavailable status instead of appearing to succeed.

### Logging APIs

`Debug.Log`, `LogWarning`, `LogError`, `LogException`, `LogAssertion`, and
their format variants produce entries with type, message, context object, and
stack trace. `Application.logMessageReceived` delivers every entry on the main
thread and `Application.logMessageReceivedThreaded` delivers every entry from
any thread, each with its type and stack trace. `Debug.unityLogger` honors
`logEnabled` and `filterLogType`, and its `logHandler` can be replaced.
`Debug.Assert` calls are compiled out when `UNITY_ASSERTIONS` is not defined.
`Debug.Break` pauses an editor play session after the current frame and has no
effect in players. `Application.SetStackTraceLogType` and the per-log-type
stack trace Player settings determine the stack detail recorded. Players write
their log file to the location Unity uses on each target and honor the
`-logFile` command-line argument. Collecting player logs from devices is owned
by DIA-003 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

### Recognized unsupported content

Visual Scripting graphs, script machines, and state machines are recognized
and reported Unsupported, as declared in the [Index](INDEX.md). Their graphs do
not execute; the GameObjects that carry them, and every other component on
those objects, remain usable, and the report names each affected object.

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

The discovery and component APIs behave as in Unity:

* `Object.FindObjectOfType` and `FindObjectsOfType`, with and without
  `includeInactive`, `FindObjectsByType`, `FindFirstObjectByType`, and
  `FindAnyObjectByType`;
* `GameObject.Find`, which returns active objects only and accepts
  hierarchy path syntax;
* `GameObject.FindWithTag`, `FindGameObjectsWithTag`, and `CompareTag`, which
  raise Unity's error for a tag that is not defined in the project;
* `Resources.FindObjectsOfTypeAll`, which includes inactive, hidden, and asset
  objects;
* `Transform.Find` by child name or relative path;
* `GetComponent`, `GetComponents`, `GetComponentInChildren`,
  `GetComponentInParent`, and their plural forms, with `includeInactive`, and
  `TryGetComponent`.

In the editor and editor play sessions, an unassigned serialized reference or
an absent requested component returns Unity's editor placeholder object, so
member access raises `UnassignedReferenceException` or
`MissingComponentException` naming the field or component; in players the same
access raises `NullReferenceException`, as in Unity.

### Transform behavior

`Transform.SetParent` with `worldPositionStays` true preserves world position,
rotation, and scale as closely as Unity does, and with false preserves local
values. `lossyScale` approximates scale under skew as Unity does. A negative
scale mirrors the object and reverses triangle winding for rendering and
physics as in Unity. Sibling index operations (`SetSiblingIndex`,
`SetAsFirstSibling`, `SetAsLastSibling`, `GetSiblingIndex`) define hierarchy
order. `Transform.hasChanged` becomes true when Unity sets it and stays true
until script code resets it.

### Instantiation and cloning

`Object.Instantiate` with and without parent, position, and rotation clones
GameObjects, components, ScriptableObjects, and other assets at runtime. The
clone's name receives Unity's `(Clone)` suffix; references to objects inside the
cloned hierarchy are remapped to their clones; references to objects outside
it are kept; and `Awake` and `OnEnable` run on active clones before
`Instantiate` returns. Serialized-value preservation in clones follows SER-001
in [Serialization and prefabs](serialization-and-prefabs.md).

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
* `OnAnimatorMove` and `OnAnimatorIK`, owned by ANI-002 in
  [Rendering and animation](rendering-and-animation.md).

Rendering and camera messages such as `OnRenderObject`, `OnWillRenderObject`,
`OnPreCull`, `OnPreRender`, `OnPostRender`, and `OnRenderImage` are owned by
REN-006 in [Rendering and animation](rendering-and-animation.md).

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
  running background work retain their values from the previous play session, and
  only explicit resets such as `SubsystemRegistration` initialization methods
  clear them. A player loop modified through `PlayerLoop.SetPlayerLoop`
  persists into the next play session, as in Unity.
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

`Time.captureFramerate` set above zero makes each rendered frame advance game
time by exactly one over that value, regardless of wall-clock time, as in
Unity. Running a headless play session on a fixed simulated frame delta, and
the conditions for stepping it, are the time control owned by AUT-002 in
[Automation and MCP](automation-and-mcp.md).

As a developer, I can distinguish:

* rendered frames from fixed simulation steps;
* scaled from unscaled elapsed time;
* editor pause from a game's own pause state;
* runtime time from edit-time preview time;
* a slow frame from an intentional time-scale change.

### Player loop

`PlayerLoop.GetDefaultPlayerLoop`, `PlayerLoop.GetCurrentPlayerLoop`, and
`PlayerLoop.SetPlayerLoop` expose the `PlayerLoopSystem` tree with the
subsystem types of Unity 2022.3 LTS. A system inserted by script
code runs at its position in every frame of editor play sessions, headless
play sessions, and players. Removing or reordering built-in systems has the
effect it has in Unity, including the loss of the work those systems perform.
The effective player loop is inspectable in the editor and through automation.

## Coroutines and asynchronous operations

A MonoBehaviour can start, enumerate, and stop supported coroutines. A
coroutine can wait for the next frame, a fixed update, end of frame, a scaled or
unscaled duration, a predicate, another coroutine, or a supported asynchronous
operation, including custom `CustomYieldInstruction` subclasses.

A coroutine ends when one of the following occurs:

* its enumerator completes or throws, in which case the exception is logged
  naming the script, object, and coroutine;
* it is stopped by `StopCoroutine` or `StopAllCoroutines` on its owner;
* its owning MonoBehaviour is destroyed, including by unloading its scene;
* its owner's GameObject becomes inactive in the hierarchy.

Setting the owning MonoBehaviour's `enabled` to false does not stop its
coroutines. `StartCoroutine` on a MonoBehaviour whose GameObject is inactive
fails with Unity's error naming the object, and no coroutine starts.
`StopCoroutine` with a method name stops only coroutines that were started by
name; coroutines started from an enumerator or `Coroutine` value are stopped
through that value.

Resumption points follow Unity's frame order:

* `yield return null` and a yielded integer resume after `Update` in the next
  frame;
* `WaitForFixedUpdate` resumes after the next fixed simulation step;
* `WaitForEndOfFrame` resumes after the frame has rendered; in a headless play
  session without rendering, it resumes at the point in the frame where
  rendering would have occurred;
* `WaitForSeconds` uses scaled time, and `WaitForSecondsRealtime` unscaled
  time, each evaluated at Unity's point in the frame;
* `WaitUntil` and `WaitWhile` evaluate their predicates once per frame at
  Unity's point, and an exception in a predicate ends the coroutine with the
  exception logged;
* a nested coroutine or yielded enumerator resumes its caller after it
  completes;
* an already-completed asynchronous operation resumes the coroutine at the
  next resumption point, not synchronously.

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
reported as such. Ending a play session does not silently cancel such work, as
in Unity.

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
  scene, where it survives single loads until the play session ends; applied to a
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
* A bundle built for another target, built by Unity rather than unitas
  (BLD-005 in
  [Builds, platforms, XR, and networking](build-platforms-xr-networking.md)),
  or with a serialized form unitas cannot read (SER-005 in [Serialization and prefabs](serialization-and-prefabs.md)),
  is reported Unsupported naming the bundle and reason rather than partially
  loaded.

Bundles are built and read with LZMA, LZ4, or no compression.
`AssetBundle.RecompressAssetBundleAsync` converts between compression forms.
A load with a CRC value fails on a mismatch with a diagnostic naming the
bundle. `UnityWebRequestAssetBundle` with a version, hash, or
`CachedAssetBundle` stores and reuses downloads through `Caching`, which exposes
cache paths, `ClearCache`, and space limits as in Unity; a cache that cannot be
written on the target is reported with the target and path.

### Addressables

With the Addressables package (scoped by CMP-002 in the
[Compatibility contract](compatibility-contract.md)), assets load by address
key, label, `AssetReference`, and resource location. In players, Addressables
content and catalogs are the ones unitas built (BLD-005); content built by
Unity is reported Unsupported naming the catalog or bundle. Load, instantiate, and
download operations return handles with status, progress, result, and
exception. Each load increments a reference count; `Release` and
`ReleaseInstance` decrement it, and content unloads when its count reaches zero.
Releasing an invalid or already-released handle is reported. Local and remote
catalogs, catalog update checks, content updates, download size queries, and
dependency downloads behave as the package defines, and an invalid key fails
with the package's exception. `WaitForCompletion` completes an operation
synchronously where the package version allows it; on the browser it is
Platform-limited and reported naming the calling member. In the editor, the
selected Addressables play mode script (asset database, simulated groups, or
existing build) is honored and reported.

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
error with the response code, and data processing error, reported
through `UnityWebRequest.Result`.

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
`insideUnitSphere` and `rotation` are equivalent within the floating-point
variation the [Compatibility contract](compatibility-contract.md) allows.
Integer `Range` excludes its maximum and floating-point `Range` includes it,
as in Unity. `System.Random` follows .NET behavior.

## Serialization-facing script behavior

Which script fields persist, which value kinds are supported, and how
`[SerializeField]`, `[HideInInspector]`, `[NonSerialized]`,
`[SerializeReference]`, and `[FormerlySerializedAs]` affect storage are owned
by SER-001 in [Serialization and prefabs](serialization-and-prefabs.md).
Presentation of serialized fields in the inspector is owned by EDT-002 in
[Editor and authoring](editor-and-authoring.md).

## ScriptableObject behavior

ScriptableObject asset identity, duplication, and the separation between a
project asset and its runtime instances are owned by SER-001 in
[Serialization and prefabs](serialization-and-prefabs.md); runtime cloning is
defined in [Instantiation and cloning](#instantiation-and-cloning).

`[CreateAssetMenu]` offers the type for creation in the editor and through
automation. Changes that script code makes to a project ScriptableObject asset
during an editor play session persist after the play session ends, as in
Unity, and are shown as unsaved asset changes.

ScriptableObjects receive `Awake`, `OnEnable`, `OnDisable`, `OnDestroy`, and
`OnValidate` at Unity's points: when the asset loads, when it unloads, around
each editor domain reload, and on `ScriptableObject.CreateInstance`. In
players, only ScriptableObjects referenced by included content, `Resources`
folders, loaded bundles or Addressables content, or the Preloaded Assets
Player setting exist, and changes made to them at player time are lost when
the player restarts.

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

At minimum, the following editor APIs are Supported:

* `AssetDatabase` create, import, refresh, move, rename, copy, delete, find,
  load, and GUID-to-path conversion;
* `EditorUtility.SetDirty`, dialogs, and progress bars;
* `SerializedObject` and `SerializedProperty` reading and writing, including
  `ApplyModifiedProperties` and `Update`;
* `Undo.RecordObject` and `Undo.RegisterCreatedObjectUndo`;
* `EditorPrefs`;
* `EditorSceneManager` scene opening and saving;
* `PrefabUtility` instantiation, applying, reverting, and unpacking.

Every other editor member carries its own compatibility state in the API
reference owned by AUT-006 in [Automation and MCP](automation-and-mcp.md), and
calling an Unsupported member reports the member. Edits made through these
APIs join undo history (EDT-005) and are attributed to the person or agent
session that invoked the script (EDT-008), as defined in
[Editor and authoring](editor-and-authoring.md).

In a headless session or when an agent session invoked the script, modal
dialogs and progress bars never block. Each is reported as a structured event,
and a dialog returns the answer supplied for it as defined by AUT-002 in
[Automation and MCP](automation-and-mcp.md): a play session, an editor-method
invocation, or a command-line invocation can supply answers by dialog title and
button, and each dialog and the answer used is recorded. A dialog with no
supplied answer returns its default button (`ok` for
`EditorUtility.DisplayDialog`).

### Build scripting API

`BuildPipeline.BuildPlayer` with `BuildPlayerOptions` produces the same build
as an equivalent build request made through unitas (BLD-001 in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md)) and
returns a `BuildReport` with its summary, steps, and messages.
`IPreprocessBuildWithReport`, `IPostprocessBuildWithReport`,
`IProcessSceneWithReport`, `IFilterBuildAssemblies`, and `[PostProcessBuild]`
callbacks run in Unity's order, honoring `callbackOrder`. An exception in a
build callback fails the build with a diagnostic naming the callback type and
method. `EditorUserBuildSettings` and `PlayerSettings` members read and change
the project settings owned by PRJ-007 in
[Projects and assets](project-and-assets.md), and changes are stored in
Unity-readable form.

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
allocations are reported as leaks, with their allocation site when leak
detection with full stack traces is enabled. Release players omit these
checks, as in Unity.

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
thread with the same results, and the absence of parallelism is reported as a
performance property (PRF-001), not as a compatibility limitation.

### Entities content

Entities and Entities Graphics (`com.unity.rendering.hybrid` and
`com.unity.entities.graphics`) content is recognized and reported Unsupported, as declared
in the [Index](INDEX.md), naming each affected asset, subscene, and script.
Jobs, Burst-compiled code, and native containers that do not depend on those
packages remain Supported.

## Logging and exception behavior

Log entries expose severity, message, time, frame, stack information, runtime
instance, originating object, and, for entries received from a connected
player, that player connection. Repeated messages may be grouped without
losing their count or distinct context.
Presentation and navigation of logs are owned by DIA-001 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md).

Unhandled exceptions remain visible. Development players preserve enough
context to identify the responsible script and execution point. Release-player
diagnostics accurately reflect the selected product settings and never imply
that an exception did not occur merely because extended details were omitted.

## Acceptance stories

1. A supported Unity MonoBehaviour compiles, attaches to the intended object,
   retains inspector values, and receives its expected lifecycle callbacks.
   (RUN-002, RUN-003, RUN-007, SER-001)
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
7. Code guarded by `UNITY_EDITOR`, `UNITY_2022_3_OR_NEWER`, a platform
   symbol including `UNITY_SERVER`, `DEVELOPMENT_BUILD`, and a `csc.rsp`
   define compiles in or out exactly as in Unity 2022.3 for the editor and for
   a release and a development player of two targets. (RUN-007)
8. A project migrated from Unity 2020.3 compiles with C# 9 and the Unity 2022.3
   defines, and a covariant return type is diagnosed as Unity 2022.3
   diagnoses it. (RUN-002, RUN-007)
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
32. A coroutine keeps running after its MonoBehaviour's `enabled` is set to
    false and stops when its GameObject is deactivated; `StartCoroutine` on an
    inactive GameObject fails with Unity's error naming the object; a
    by-name `StopCoroutine` leaves enumerator-started coroutines running; and
    `WaitForEndOfFrame` resumes in a headless play session without rendering.
    (RUN-004)
33. A custom system inserted with `PlayerLoop.SetPlayerLoop` runs at its
    position every frame in an editor play session, a headless play session,
    and a player, and with domain reload disabled it remains installed in the
    next play session. (RUN-003, RUN-004)
34. With `Time.captureFramerate` set to 30, each rendered frame advances game
    time by one thirtieth of a second regardless of wall-clock duration.
    (RUN-004)
35. In a headless session, `Application.platform` reports the host editor
    value, `Application.unityVersion` reports the project's 2022.3 LTS version,
    `Application.Quit` is ignored during play, and `SystemInfo.graphicsDeviceType`
    is `Null` without a graphics device. (RUN-002)
36. A handler on `Application.logMessageReceivedThreaded` receives entries
    logged from a worker thread with type and stack trace, `Debug.Assert` is
    absent from a build without `UNITY_ASSERTIONS`, and a player honors
    `-logFile`. (RUN-002)
37. A Visual Scripting script machine and an Entities subscene are each
    reported Unsupported naming the affected objects, while the GameObject that
    carries the script machine, and a job using `NativeArray` in the same
    project, keep working. (RUN-002, RUN-011)
38. On an Android player, an `AndroidJavaClass` call reaches Java and an
    `AndroidJavaProxy` callback reaches managed code; on a desktop build, the
    same calls are listed as Platform-limited naming the calling member, and an
    iOS build's `[MonoPInvokeCallback]` reverse callback is invoked under
    ahead-of-time compilation. (RUN-007)
39. `GameObject.Find` ignores an inactive object that
    `FindObjectsOfType(true)` returns, `CompareTag` with an undefined tag
    raises Unity's error, and accessing an unassigned serialized reference
    raises `UnassignedReferenceException` in an editor play session and
    `NullReferenceException` in a player. (RUN-001)
40. `SetParent` with `worldPositionStays` true keeps the world pose of a child
    under a negatively scaled parent, sibling index changes reorder the
    hierarchy, and an instantiated hierarchy's internal references point to
    the clones while external references are kept. (RUN-001)
41. A ScriptableObject created with `CreateInstance` receives `Awake` and
    `OnEnable`; a ScriptableObject referenced only by Preloaded Assets exists
    in a player; and a change made at player time is gone after restart.
    (RUN-001)
42. An LZ4 AssetBundle downloaded with a version hash is cached and reused on
    the next request, a CRC mismatch fails naming the bundle, and
    Addressables `WaitForCompletion` in a browser build is reported
    Platform-limited. (RUN-006)
43. A `BuildPipeline.BuildPlayer` call from a `[MenuItem]` command produces the
    same build as the equivalent unitas build request, runs preprocess,
    scene-processing, and postprocess callbacks in `callbackOrder`, returns a
    `BuildReport`, and a throwing preprocess callback fails the build naming
    it. (RUN-010)
44. An editor script that calls `EditorUtility.DisplayDialog` during an agent
    session returns `ok` without blocking unless the agent session supplied an
    answer, and the dialog is reported as a structured event. (RUN-010)
45. On a browser build, an `IJobParallelFor` job runs on the main thread with
    the same results as on a desktop build, and the report lists the absence
    of parallelism as a performance property. (RUN-011)
