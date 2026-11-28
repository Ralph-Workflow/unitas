# Web Target

## Scope

This specification details the Web target, the browser delivery of PLT-001,
which [Builds, platforms, XR, and networking](build-platforms-xr-networking.md)
owns. It defines the browser bundle and the page that hosts it: page
lifecycle, graphics API selection, the WebAssembly player's memory, threads,
SIMD, exceptions, and symbols, build output, templates, and the loader API,
compression, caching, and hosting headers, embedding, JavaScript interop,
browser display, input, audio, video, storage, and networking, the browser
support matrix, web performance and budgets, diagnostics, and automated
browser testing. It owns no catalog feature; its acceptance outcomes
demonstrate PLT-001 together with the features they name. Build results,
ahead-of-time and stripping guarantees, content builds, development-player
connections, and licensing are defined once in
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md) and
apply to the Web target unchanged; this document states only what the browser
changes.

The Web target is Unity 2022.3 LTS's WebGL build target. It produces a browser
bundle: a WebAssembly player, its loader, its data, and a page that hosts it.
A project that Unity 2022.3 LTS builds for WebGL builds for the Web target with
its WebGL Player Settings, Build Settings, templates, and plug-ins honored. The
`UNITY_WEBGL` define is set, and `Application.platform` returns
`RuntimePlatform.WebGLPlayer`.

Behavior that Unity 2022.3 LTS offers on WebGL is the baseline. Behavior that
Unity 2022.3 LTS lacks there, such as WebGPU, managed threads, WebAssembly SIMD,
and script debugging, is a unitas extension, and the build result names each
extension a build uses. A limitation of Unity's WebGL player is never carried
over as a limitation of the Web target. Only a capability that the browser,
the operating system, or the device does not offer is reported as
Platform-limited, naming the browser, the version, and the missing capability.

The browser player contains no MCP interface. An agent reaches a running
browser player only through the editor's connection to a development player
(BLD-004).

## Architecture

