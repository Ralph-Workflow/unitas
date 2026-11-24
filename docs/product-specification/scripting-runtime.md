# Scripting and Runtime Product Specification

## Scope

This specification defines the observable C# scripting and runtime behavior of
unitas. Compatibility means that supported Unity scripts retain their public
meaning, not merely that similarly named types compile.

## Script authoring experience

As a gameplay programmer, I can add supported C# scripts to an existing project
and use them as components, serialized assets, editor-visible types, and runtime
behavior without changing Unity namespaces or ordinary source patterns.

The product:

* recognizes supported script types after a successful compilation;
* reports errors with file, line, column, severity, and message;
* keeps the last valid project state inspectable when a later edit fails;
* preserves serialized field values when type and field identities remain
  compatible;
* distinguishes a missing script from a script that compiled but failed at
  runtime;
* reports ambiguous, duplicate, unavailable, or incompatible types explicitly;
* exposes compilation state to both the visual editor and authorized agents.

One script error does not appear as an asset-import, scene, or build error unless
that operation is genuinely affected by the script failure.

## Runtime type behavior

The end-state scripting profile includes the C# 9 and .NET Standard 2.1 surface
declared by unitas. Compatibility reports distinguish this unitas profile from
the language and API profiles actually available in each supported Unity
project version. A supported type or member has compatible
effects, return values, exceptions, null behavior, timing, object lifetime, and
platform limits.

Core compatibility covers Object, GameObject, Component, Behaviour,
MonoBehaviour, Transform, ScriptableObject, common mathematical value types,
Time, Debug, Application, Screen, Resources, PlayerPrefs, instantiation,
destruction, discovery, and component access within the declared product scope.

Where an API is unavailable, the product identifies the exact type or member
and its consequence. An inert placeholder is not reported as support.

## Object identity and lifetime

As a programmer, I can rely on stable runtime identity for live objects.
Instantiation produces a distinct object graph with supported serialized values
and internal references preserved. Destruction makes the object unavailable at
the compatible frame boundary and produces compatible comparison and null-like
behavior for retained managed references.

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

Instantiation, enablement changes, hierarchy activation, scene loading,
scene unloading, destruction, application focus, and shutdown produce
inspectable event sequences. Exceptions in one callback identify the script,
method, object, scene, and frame and do not masquerade as successful execution.

## Time and frame behavior

Time exposes frame delta, unscaled delta, fixed delta, elapsed time, frame count,
time scale, real time, and supported smoothed values with compatible
relationships. Pausing gameplay, changing time scale, stepping a frame, and
running multiple fixed steps in one rendered frame have defined effects.

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
operation.

The coroutine ends when it completes, is stopped through a compatible API, its
owner is destroyed or made inactive under the compatible rule, or another
compatible lifetime rule applies. Disabling a MonoBehaviour and deactivating
its GameObject remain distinct cases and do not implicitly share stop behavior.
Yielded nulls, already-completed operations, nested enumerators, destroyed
objects, predicate exceptions, and scene unloading all have explicit behavior.

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

## ScriptableObject behavior

As a developer, I can create ScriptableObject assets, reference them from scenes
and other assets, inspect and edit their supported fields, instantiate temporary
runtime copies, and preserve asset identity across moves and renames.

Changing a runtime copy does not silently mutate the project asset. Shared asset
references continue to refer to the same asset unless explicitly replaced.

## Logging and exception behavior

Log entries expose severity, message, time, frame, stack information, runtime
instance, originating object, and player connection where available. Repeated
messages may be grouped without losing their count or distinct context.

Unhandled exceptions remain visible. Development players preserve enough
context to identify the responsible script and execution point. Release-player
diagnostics accurately reflect the selected product settings and never imply
that an exception did not occur merely because extended details were omitted.

## Acceptance stories

1. A supported Unity MonoBehaviour compiles, attaches to the intended object,
   retains inspector values, and receives its expected lifecycle callbacks.
2. Cloning and destroying an object produces compatible identity, reference,
   callback, and frame-boundary behavior.
3. Scaled and unscaled coroutines remain distinguishable through pause and
   time-scale changes.
4. A failed script edit identifies its source location and does not destroy the
   last inspectable valid project state.
5. Unsupported APIs and values are reported precisely instead of behaving as
   successful no-ops.