This section applies [Engine architecture](architecture.md) and the Architecture section of
[Builds, platforms, XR, and networking](build-platforms-xr-networking.md#architecture) to the
browser.

* **Player:** the browser player is the target's generic player compiled to WebAssembly. The
  .NET WebAssembly toolchain compiles the project's managed code ahead of time into the same
  module, and an interpreter runs code generated at runtime. The page API is a front end over
  the player's own operations.
* **Loader:** the loader is the platform layer's bootstrap. It detects browser capabilities,
  then selects the module variant and the texture format set. It streams and compiles the
  module while data downloads, then starts the engine. Nothing above the platform layer
  queries the browser directly. Every capability enters through detection as supported,
  denied, or absent.
* **Platform layer:** the canvas and its graphics context, DOM input events, the Gamepad and
  sensor APIs, Web Audio, IndexedDB-backed storage, fetch, WebSocket, WebRTC, and Web Workers
  implement the platform interface. Media decoding the browser cannot do runs in the engine's
  own decoders, not in a separate browser-only path.
* **Frame loop:** the browser's animation frame callback paces the PlayerLoop. The player
  never runs a blocking loop on the page thread. Throttling of a hidden tab changes only how
  often the loop runs, and the loop bounds the time it publishes.
* **Threads:** on a cross-origin isolated page, Web Workers that share the WebAssembly memory
  serve the job system and .NET threads, as cores do on native targets. Without isolation,
  the same jobs run inline on the main thread. Browser APIs that only the page thread may
  call are reached from workers by forwarding the call; the worker waits and the page thread
  does not.
* **The page thread never blocks on the browser:** storage, fetch, and decoding are
  asynchronous. Synchronous managed APIs over asynchronous storage, such as `PlayerPrefs` and
  files under `persistentDataPath`, work on an in-memory copy that is persisted in the
  background, and the copy's durability is observable.
* **Graphics:** the render graph targets WebGPU or WebGL 2 through the same backends it uses
  on native targets. The backend chosen at startup is fixed for the session. On WebGL 2, the
  render graph lowers compute passes to passes the API can run; there is no second feature
  implementation. After a context or device loss, GPU resources are rebuilt by re-uploading
  cooked data held in memory or in the build's cache, never by reloading scenes.
* **Memory:** one linear WebAssembly memory holds the native and managed heaps. It grows only
  as the configured growth mode allows, and steady-state play does not grow it. GPU memory
  belongs to the browser and is accounted separately.
* **Content delivery:** build data is packaged into the same bundles as on other targets,
  split by scene and Addressables group. Bundles are streamed through fetch into an IndexedDB
  cache keyed by content hash, and the asset subsystem loads them asynchronously through its
  ordinary path.
* **Interop:** jslib functions are linked into the module's imports at build time. Numeric
  arguments cross the boundary without allocation. Strings and arrays are copied through the
  heap views, which stay valid across heap growth and shared memory.
* **Editor, headless, and player:** the development server, the browser test runner,
  symbolication, and the web build profile are editor commands, available headless and
  through the editor's MCP interface. The browser player contains no MCP and no editor code.
  Automated browser tests run the same Web build that ships.

## Session and page lifecycle

A browser player reports, to the page and to diagnostics:

* loading progress and each startup phase;
* successful startup;
* a missing browser capability, naming it;
* storage availability and persistence;
* whether audio is permitted, and that the next gesture enables it;
* focus changes, visibility changes, resizing, and fullscreen changes;
* unrecoverable startup and runtime errors, with their cause.

Browser security and permission restrictions are visible to the game and to
diagnostics. A permission the person denies returns an explicit status, never
apparent success.

Page refresh and navigation away end the player session. Data the game wrote to
persistent storage before that point remains available to the next session.
During page hide and unload, the player delivers `OnApplicationPause(true)` and
`OnApplicationQuit` to the extent the browser runs page code at that time, and
it finishes pending persistent writes as SAV-001 in
[Accessibility, localization, and persistence](accessibility-localization-persistence.md)
defines. A page restored from the browser's back-forward cache resumes the same
session and delivers `OnApplicationFocus` and `OnApplicationPause(false)`.

Offline state and its changes are reported to the game, through
`Application.internetReachability`, and to diagnostics.

`Application.Quit` stops the player, releases its memory, and leaves the page
open, as in Unity. The page's `Quit` call does the same (see
[Loader and page API](#loader-and-page-api)).

## Graphics APIs and selection

WebGL 2 is the Unity 2022.3 LTS baseline graphics API for the Web target.
WebGPU is a unitas extension. The WebGL target's Graphics APIs list in Player
Settings includes both.

* With Auto Graphics API on, the player tries WebGPU first and WebGL 2 second.
* With a manual list, the player tries the listed APIs in their order and
  never uses an API that the list omits.
* The player falls back to the next API when the browser does not offer the
  API, when the browser refuses an adapter or device, when the adapter lacks a
  feature or limit the build requires, or when device creation fails. The
  startup report names the API tried, the reason it was not used, and the API
  selected.
* The WebGL Power Preference Player setting (Default, Low Power, and High
  Performance) is passed to the browser's adapter and context requests.
* `SystemInfo.graphicsDeviceType` returns `OpenGLES3` under WebGL 2, as in
  Unity. `SystemInfo.graphicsDeviceVersion` and `graphicsDeviceName` name the
  active API, its version, and the adapter. Under WebGPU,
  `SystemInfo.graphicsDeviceType` returns the `GraphicsDeviceType` value that
  later Unity releases name `WebGPU`, which no Unity 2022.3 LTS member names, so
  a project compiled against Unity 2022.3 LTS sees an unnamed value;
  `graphicsDeviceVersion` begins with `WebGPU`, and the use of WebGPU is
  reported as a unitas extension.
* When neither API is available, startup fails before the engine loads its
  data, and the page receives an error that names the missing API. This is a
  real browser absence. It covers a browser with WebGL disabled and a GPU the
  browser blocks.

## Render pipelines and features on the web

Projects authored for the Built-in Render Pipeline and URP 14.x render on both
WebGPU and WebGL 2. HDRP 14.x and project-defined scriptable render pipelines
render on both as well, which is a unitas extension: Unity 2022.3 LTS does not
offer them on WebGL. The pipeline rules are owned by REN-006 in
[Rendering and animation](rendering-and-animation.md).

On both APIs, every rendering feature in that specification is available,
following its graphics API rule: a feature that the API lacks but the device
can perform is provided by other means.

* **Built-in Render Pipeline:** the forward, deferred, and legacy vertex-lit
  paths, realtime and baked lighting, shadows, Linear and Gamma color space,
  MSAA, HDR, GPU instancing, and Built-in post-processing render on both APIs.
* **URP 14.x:** the Forward, Forward+, and Deferred paths, camera stacking,
  the 2D Renderer, render scale, HDR, MSAA, decals, SSAO, renderer features,
  and the SRP Batcher render on both APIs.
* **Compute-dependent features:** `ComputeShader` dispatch, Visual Effect
  Graph, GPU culling, GPU skinning, and indirect drawing run natively on
  WebGPU. On WebGL 2, which has no compute stage, they run through the render
  graph's lowering of compute passes for backends without a compute stage, so
  no feature disappears on WebGL 2. Results match WebGPU within the capacities
  the build reports, and each capacity lower than on WebGPU is reported per
  feature and asset. `SystemInfo.supportsComputeShaders` returns true on both
  APIs. A per-target setting keeps Unity's WebGL value of false for projects
  that choose a fallback path from it.
* **Hardware ray tracing:** browsers offer no ray-tracing access, so ray-traced
  effects are Platform-limited on both APIs. The report names the effect and
  what renders instead.

Each feature's provision on the active API (direct, provided by other means,
or Platform-limited) is inspectable as REN-006 defines.

## Capability detection

At startup the player detects and reports:

* the graphics API, adapter, limits, optional features, and compressed texture
  formats;
* cross-origin isolation, thread availability, and the worker count;
* WebAssembly SIMD, exception handling, and 64-bit memory support;
* whether the page is a secure context;
* storage quota, persistence, and whether storage is session-only;
* audio state, gamepad access, fullscreen, pointer lock, keyboard lock, and
  clipboard access;
* the device pixel ratio, touch support, and device memory and processor count
  where the browser reports them.

`SystemInfo` members (`maxTextureSize`, `SupportsTextureFormat`,
`SupportsRenderTextureFormat`, `processorCount`, `systemMemorySize`,
`operatingSystem`, `deviceType`, and the rest) return values derived from this
detection. A value the browser hides is reported as unknown in diagnostics, and
the member returns Unity's documented fallback for it. The full capability
report is available in development diagnostics and through the editor's
connection to a development player.

## Lost graphics context

When the browser loses the WebGL 2 context or the WebGPU device, the player
stops rendering and keeps running game logic. When the browser restores the
context or provides a new device, the player re-creates every GPU resource and
resumes rendering on the next frame. Render texture contents are lost, as Unity
documents for device loss: `RenderTexture.IsCreated` returns false until the
texture is used or created again. Every other texture, mesh, buffer, and shader
renders as before.

If the browser does not restore the context, or re-creation fails, the player
reports an unrecoverable graphics error to the page and to diagnostics, naming
the cause. It stops rendering without freezing the page. Each loss and
restoration is recorded with its time and duration.

## Texture compression

The Web target supports ASTC, ETC2, and DXT (BC) texture compression, chosen per
browser and device.

* The Player Settings Texture Compression format and the Build Settings
  Texture Compression override (Use Player Settings, DXT, ETC2, and ASTC) are
  honored, along with per-texture format overrides for the WebGL target.
* **Multi-format builds:** a build option includes texture data in several
  formats in one build. At startup the loader detects the formats the device
  supports and downloads only the data for the best one, in the order the
  project sets. The default order is ASTC, then DXT, then ETC2.
* **Runtime selection:** a texture whose format the device cannot sample is
  decompressed at load to an uncompressed format that holds every channel. The
  player reports the texture, the format, and the extra memory.
* **Supercompressed textures:** Crunch-compressed textures, and KTX2 textures
  with Basis Universal supercompression from a package that CMP-002 lists,
  transcode at load to the best format the device offers.
* **Reporting:** the build result lists each texture's format per included
  format set. At runtime the player reports the format set selected, each
  transcoded or decompressed texture, and the memory it uses.

## WebAssembly player

Managed code on the Web target runs with ahead-of-time compilation (BLD-003).
Every managed-code guarantee in
[Scripting backends, ahead-of-time compilation, and code stripping](build-platforms-xr-networking.md#scripting-backends-ahead-of-time-compilation-and-code-stripping)
holds in the browser.

### Code size and stripping

* The managed stripping level, `link.xml`, and the preservation attributes
  work as defined in
  [Scripting backends, ahead-of-time compilation, and code stripping](build-platforms-xr-networking.md#scripting-backends-ahead-of-time-compilation-and-code-stripping).
  Strip Engine Code removes whole engine modules the project does not use and
  never changes a module it keeps, and the build result lists the modules
  kept and the reason each one is kept.
* The Build Settings Code Optimization choices (Shorter Build Time, Runtime
  Speed, Runtime Speed with LTO, Disk Size, and Disk Size with LTO) and the
  IL2CPP Code Generation setting are honored. They apply to the project's
  managed code and to how it is linked with the engine code; the engine code
  itself is the same for every project. The size report states the
  effect of the choice used on code size, measured against the build's
  previous Code Optimization choice when one exists.
* The browser compiles the module while it downloads, and instantiates it as
  soon as compilation completes. Data download continues in parallel.

### Memory

* The Initial Memory Size and Maximum Memory Size settings, and Memory Growth
  Mode (None, Linear, and Geometric) with its linear step, geometric step, and
  geometric cap, are honored.
* With growth mode None, the heap stays at its initial size.
* Heaps above 4 GB are a unitas extension, available in browsers that offer
  64-bit WebAssembly memory. Elsewhere, a Maximum Memory Size above the
  browser's limit is reported at build time and at startup, naming the limit.
* The current heap size, the managed heap, and the heap high-water mark are
  reported in development diagnostics and through the profiler (PRF-001).

### Out of memory

When an allocation cannot be satisfied, the player reports out of memory to the
page and to diagnostics. The report states the requested size, the current
heap size, the configured maximum, the browser's limit, and the high-water
mark. Development builds add the largest managed and native allocations. The
page stays responsive and shows the report through the template. A managed
allocation failure raises `OutOfMemoryException` in managed code first, as on
desktop, so a game that catches it can recover.

### Threads

Managed threads in the browser, the behavior of a page that is not
cross-origin isolated, and the setting that keeps Unity's browser threading
behavior are defined by RUN-004 in
[Scripting runtime](scripting-runtime.md#threading-and-asyncawait). The browser
adds these specifics:

* **Workers:** on a cross-origin isolated page, the job system, the thread
  pool, and explicitly created threads run on Web Workers that share the
  WebAssembly memory. Native plug-in code and engine work use them as well.
  The startup report states the number of workers.
* **Jobs without isolation:** jobs run inline on the page thread with the same
  results, and the lack of parallelism is reported as a performance property.

The headers that isolation requires are listed in
[Hosting and server headers](#hosting-and-server-headers).

### SIMD

WebAssembly SIMD is a unitas extension, on by default. Managed code, engine
code, and Burst-compiled code (RUN-011 in
[Scripting runtime](scripting-runtime.md)) use SIMD instructions where the
browser offers them. When a browser in the release's declared support matrix
lacks SIMD, the build includes a module without SIMD and the loader selects
it. The startup report states which module runs. A build setting turns SIMD
off.

### Exceptions

The Enable Exceptions setting (None, Explicitly Thrown Exceptions Only, Full
Without Stacktrace, and Full With Stacktrace) selects diagnostic detail, not
correctness.

* Under every value, managed exceptions are thrown and caught as on desktop.
  This includes the null-reference, array-bounds, cast, and arithmetic
  exceptions that the runtime detects.
* Full With Stacktrace fills `Exception.StackTrace` with managed frames in
  release builds. The other values leave it empty in release builds.
  Development builds always include managed frames.
* The size report states the code size of the value used.
* WebAssembly Arithmetic Exceptions keeps its Unity meaning. Throw, the
  default, raises `DivideByZeroException` and `OverflowException` as on
  desktop. Ignore produces the value the WebAssembly operation returns, and
  the build result reports Ignore as a difference from desktop behavior.

### Debug symbols and stack traces

* The Debug Symbols setting (Off, External, and Embedded) is honored. External
  writes a separate symbols file that the page does not download.
* Every build produces source maps and symbol files matched to its build
  identifier (DIA-003 in
  [Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md)).
* In development builds, and in release builds with Embedded symbols, browser
  console stack traces show managed type, method, file, and line.
* For release builds with External or Off, a stack trace that a crash report
  or the console captured is symbolicated through the editor, the command line,
  and the editor's MCP interface.

## Build output and templates

A Web build writes a folder that holds the page (`index.html`), a `Build` folder
with the loader, framework, code, and data files, `StreamingAssets`, the
template's `TemplateData`, the third-party notices (LIC-001), and a header
manifest. The header manifest lists every file with the response headers it
requires. The build also writes server configuration for Apache, nginx, and IIS
that applies those headers.

WebGL templates:

* **Built-in templates:** the Default template shows the product name, a
  loading bar, a fullscreen button, and the warning banner, with no Unity
  branding. The Minimal template shows only the canvas. The Progressive Web App
  template adds a web app manifest and offline support.
* **Project templates:** every folder in `Assets/WebGLTemplates` with an
  `index.html` is a selectable template. Its `thumbnail.png` appears in the
  template picker, and its other files are copied to the build.
* **Selection:** `PlayerSettings.WebGL.template` values in the
  `APPLICATION:<name>` and `PROJECT:<name>` forms select the template, as in
  Unity.
* **Template variables:** the built-in variables (`COMPANY_NAME`,
  `PRODUCT_NAME`, `PRODUCT_VERSION`, `WIDTH`, `HEIGHT`, `BACKGROUND_COLOR`,
  `SPLASH_SCREEN_STYLE`, `UNITY_VERSION`, `DEVELOPMENT_PLAYER`,
  `DECOMPRESSION_FALLBACK`, `LOADER_FILENAME`, `FRAMEWORK_FILENAME`,
  `CODE_FILENAME`, `DATA_FILENAME`, `MEMORY_FILENAME`, `SYMBOLS_FILENAME`, and
  `BACKGROUND_FILENAME`) expand in `{{{ }}}` tags, and JavaScript expressions in
  tags and `#if`, `#else`, and `#endif` directives evaluate as in Unity.
  `UNITY_VERSION` expands to the value `Application.unityVersion` returns.
* **Custom variables:** any other variable a template uses appears as a custom
  template variable in Player Settings, and its value is stored with the
  project's Player Settings.
* **Template errors:** a template with an unknown directive, an unbalanced
  `#if`, or an expression that fails to evaluate is reported in the build
  result, naming the file, the line, and the cause. It is never silently
  ignored.

## Loader and page API

The page starts the player with `createUnityInstance(canvas, config,
onProgress)`, as in Unity.

* **Config:** `dataUrl`, `frameworkUrl`, `codeUrl`, `memoryUrl`, `symbolsUrl`,
  `streamingAssetsUrl`, `companyName`, `productName`, `productVersion`,
  `devicePixelRatio`, `matchWebGLToCanvasSize`, `webglContextAttributes`,
  `cacheControl`, and `showBanner` keep their Unity meaning. An unknown config
  key is reported in the console in development builds.
* **Progress:** `onProgress` receives values from 0 to 1 that never decrease.
  It reaches 1 when the first scene is ready.
* **Result:** the returned promise resolves to the instance once the player has
  started. It rejects with a message that names the cause on any startup
  failure, including an unsupported browser, a missing graphics API, a file
  that failed to download or decompress, and out of memory.
* **Instance:** `SetFullscreen(1)` and `SetFullscreen(0)` enter and leave
  fullscreen, subject to the gesture rule below. `SendMessage(objectName,
  methodName, value)` calls a method on a named GameObject, as in Unity.
  `Quit()` returns a promise that resolves once the player has stopped and
  released its memory. After `Quit()`, the same canvas can host a new
  instance.
* **Warnings:** `showBanner` receives warnings and errors with their severity.
  The Default template displays them.

## Compression, caching, and file names

* **Compression Format:** Brotli, Gzip, and Disabled are honored. Brotli
  produces the smallest transfer.
* **Decompression Fallback:** with the fallback on, the loader decompresses
  files that arrive without a matching `Content-Encoding` header. Development
  builds warn that the header is missing and state the startup cost. With the
  fallback off, a compressed file that arrives without the header fails
  startup with an error that names the file and the header the server must
  send.
* **Brotli over plain HTTP:** browsers accept Brotli content encoding only on
  secure origins. A Brotli build served over plain HTTP without the fallback is
  reported with that cause.
* **Data Caching:** with Data Caching on, downloaded build files are stored in
  the browser's IndexedDB cache, and later visits load them from the cache.
  `cacheControl` decides per URL whether a file is revalidated, treated as
  immutable, or not cached. A cache the browser has evicted or that is
  corrupt is detected and downloaded again.
* **Name Files As Hashes:** file names become digests of their content, so a
  host can mark them immutable and a new build never collides with a cached
  file from an earlier build.

## Remote content and Addressables

Addressables content and AssetBundles for the Web target build and load as
defined in
[Content builds and post-release content updates](build-platforms-xr-networking.md#content-builds-and-post-release-content-updates)
and RUN-006.

* Remote load paths that point at a CDN load catalogs and bundles from it. The
  `Caching` API stores downloaded bundles in IndexedDB.
* A bundle request that the browser blocks for cross-origin reasons is
  reported, naming the URL, the CDN's missing response header, and the
  cross-origin rule.
* On a cross-origin isolated page, content from another origin must carry a
  cross-origin resource policy or CORS headers. A missing header is reported
  in the same way.
* Download size queries, dependency downloads, and catalog update checks work
  as on other targets.

## Hosting and server headers

The header manifest states every header the build needs. Development builds
check the headers they receive at startup and report each missing or wrong
header, naming the file and the expected value.

| Header | Files | Value |
|---|---|---|
| `Content-Encoding` | Brotli files (`.br`) | `br` |
| `Content-Encoding` | Gzip files (`.gz`) | `gzip` |
| `Content-Type` | `.wasm` | `application/wasm` |
| `Content-Type` | `.js` | `application/javascript` |
| `Content-Type` | `.data` and bundles | `application/octet-stream` |
| `Content-Type` | `.symbols.json` | `application/json` |
| `Cross-Origin-Opener-Policy` | the page | `same-origin` |
| `Cross-Origin-Embedder-Policy` | the page | `require-corp` or `credentialless` |
| `Cross-Origin-Resource-Policy` | files from another origin | `cross-origin` |

* With Decompression Fallback on, compressed files use the `.unityweb`
  extension. Serving them with the `Content-Encoding` of the build's
  compression format skips decompression in the loader.
* Without `application/wasm`, the browser cannot compile while downloading.
  The player still starts, and development builds report the slower startup
  and name the header.
* Without the two cross-origin isolation headers on the page, the player runs
  without browser threads, as described in [Threads](#threads).

### Local development server

The editor serves Web builds from a local development server that sends every
header in the manifest, including the cross-origin isolation headers and
`Content-Encoding`. Build And Run uses it.

* The server picks a free port and reports the URL.
* Serving to other devices on the local network, for testing on phones and
  tablets, is opt-in. It serves over HTTPS with a development certificate,
  because WebGPU, browser threads, and the clipboard require a secure context
  on any host other than the local machine. Trusting that certificate on a
  device is the person's decision.
* The server runs in the headless editor as well. Agents start, inspect, and
  stop it through the editor's MCP interface under the deploy scope (AUT-007).

### Embedding and game portals

* **Iframes:** a build embedded in an iframe works when the embedding page
  grants the permissions the game uses: fullscreen, autoplay, gamepad,
  clipboard, pointer lock, and cross-origin isolation. The player reports each
  permission the embedding page's policy denies. Keyboard input reaches the
  player once the person clicks the frame.
* **Portals:** a build option produces output for portals that host the game
  on their own domain, accept a single archive, and do not let the developer
  set response headers. That output uses relative paths only, uses the
  decompression fallback, and needs no server configuration. The build result
  states which features depend on headers the portal may not send, such as
  browser threads.
* **Portal SDKs:** a portal's JavaScript SDK is reached through JavaScript
  interop.

## JavaScript interop

* **Plug-ins:** `.jslib` files in the project and its packages are included in
  Web builds as RUN-007 in [Scripting runtime](scripting-runtime.md) defines.
  Their `mergeInto(LibraryManager.library, ...)` functions, `$`-prefixed
  dependencies, `__deps` entries, and `autoAddDeps` resolve as in Unity.
  `.jspre` files run before the player starts, in a stable order that the
  build result lists.
* **Calling JavaScript from C#:** `[DllImport("__Internal")]` declarations call
  jslib functions with `int`, `float`, `double`, `bool`, string, and array
  arguments, and return numbers, booleans, and strings.
* **Calling C# from JavaScript:** `SendMessage` from a jslib or the page calls
  a method with no argument, a string, or a number. Managed callbacks passed as
  function pointers and marked `[MonoPInvokeCallback]` are called from
  JavaScript through the `makeDynCall` and `dynCall` forms that Unity 2022.3
  LTS documents.
* **Marshalling:** strings convert with `UTF8ToString`, `stringToUTF8`, and
  `lengthBytesUTF8`. A string returned to C# is allocated with `_malloc`.
  Arrays pass as a pointer and a length and are read through the `HEAP8`,
  `HEAPU8`, `HEAP16`, `HEAPU16`, `HEAP32`, `HEAPU32`, `HEAPF32`, and `HEAPF64`
  views. These views stay valid after the heap grows and when memory is shared
  between threads.
* **Module:** `Module` is reachable from jslib code and, as the instance's
  `Module`, from the page.
* **Threads:** a jslib function called from a managed background thread runs on
  the page's main thread, where the page's document is reachable, and the
  calling thread waits for its result.

Errors in each direction:

* A JavaScript exception thrown in a jslib function raises an exception in the
  calling C# code. The exception names the jslib function and carries the
  JavaScript message and stack. The player keeps running.
* A `DllImport` that names no included jslib function fails the build, naming
  the declaration and the missing function.
* `SendMessage` to an object or method that does not exist logs an error that
  names both, as in Unity.
* An exception thrown by the C# method that `SendMessage` called is logged with
  its managed stack and does not stop the player.
* An exception thrown by a managed callback that JavaScript called reaches the
  JavaScript caller as an error that names the managed exception type and
  message.

## Canvas, display, and fullscreen

* **Canvas size:** the player renders at the canvas's CSS size multiplied by
  the device pixel ratio. `config.devicePixelRatio` overrides the ratio, and a
  project setting caps it, for performance on high-density screens.
  `Screen.width`, `Screen.height`, and `Screen.dpi` report the result.
* **Resizing:** resizing the canvas or the window changes `Screen.width` and
  `Screen.height` from the next frame. With `matchWebGLToCanvasSize` off,
  `Screen.SetResolution` sets the render resolution and the browser scales it
  to the canvas.
* **Fullscreen:** `Screen.fullScreen`, `Screen.fullScreenMode`, and
  `SetFullscreen` request fullscreen. Browsers grant fullscreen only during a
  user gesture, so a request made from game code applies at the next click,
  touch, or key press on the canvas, as in Unity. Until then, the request is
  reported as pending. Leaving fullscreen with Escape or a browser control
  updates `Screen.fullScreen`. A browser that offers no element fullscreen,
  such as Safari on iPhone, reports fullscreen as Platform-limited.
* **Pointer lock:** `Cursor.lockState` follows the same gesture rule.
  `WebGLInput.stickyCursorLock` keeps its Unity meaning. A lock request that
  the browser refuses, for example right after the person pressed Escape, is
  reported and retried at the next gesture.
* **Cursor:** `Cursor.SetCursor` and `Cursor.visible` apply to the canvas.

## Keyboard and focus

* `WebGLInput.captureAllKeyboardInput` keeps its Unity meaning. When it is
  true, the player receives keyboard input from the whole page. When it is
  false, the player receives keys only while the canvas has focus, so HTML
  elements on the page receive their own input.
* Clicking the canvas gives it focus. Focus loss delivers
  `OnApplicationFocus(false)`, and keys held at that moment are released.
* Browsers reserve some shortcuts, such as closing a tab. These never reach the
  player, and they are listed in product knowledge (AUT-006). In fullscreen, in
  browsers that offer keyboard lock, a project setting captures Escape and
  other system keys.

## Gamepads, touch, sensors, and mobile browsers

* **Gamepads:** gamepads come from the browser's Gamepad API for the legacy
  Input Manager (INP-001) and the Input System (INP-002), with device
  recognition as UI, audio, and input defines. A gamepad appears once the
  person presses one of its buttons, as browsers require. Rumble works where
  the browser offers vibration for that gamepad. Light bars and other HID
  outputs work through the browser's HID access where the browser offers it
  and the person grants it. Elsewhere they are reported as Platform-limited.
* **Touch:** multi-touch reaches `Input.touches` and the Input System
  `Touchscreen`, including touch cancellation when the browser takes over a
  gesture. Templates stop page scrolling and zooming on the canvas.
  `Input.mousePresent` and `Input.touchSupported` reflect the device.
* **Sensors:** accelerometer, gyroscope, and attitude data come from the
  browser's motion and orientation events. On browsers that require
  permission, such as Safari on iOS, the first sensor use asks for it at the
  next gesture, and a denial returns an explicit status.
* **Orientation and safe area:** `Screen.orientation` reports the current
  orientation. An orientation lock applies where the browser allows it, such
  as fullscreen in Chrome on Android, and is reported as Platform-limited
  elsewhere. `Screen.safeArea` reflects the browser's safe-area insets.
* **Vibration:** `Handheld.Vibrate` works where the browser offers vibration,
  and returns an explicit status elsewhere.
* **Mobile browsers:** Safari on iOS and iPadOS and Chrome on Android are
  supported browsers. On these devices the player applies the mobile memory
  limit the browser enforces. It reports the limit at startup and keeps the
  out-of-memory report above. Browsers on iOS and iPadOS that use Safari's
  engine behave as Safari.

## Text input and clipboard

* **Virtual keyboard:** on touch devices, focusing a uGUI, TextMeshPro, UI
  Toolkit, or IMGUI text field opens the browser's on-screen keyboard. The
  keyboard follows the field's content type, multiline setting, and secure
  setting where the browser allows. Text, selection, composition, submission,
  and cancellation flow back to the field. `TouchScreenKeyboard` behaves as UI,
  audio, and input defines, and `WebGLInput.mobileKeyboardSupport` keeps its
  Unity meaning.
* **IME:** composition works on desktop browsers as UI, audio, and input
  defines.
* **Clipboard:** clipboard access behaves as UI-001 in
  [UI, audio, and input](ui-audio-input.md) defines for the browser target.
  Writing happens within the gesture that triggered the copy, where the
  browser requires one.

## Visibility and background tabs

* Hiding the page, by switching tabs or minimizing, delivers
  `OnApplicationFocus(false)` and, with Run In Background off,
  `OnApplicationPause(true)`. Showing it again delivers the reverse.
* With Run In Background on, the player keeps running in a hidden tab at the
  rate the browser allows. Browsers throttle hidden tabs and may stop their
  frames entirely. The player reports the throttling.
* Returning from a throttled or stopped state never produces a burst of
  catch-up frames. `Time.deltaTime` is bounded by `Time.maximumDeltaTime`,
  fixed updates are bounded as Unity defines, and game time continues from
  where it stopped without a jump. `Time.realtimeSinceStartup` reports real
  elapsed time.
* Audio follows the pause state. Network connections stay open while the
  browser allows.

## Audio and video on the web

Audio plays through the browser's Web Audio output. Every AUD-001 behavior in
[UI, audio, and input](ui-audio-input.md) applies: clip load types
(Decompress On Load, Compressed In Memory, and Streaming), mixer groups,
snapshots and effects, filters, `OnAudioFilterRead`, spatial audio, and
`GetSpectrumData` and `GetOutputData`. These are unitas extensions where Unity
2022.3 LTS's WebGL player lacks them.

* **Autoplay unlock:** browsers start audio only after a user gesture. Until
  the first click, touch, or key press on the page, audio is reported as not
  permitted, and sources that play are tracked as playing silently. The first
  gesture enables audio, and sources continue from their current time. On
  Safari for iOS, the unlock uses the gesture that the browser accepts for
  audio, including a touch ending.
* **Suspension:** a context that the browser suspends later resumes after the
  next gesture. The game can observe whether audio is currently permitted.
* **Device mute:** the iOS ring/silent switch affects browser audio as the
  operating system decides. The player reports that state where the browser
  exposes it.
* **Compressed audio:** every encoding AUD-001 lists decodes in every supported
  browser, including encodings the browser's own decoder lacks.
* **Latency:** the output latency the browser reports is shown in audio
  inspection and diagnostics. Where the browser reports none, the latency is
  reported as unknown.
* **Microphone:** microphone capture behaves as UI, audio, and input defines
  for the browser target.

Video follows MED-001.

* `VideoPlayer` plays `VideoClip` assets and URL sources on the Web target.
  Unity 2022.3 LTS's WebGL player plays only URLs, so `VideoClip` playback is
  a unitas extension.
* Every codec and container MED-001 lists plays in every supported browser,
  including codecs the browser's own decoder lacks. The decoding path in use
  is reported as a performance property.
* A cross-origin video URL whose server sends no CORS headers cannot be read
  into a texture in a browser. It is reported as Platform-limited, naming the
  URL and the missing header.
* Video with sound starts under the same autoplay rule as audio.

## Storage

* **PlayerPrefs:** `PlayerPrefs` is stored in the page origin's IndexedDB
  storage, with no fixed size limit, as SAV-001 defines. Reads and writes in
  game code are synchronous, as in Unity. `PlayerPrefs.Save` returns once the
  write is queued. Durability is observable: the preferences are reported as
  pending until the write completes, as for files.
* **Files:** `Application.persistentDataPath` returns the path form Unity
  2022.3 LTS returns for WebGL, scoped to the page's origin. Files under it use
  `System.IO` synchronously. A closed file is copied to IndexedDB
  automatically and reported as pending until the copy completes, as SAV-001
  defines.
* **Quota:** the player reports the quota and usage the browser grants. A
  write that exceeds the quota fails in the calling code with the .NET
  exception for a full disk, naming the path, never with apparent success.
* **Persistence:** a project setting asks the browser to make storage
  persistent. Whether the browser granted that is observable. Storage that the
  browser may evict is reported as best-effort.
* **Private browsing:** where private browsing makes storage session-only or
  smaller, the player reports storage as session-only at startup, and writes
  behave as above within that storage.
* **Origin scope:** builds served from different origins never share
  storage.

## Networking on the web

* **Web requests:** `UnityWebRequest` runs over the browser's fetch and
  `XMLHttpRequest` APIs, with the RUN-009 behaviors in
  [Scripting runtime](scripting-runtime.md). Upload and download progress,
  streaming download handlers, timeouts, and `Abort` work. A request blocked
  by CORS returns `ConnectionError`, and diagnostics name the URL and the
  missing response header. An HTTP request from an HTTPS page is reported as
  mixed content. Forbidden headers are reported as RUN-009 defines.
* **WebSocket:** `ClientWebSocket` and the Unity Transport WebSocket interface
  connect to WebSocket servers, as RUN-009 and
  [Unity Transport](build-platforms-xr-networking.md#unity-transport) define. A `ws://` URL from an HTTPS page
  is reported as mixed content.
* **WebRTC:** a game that opens WebRTC data channels, through JavaScript
  interop or a package that CMP-002 lists, receives their messages in managed
  code in order. Connection state changes are reported.
* **No raw sockets:** browsers give pages no raw TCP or UDP access. This is a
  real browser absence, and socket use is reported as RUN-009 defines.

## Browser support matrix

Each release declares, under CMP-005 in the
[Compatibility contract](compatibility-contract.md), the minimum browser
versions for the Web target. The matrix covers at least:

| Browser | Platforms | Versions |
|---|---|---|
| Chrome | Windows, macOS, Linux, ChromeOS, Android | current stable and previous major |
| Edge | Windows, macOS | current stable and previous major |
| Firefox | Windows, macOS, Linux, Android | current stable, previous major, and current Extended Support Release |
| Safari | macOS, iOS, iPadOS | current major and previous major |

For each browser and platform, the matrix states WebGPU, browser threads,
SIMD, 64-bit memory, compressed texture formats, audio, gamepad, fullscreen,
pointer lock, clipboard, and storage persistence.

On a browser below the declared minimum, the loader does not start the player.
The page receives an error that names the browser, its version, and the
minimum, and the Default template shows it. A project setting lets the player
start anyway, with a warning banner. A browser that meets the minimum but lacks
an optional capability, such as WebGPU or threads, starts with the fallback
defined above and is not treated as unsupported.

## Web performance and optimization

The outcomes below are measured on the reference projects and reference
hardware that each release records under PRF-002 in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md#declared-budgets),
and the measurements are published with each release.

### Rendering

Under WebGL 2, the outcomes below hold on the same render graph as WebGPU.

* **Batching and instancing:** on the reference projects, draw-call and batch
  counts under WebGL 2 and WebGPU equal those of the same project's Android
  OpenGL ES 3 player.
* **Shader warm-up:** shader programs and pipelines that the build's shader
  variant collections and its first scene need are compiled at load, and the
  startup report states the warm-up time. `ShaderVariantCollection.WarmUp` and
  `Shader.WarmupAllShaders` are honored.
* **Compilation hitches:** on the reference projects, no frame during gameplay
  stalls longer than 4 ms for shader or pipeline compilation. Development
  builds report each variant that was first compiled during gameplay, naming
  the shader, pass, and keywords.
* **Texture uploads:** no texture upload stalls a frame for more than 2 ms.
* **Supercompressed textures:** these transcode to the best format the device
  offers, as described in [Texture compression](#texture-compression).
* **Compute-dependent features:** these run on both APIs, as described in
  [Render pipelines and features on the web](#render-pipelines-and-features-on-the-web).

### WebAssembly

* **Code Optimization:** each choice has a measured effect on code size and
  frame time, stated in the size report.
* **Streaming:** the module compiles and instantiates while data downloads.
* **SIMD:** used where the browser offers it.
* **Garbage collection:** with incremental garbage collection, which is on by
  default and honors `GarbageCollector.GCMode`, main-thread collection pauses
  are 2 ms or less at the 99th percentile on the reference projects.
* **Memory:** during steady-state play after load, the heap does not grow. In a
  replayed session (TST-002) that loops the same gameplay, the heap size after
  the last loop equals the size after the first loop.
* **High-water mark:** the heap high-water mark is reported in development
  diagnostics, in the profiler, and in out-of-memory reports.

### Startup and size

* **Progressive loading:** a build option splits build data by scene and by
  Addressables group. The first interactive frame waits only for the content
  the first scene needs. Later scenes download in build order after the first
  frame. A load of a scene that has not arrived waits for its content, and
  `LoadSceneAsync` progress includes that download. The wait is reported in
  development diagnostics.
* **Startup report:** the startup report states the time spent in download,
  compilation, instantiation, engine start, first scene load, and shader
  warm-up, and the time to the first interactive frame.
* **Size report:** every Web build produces a download-size report. It breaks
  the transfer into code (the WebAssembly module and framework), data
  (scenes, assets, and resources), streaming assets, and Addressables
  content. It lists sizes before and after compression, per assembly, per
  engine module, and per asset, with the largest contributors first. It also
  lists the change from the previous build of the same target.

### Engine web budgets

These are engine budgets, declared and measured alongside the PRF-002 budgets
in
[Diagnostics, profiling, and playtesting](diagnostics-profiling-playtesting.md#declared-budgets)
under PRF-002's measurement and regression rules. The reference desktop, the
reference integrated-GPU laptop, and the reference mid-range phone are the
reference hardware each release records there. A measurement beyond a budget
is a visible regression.

* The [Roll-a-Ball-style sample](sample-games/01-roll-a-ball.md)'s compressed
  transfer is no more than 15 MB.
* Its first interactive frame arrives within 3 s on a cold cache over a
  50 Mbps connection, and within 1 s on a warm cache, on the reference desktop.
* It runs at 60 fps at 1080p on the reference integrated-GPU laptop in Chrome,
  Edge, Firefox, and Safari.
* It runs at 60 fps on the reference mid-range phone in Chrome on Android and
  Safari on iOS.
* The [2D platformer sample](sample-games/02-2d-platformer.md)'s heap stays
  under 256 MB.

### Project budgets

A project declares its own web budgets: compressed transfer size, initial
download size, heap size, and time to first interactive frame. Each build
result and each headless browser test run reports every project budget as met
or exceeded. A project setting makes an exceeded budget fail the build.
Project budgets are separate from the engine budgets of PRF-002.

### Frame pacing

* Frames are driven by the browser's animation frame callback. With
  `Application.targetFrameRate` at -1, the player presents on every display
  refresh, including on high-refresh displays. A positive value paces frames
  below the refresh rate, as REN-007 in
  [Rendering and animation](rendering-and-animation.md) defines.
* Background-tab throttling is handled as described in
  [Visibility and background tabs](#visibility-and-background-tabs).
* The device pixel ratio cap and resolution scale settings (URP render scale
  and dynamic resolution) are available for performance, and their effect is
  shown in the profiler.

### Web build profile

The editor has a web build profile view. For a Web build, it reports size per
assembly, engine module, and asset, flags the largest contributors, and
recommends stripping, Code Optimization, compression, and texture format
settings with their estimated savings. Agents use it as an editor command
through the editor's MCP interface (AUT-002). The browser player itself
carries no such interface.

## Web diagnostics

* **Console forwarding:** `Debug.Log`, `Debug.LogWarning`, `Debug.LogError`,
  and exceptions reach the browser console with matching severity. Browser
  console messages, including graphics API errors and page script errors,
  reach the editor's device log stream (DIA-003) when the editor is connected.
* **Development builds:** Autoconnect Profiler, Deep Profiling Support, and
  Show Diagnostics Overlay are honored. A development browser player connects
  to the editor's profiler (PRF-001) and to script debugging (DIA-002) with
  breakpoints and stepping, under the pairing rules of
  [Deployment, device connection, and development players](build-platforms-xr-networking.md#deployment-device-connection-and-development-players).
  Script debugging is a unitas extension.
* **Crash and exception reports:** unhandled managed exceptions, WebAssembly
  traps, out-of-memory failures, and unrecoverable graphics errors produce
  DIA-003 reports. Each report carries the build identifier, the browser and
  its version, the operating system, the graphics API and adapter, and mapped
  stacks. Reports go only to a destination the project configures, and the
  page's error path receives each one.

## Automated browser testing

The headless editor and the command line (BLD-002) run project tests (TST-003)
and structured scenarios (TST-002) in a Web build inside browsers that the
editor launches and controls with no display. The Unity test-platform argument
for WebGL selects this mode.

* The invocation selects one or more installed browsers (Chromium-based,
  Firefox, and WebKit-based) and whether the page is cross-origin isolated.
* The editor serves the build with its development server and collects test
  results, logs, console output, crash reports, startup reports, and project
  budget results into the same machine-readable output and `-testResults` file
  as other test runs.
* Each result records the browser, its version, the graphics API, and whether
  rendering ran on a hardware GPU or a software GPU adapter.
* A requested browser that is not installed is reported as a missing toolchain
  with its distinct exit status.

## Acceptance outcomes

1. A Web build with Auto Graphics API on runs on WebGPU in a browser that
    offers it. In a browser without WebGPU, and in one whose adapter lacks a
    required feature, the same build runs on WebGL 2, and the startup report
    names the API tried, the reason it was skipped, and the API selected. A
    build whose Graphics APIs list holds only WebGL 2 never uses WebGPU.
    (PLT-001, BLD-001)
2. A Web build served with the cross-origin isolation headers runs a thread
    that blocks on its result and an `IJobParallelFor` on workers with the
    same results as a desktop build. Served without the headers, it starts,
    reports that browser threading is unavailable and names the missing
    headers, and runs the jobs on the main thread with the same results.
    (PLT-001, RUN-004, RUN-011)
3. A Brotli build with Decompression Fallback off starts when served with
    `Content-Encoding: br`, and fails without the header with an error naming
    the file and the header. The same project with the fallback on starts
    without the header, and its development build warns about the missing
    header. A build served without `application/wasm` starts, and its
    development build names that header. (PLT-001, BLD-001)
4. A jslib function called through `[DllImport("__Internal")]` receives a
    string and a float array, returns a string that C# reads intact, and calls
    a `[MonoPInvokeCallback]` method with a number. A `SendMessage` from the
    page reaches a named GameObject. A JavaScript exception in the jslib
    function raises a C# exception that names the function and carries the
    JavaScript message, and the player keeps running. (PLT-001, RUN-007)
5. A browser game saves with `PlayerPrefs.Save` and writes a file under
    `Application.persistentDataPath`. After both report synchronized, the
    page is reloaded, and both values load in the new session. A write beyond
    the storage quota fails in the calling code. In private browsing, storage
    is reported as session-only at startup. (PLT-001, SAV-001)
6. On Safari for iOS, a game that starts music before any touch reports
    audio as not permitted. The first touch enables audio, the music
    continues from its current time, and a context that the browser suspends
    later resumes after the next touch. (PLT-001, AUD-001)
7. When the WebGL 2 context is lost and restored during play, game logic keeps
    running, rendering resumes on the next frame with every texture and mesh
    intact, and render textures report `IsCreated` false until used again.
    When the browser does not restore the context, the page receives an
    unrecoverable graphics error naming the cause, and the page stays
    responsive. The same holds for a lost WebGPU device. (PLT-001, REN-006)
8. A project template in `Assets/WebGLTemplates` uses `PRODUCT_NAME`,
    `PRODUCT_VERSION`, `DATA_FILENAME`, a `#if DECOMPRESSION_FALLBACK` block,
    and a custom variable. The built page contains the expanded values, the
    custom variable appears in Player Settings with its stored value, and a
    template with an unbalanced `#if` is reported by file and line.
    (PLT-001, BLD-001)
9. A Web build's size report breaks the transfer into code, data, streaming
    assets, and Addressables content, before and after compression, per
    assembly, engine module, and asset, with the largest contributors first.
    Changing Code Optimization from Runtime Speed to Disk Size with LTO shows
    the measured code-size change. A project budget the build exceeds is
    reported, and fails the build when the project setting asks for that.
    (PLT-001, BLD-001, BLD-003)
10. A command-line invocation with no display runs a project's play-mode
    tests and a structured scenario in a Web build in a Chromium-based
    browser and in Firefox, both controlled by the editor. Results, console
    output, and startup reports for each browser land in the `-testResults`
    file and the machine-readable output. A requested browser that is not
    installed exits with the missing-toolchain status. (BLD-002, TST-002,
    TST-003)
11. A multi-format Web build with ASTC, DXT, and ETC2 data downloads only
    ASTC data on a phone that offers ASTC and only DXT data on a desktop that
    offers DXT. A texture in a format the device cannot sample is decompressed
    at load and reported with its extra memory. (PLT-001, REN-002)
12. A browser game that allocates beyond its Maximum Memory Size receives
    `OutOfMemoryException`, and when it does not catch it, the page shows an
    out-of-memory report with the requested size, the heap size, the
    configured maximum, and the high-water mark, and stays responsive.
    (PLT-001, DIA-003)
13. In a Web build embedded in an iframe, a fullscreen request and a cursor
    lock request made from game code apply at the next click on the canvas. In
    an iframe whose policy denies fullscreen, the player reports the denied
    permission. Escape leaves fullscreen and releases the cursor, and
    `Screen.fullScreen` and `Cursor.lockState` report it. (PLT-001)
14. On the reference projects under WebGL 2, the startup report states the
    shader warm-up time, no gameplay frame stalls longer than 4 ms for shader
    or pipeline compilation or longer than 2 ms for a texture upload, and the
    draw-call counts equal those of the Android OpenGL ES 3 player.
    (PLT-001, PRF-002)
15. A 30-minute replayed session of the
    [2D platformer](sample-games/02-2d-platformer.md) in a browser shows
    main-thread garbage-collection pauses of 2 ms or less at the 99th
    percentile, the same heap size after the last loop as after the first,
    a heap that stays under 256 MB, and a reported heap high-water mark.
    (PLT-001, PRF-002, TST-002)
16. The [Roll-a-Ball-style sample](sample-games/01-roll-a-ball.md) built for the Web target transfers no more
    than 15 MB compressed, shows its first interactive frame within 3 s on a
    cold cache over a 50 Mbps connection and within 1 s on a warm cache on the
    reference desktop, runs at 60 fps at 1080p on the reference
    integrated-GPU laptop in Chrome, Edge, Firefox, and Safari, and runs at
    60 fps on the reference mid-range phone in Chrome on Android and Safari on
    iOS. A measurement beyond any of these budgets is published as a
    regression. (PLT-001, PRF-002)
17. A URP project with a Visual Effect Graph effect, GPU culling, and a
    project `ComputeShader` renders on WebGL 2 in a browser without WebGPU.
    Each feature's results match WebGPU within the capacities the build
    reports, and each capacity lower than on WebGPU is reported by feature and
    asset. (PLT-001, REN-004, REN-006)
18. A Web build split by scene shows its first interactive frame before its
    later scenes have downloaded, and loading a later scene with
    `LoadSceneAsync` reports progress that includes its download. (PLT-001,
    RUN-005, RUN-006)
19. An agent requests the web build profile through the editor's MCP
    interface and receives size per assembly, engine module, and asset, the
    largest contributors, and recommended stripping, compression, and texture
    format settings. The browser player built afterward contains no MCP
    interface. (PLT-001, AUT-002)
20. A browser game running at `targetFrameRate` -1 presents on every refresh
    of a 120 Hz display. After its tab is hidden for one minute and shown
    again, `Time.deltaTime` on the first frame is no larger than
    `Time.maximumDeltaTime`, no burst of catch-up frames occurs, and
    `OnApplicationFocus` and `OnApplicationPause` were delivered.
    (PLT-001, REN-007)
21. A browser game's `UnityWebRequest` to a server that sends no CORS headers
    returns `ConnectionError`, and diagnostics name the URL and the missing
    header. A `ClientWebSocket` connects to a WebSocket server, and a raw
    socket fails with the platform-not-supported exception. Addressables
    content loads from a CDN on a cross-origin isolated page, and a bundle
    without a cross-origin resource policy is reported by URL. (PLT-001,
    RUN-009, BLD-005)
22. The editor's local development server serves a Web build with every
    header in its manifest, and a phone on the local network opens the
    build over HTTPS with browser threads available. A browser below the
    declared minimum version receives an error naming the browser, its
    version, and the minimum. (PLT-001, BLD-004)
23. A development browser player connects to the editor's profiler and
    debugger with its pairing credential. A breakpoint in a C# method stops
    the player and the debugger steps through it. An unhandled exception
    produces a crash report with the build identifier, browser, graphics API,
    and a stack mapped to C# source lines. (PLT-001, BLD-004, DIA-002,
    DIA-003)
