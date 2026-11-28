# Settings and Preferences

## Scope

This document details four windows of the visual editor: Project Settings,
Preferences, Package Manager, and Build Settings. It details the workspace,
command, and settings-page behavior of EDT-006 and the inspector editing
conventions of EDT-002 as they apply to settings, both owned by
[Editor and authoring](../editor-and-authoring.md). This document owns no
catalog ID.

The meaning, storage, and effect of each project setting are owned by PRJ-007,
package resolution by PRJ-006, and project trust and fetch consent by PRJ-008,
all in [Projects and assets](../project-and-assets.md). Build configuration and
results are owned by BLD-001, and toolchain acquisition by PRD-001, in
[Builds, platforms, XR, and networking](../build-platforms-xr-networking.md).
Per-user and per-project unitas configuration is owned by AUT-007 in
[Automation and MCP](../automation-and-mcp.md). This document describes the
windows through which a person and an agent reach them.

## Common settings-window behavior

Project Settings and Preferences share one window layout.

* **Category list.** A tree on the left lists the pages. Built-in pages are
  listed alphabetically; pages that packages and the project declare as
  settings providers (RUN-010) are listed among them by their declared path, and
  a provider path with several segments forms a parent with children.
  Clicking a page shows it; Up and Down move between pages, and Left and
  Right collapse and expand parents.
* **Search field.** A search field above the page searches page names and
  the keywords that each page and provider declares, filters the category list
  to matching pages, and highlights matching property labels on the shown
  page. Clearing the search restores the full list and the page that was
  shown.
* **Page header.** Each page has a title bar with, where the page supports
  them, a presets button (opens the preset selector for that settings object),
  a help button (opens the page's reference documentation), and a menu with
  **Reset** and **Copy** and **Paste** of the page's values.
* **Editing.** Controls follow the [Inspector](inspector.md) conventions:
  property context menus and label drag on numbers. Each page edits one
  settings object, so no mixed-value state arises. Each gesture is one undo
  operation (EDT-005).
* **Persistence.** A Project Settings edit dirties the settings file that
  holds it (under `ProjectSettings/`, or the package's settings asset), and
  `File/Save Project` and `File/Save` write it; closing the project names it
  among the unsaved scope. A Preferences edit is written at once to per-user
  storage and never dirties project content.
* **Pending effect.** A setting whose effect begins at a later boundary (the
  next recompilation, the next play session, the next build, or an editor
  restart) shows that boundary beside it until it takes effect, as PRJ-007
  defines.
* **Errors.** A settings file that cannot be read shows the page with an
  error banner naming the file and cause, and the page's controls read-only;
  unknown keys are preserved, never discarded.
* **Window placement.** Both windows open floating the first time and
  thereafter where the workspace arrangement placed them; each opens at the
  last page shown.

## Project Settings window

The Project Settings window opens from `Edit/Project Settings...`. It shows one
page per settings family. Each page's properties and their effects are those
listed by PRJ-007; this section states the page's arrangement.

* **Adaptive Performance, Burst AOT Settings, Input System Package, Memory
  Settings, Visual Scripting, XR and other package pages** appear when their
  packages declare them.
* **Audio.** Global volume, volume rolloff scale, Doppler factor, default
  speaker mode, system sample rate, DSP buffer size, maximum virtual and real
  voices, spatializer and ambisonic decoder plug-in drop-downs, Disable Audio,
  and Virtualize Effects.
* **Editor.** Sections for the remote input device, Asset Serialization mode,
  Default Behavior Mode (3D or 2D, which sets import and Scene view defaults
  for new content), Sprite Packer mode, C# project generation options, Enter
  Play Mode Options with their Reload Domain and Reload Scene toggles,
  Numbering scheme for duplicated object names (game object naming and
  digits, space before number), Line Endings For New Scripts, Shader
  Compilation, Prefab Mode default environments, Graphics, and Asset Pipeline
  options. Changing the serialization mode states that every asset will be
  rewritten in the new form and lists the count before committing.
* **Graphics.** Defined in
  [Lighting and rendering tools](lighting-and-rendering-tools.md#render-pipeline-graphics-and-quality-pages).
* **Input Manager.** The Axes list with a size field; each axis expands to
  name, descriptive names, negative and positive buttons, alternates,
  gravity, dead, sensitivity, snap, invert, type, axis, and joystick number.
  Context menu items on an axis: **Duplicate Array Element** and **Delete Array
  Element**.
* **Package Manager.** Advanced Settings (Enable Pre-release Packages, Show
  Dependencies) and Scoped Registries (a list with name, URL, and scopes per
  registry, **+** and **−** buttons, and **Save** and **Revert**). A registry
  whose URL cannot be reached is marked with the cause after saving.
* **Physics.** Gravity, default material, bounce threshold, default contact
  offset, solver iteration counts, sleep threshold, queries options, auto sync
  transforms, contact generation and pairs modes, broadphase type, world
  bounds and subdivisions, friction type, the Layer Collision Matrix, and
  cloth settings. The matrix shows layers as rows and columns with a
  checkbox per pair; hovering a checkbox names both layers; **Disable All**
  and **Enable All** set the whole matrix.
* **Physics 2D.** The 2D counterparts, simulation mode, gizmo options for
  drawing colliders, contacts, and AABBs, and the 2D Layer Collision Matrix.
* **Player.** A target tab strip across the top (one tab per installed
  build target group), shared fields above it (company name, product name,
  version, default icon, default cursor and hotspot), and per-target
  sections below it: Icon, Resolution and Presentation, Splash Image, Other
  Settings (rendering, color space, graphics APIs, identification, scripting
  backend, API compatibility level, scripting define symbols with a list
  editor and **Apply**, active input handling, optimization, stripping,
  logging per log type), Publishing Settings, and XR settings where the
  target has them. A per-target override shows beside its default with an
  **Override for** toggle, as PRJ-007 requires. Changing scripting define
  symbols or active input handling states that it recompiles or restarts.
* **Preset Manager.** A list of default presets per type, each with a
  filter and a preset field; drag of a preset asset adds it; order decides
  which matching preset applies first; **Add Default** offers types.
* **Quality.** Defined in
  [Lighting and rendering tools](lighting-and-rendering-tools.md#render-pipeline-graphics-and-quality-pages).
* **Scene Template.** New scene default type, dependency instantiation
  behavior per asset type, and the list of templates shown in the New Scene
  dialog.
* **Script Execution Order.** A list of scripts with their order values and
  a **Default Time** row. **+** offers a searchable script list; drag in the
  list reorders and assigns order values; a value field edits the order;
  **−** removes a script. **Apply** writes each script's `.meta` file, and
  **Revert** discards changes; leaving the page with unapplied changes asks to
  apply or revert.
* **Services.** The project's service settings (project identifier,
  organization, linked service toggles) are shown as stored and are kept
  unchanged as project content; linking and unlinking a cloud project is
  available when the service is reachable and the signed-in account has
  access, and the page states otherwise.
* **Tags and Layers.** Three foldouts: Tags (a reorderable list with **+** and
  **−**; a new tag enters rename), Sorting Layers (a reorderable list whose
  order is the draw order, with Default undeletable), and Layers (the
  builtin layers read-only and user layer slots as text fields). Removing a
  tag or layer that content uses lists the users before committing.
* **TextMesh Pro.** The package's settings page when the package or its
  essentials are present, with **Import TMP Essential Resources** and **Import
  TMP Examples & Extras** buttons when they are absent.
* **Time.** Fixed Timestep, Maximum Allowed Timestep, Time Scale, and Maximum
  Particle Timestep.
* **Timeline.** The package's settings (asset time unit, default framerate)
  when the package is present.
* **UI Builder.** The UI Builder's project settings, defined in
  [UI authoring](ui-authoring.md).
* **Version Control.** The version control mode as stored (PRJ-007), and, with
  the Version Control package, that package's section, provided by unitas's
  implementation of it.
* **XR Plug-in Management.** When the package is present: per-target tabs
  with a checkbox per XR provider plug-in, **Initialize XR on Startup**, and
  child pages per provider. When the package is absent, the page offers to
  install it.
* **unitas.** A trailing page, listed last regardless of order, shows the
  per-project unitas configuration (AUT-007): the project's review policy
  (EDT-008), project secret markings, and crash-report destinations. Its
  controls are person-only; an agent reads the page but cannot change it, and
  a pending change made outside unitas shows a **Confirm** and **Reject** bar
  until a person decides.

### Settings providers

A project or package `SettingsProvider` with project scope appears in the
Project Settings category list at its declared path and draws as an editor
extension; one with user scope appears in Preferences. Its keywords take part in
search. A provider that throws shows the error in place of its page, and the
rest of the window keeps working.

## Preferences window

The Preferences window opens from `Edit/Preferences...` on Windows and Linux,
and from the application menu's `unitas/Settings...` (Cmd+,) on macOS. Its pages
are per-user preferences; none of them changes project content.

* **General.** Load previous project on startup, disable editor analytics,
  show the project wizard at startup, editor theme, editor font, editor
  interaction mode (default, no throttling, monitor refresh rate), script
  changes while playing (recompile and continue playing, recompile after
  finished playing, stop playing and recompile), code optimization on
  startup, default Prefab Mode for opening from the Hierarchy (in context or
  in isolation), and enable play-mode tooltips.
* **2D.** The settings that 2D packages declare, such as Tile Palette grid
  colors and Sprite Editor options ([2D tools](2d-tools.md)).
* **Analysis.** Profiler frame count, show stats for the current frame,
  default recording state, default target mode (Play Mode or Edit Mode), and
  custom connection identifier
  ([Profiler and debugging windows](profiler-and-debugging-windows.md)).
* **Asset Pipeline.** Auto Refresh, Directory Monitoring, import worker
  count, and compress textures on import.
* **Colors.** One color field per editor color, grouped: Animation (property
  animated, property candidate, property recorded), General (play-mode tint),
  Scene (background, grid, selected axis, X, Y, and Z axes, selected outline,
  selected children outline, wireframe, wireframe selected, and others). The
  play-mode tint colors the editor's chrome during play mode
  ([Game view and play mode](game-view-and-play-mode.md)). **Use Defaults**
  restores every color.
* **Diagnostics.** Switches grouped by area that enable additional editor
  diagnostics; each shows its default, a changed switch is marked, and a
  switch that takes effect after restart states it. **Reset all** restores
  defaults.
* **External Tools.** External Script Editor (a drop-down of the IDE
  integrations the project's packages provide, discovered installations, the
  operating system default, and **Browse...**), its arguments, the package's
  project-file generation checkboxes and **Regenerate project files**, as
  editor-and-authoring.md defines; Image application; Revision Control Diff/Merge
  tool (including the unitas semantic merge); and target SDK and toolchain
  paths with their installed state (PRD-001).
* **GI Cache.** Maximum cache size, custom cache location with **Browse...**,
  cache compression, the current size, and **Clean Cache**.
* **Package Manager.** The package cache location for this user and its
  current size.
* **Scene View.** General options (create objects at world origin), handle
  and line options, search options (enable filtering while searching), and
  the scene view's scroll and navigation sensitivities.
* **Search.** Indexing options and the search providers listed with an
  enable toggle and order each, the default provider, and whether search
  shows the status bar.
* **Timeline.** The package's preferences (time unit display, audio waveform,
  audio scrubbing, snap to frame, edge snap) when the package is present.
* **UI Scaling.** Use the default desktop setting or a chosen editor scale,
  applied at the next restart, which the page states.
* **Agents.** The only page unitas adds to Preferences, listed among the
  others alphabetically: the color assigned to each known agent identity and
  the rule for new ones; whether live badges, unreviewed badges, and agent
  selections show in rows and the Scene view; whether agents may change the
  person's selection and move the person's Scene view cameras (both off by
  default);
  whether the Agent Activity window opens as a trailing tab when the first
  agent session connects; notifications for agent requests awaiting a
  person; the enabled MCP transports and the HTTP transport origin list;
  standing grants with **Revoke**; and retention limits, all per-user
  configuration (AUT-007). Enabling a network transport, changing grants,
  revoking, and every setting that widens what agents may do (including the
  selection and camera permissions) are person-only and are never available through MCP;
  everything else on the page is readable by agents with the read-project
  scope ([Agent-native additions](agent-native-additions.md)).

The keyboard shortcut editor is the Shortcuts Manager (`Edit/Shortcuts...`),
defined in [Shortcuts](shortcuts.md), not a Preferences page.

## Package Manager window

The Package Manager window opens from `Window/Package Manager`. It edits
`Packages/manifest.json` through the resolution of PRJ-006.

### Regions

* **Toolbar.** A **+** menu with **Add package from disk...**, **Add package
  from tarball...**, **Add package from git URL...**, and **Add package by
  name...**; a **Packages** drop-down choosing the list's context — **unitas
  Implementations** (the unitas implementations of supported Unity packages,
  which resolve the project's Unity package names), **In Project**, **My
  Assets**, **Built-in**, and one entry per scoped registry; a **Sort**
  drop-down (name, published date); a **Filters** drop-down (updates available,
  release state); a search field; and a settings menu with **Advanced Project
  Settings** (opens the Project Settings Package Manager page) and **Manual
  resolve**.
* **List pane.** On the left, packages grouped by feature set and author,
  each row with display name, version, and status markers: installed, update
  available, pre-release, experimental, deprecated, locked by the lock file,
  error, and warning. In In Project, direct dependencies are listed first and
  indirect dependencies below them when Show Dependencies is on.
* **Details pane.** On the right: display name, version and release state,
  author, source (registry, git with revision, local path, tarball, embedded,
  or built-in), links to documentation, changelog, and licenses, the
  description, the compatibility state from the supported package matrix
  (CMP-002), and tabs for **Version History**, **Dependencies** (what it
  depends on and what depends on it, each a link), **Samples** (each with
  size and **Import** or **Reimport**), and **Images** where provided.
* **Action bar.** **Install**, **Remove**, **Update to** a version,
  **Unlock** for a locked package, and, for built-in modules, **Enable** and
  **Disable**.

### Behavior

* Every add, remove, update, downgrade, enable, or disable first states the
  resulting dependency changes (added, removed, and changed packages with
  versions) and proceeds on confirmation; the manifest and lock file are then
  updated in Unity-readable form, resolution runs with progress in the window
  and the status bar, and the project recompiles as needed.
* Add from disk takes a folder's `package.json`; add from tarball takes a
  `.tgz`; add from git URL takes a URL with an optional revision and path;
  add by name takes a name and optional version. Invalid input states why
  before anything changes.
* Fetching from a project-named source without fetch consent (PRJ-008) shows
  the consent request instead of fetching. An unreachable registry, missing
  version, or unresolvable conflict shows an error banner at the bottom of
  the window naming the package, the source, and the requesting chain, with
  **Retry**; the error also appears in the [Console](console.md).
* Offline, the window states that it is offline, lists packages resolvable
  from the lock file and local content, and marks the rest unresolved.
* Importing a sample copies it into `Assets/Samples/<package>/<version>/`;
  reimporting over a modified copy asks to overwrite.
* Package content of registry, git, tarball, and built-in packages is
  read-only, and the details pane says so; embedded and local packages are
  editable.
* **My Assets** lists the store packages owned by the signed-in store account
  (CMP-003); without a signed-in account, the list states that sign-in is
  required and offers it.
* **Empty states.** A search with no results says so and offers to clear it;
  In Project with only built-in modules states that the project has no added
  packages.

## Build Settings window

The Build Settings window opens from `File/Build Settings...`
(Ctrl/Cmd+Shift+B).

### Regions

* **Scenes In Build.** A list at the top. Each row has an enable checkbox, the
  scene's path, and, at the right edge, its build index. Indices count only
  enabled scenes, from zero, and update as rows change. Dragging scenes from
  the [Project window](project-window.md) or Hierarchy scene headers onto
  the list adds them at the drop point; dragging rows reorders them;
  Delete removes the selected rows; the context menu offers **Remove
  Selection**. **Add Open Scenes** appends the open scenes not yet listed. A
  listed scene that no longer exists shows as missing and is excluded from
  the build with a warning.
* **Platform list.** On the left, the build targets, the active one marked as
  active. A target whose toolchain or platform support is not installed is
  listed with that state and an **Install** action (PRD-001); a target whose
  platform holder SDK is required shows its requirement.
* **Platform options.** On the right, the selected target's options, such as
  texture compression, architecture, run device with **Refresh**,
  **Development Build**, **Autoconnect Profiler**, **Deep Profiling
  Support**, **Script Debugging**, **Wait For Managed Debugger**, **Scripts
  Only Build**, and compression method. Options that depend on Development
  Build are disabled without it.
* **Bottom bar.** **Player Settings...** (opens the Project Settings Player
  page at the selected target's tab), **Switch Platform** when the selected
  target is not active, and **Build** and **Build And Run** (Ctrl/Cmd+B) with
  a drop-down offering **Clean Build...**.

### Behavior

* **Switch Platform** makes the selected target active, reimports
  target-dependent assets with progress in the status bar and Progress
  window, and recompiles for the target's defines; it is cancellable while
  reimporting and, when cancelled, the previous target stays active.
* **Build** asks for an output location and builds; **Build And Run** builds
  to the last location for that target and deploys to the run device. Build
  validation, progress, results, and failures are as BLD-001 defines; the
  build report opens on completion.
* Build options are stored with the build settings and player settings as
  PRJ-007 and BLD-001 define, and edits to them dirty those settings.
* A build with no enabled scene builds the open scene and states that it is
  doing so.

### Shortcuts

Global defaults from [Shortcuts](shortcuts.md) that these windows use:

| Context | Action | Default |
|---|---|---|
| Global | Build Settings | Ctrl/Cmd+Shift+B |
| Global | Build And Run | Ctrl/Cmd+B |
| Global (macOS) | Preferences | Cmd+, |
| Settings windows | Focus search field | Ctrl/Cmd+F |
| Settings windows | Previous or next page | Up, Down (category list focused) |
| Build Settings (Scenes In Build) | Remove selected scenes | Delete (Cmd+Backspace on macOS) |
| Package Manager | Focus search field | Ctrl/Cmd+F |

Project Settings, Preferences, and Package Manager have no global default
shortcut.

### Agent presence

* A settings page or field that an agent session is changing shows that
  session's agent badge on the category-list row and beside the field's label;
  the hover names the agent session and its current operation, and a click
  opens the Change Review window filtered to that change
  ([Agent-native additions](agent-native-additions.md)).
* A package that an agent session added, removed, or changed shows the badge
  on its list row and in the details pane, with the dependency changes that
  the agent's request produced.
* Project settings changes and package manifest changes are risk-flagged in
  the agent session's change set (EDT-008). Under the require-approval policy,
  a proposed settings or package change previews on its page and in the
  Package Manager as proposed, and `ProjectSettings/` and `Packages/` stay
  unchanged until approval.
* A build that an agent session started shows the badge in the Build
  Settings bottom bar and the Progress window while it runs.
* The Preferences Agents page and the Project Settings unitas page are the
  only additions; agent presence never moves, hides, or rebinds a control of
  these windows.

### Scale

* The category list, the Package Manager list, the Input Manager axes list,
  the Tags and Layers lists, the Script Execution Order list, and Scenes In
  Build realize only visible rows.
* Search across settings pages uses declared keywords and an index of page
  labels, so it does not draw every page to search it.
* Package Manager lists from large registries are paginated and load further
  pages as the list scrolls; resolution and fetching run off the main thread.

## Acceptance outcomes

1. Typing a provider keyword into the Project Settings search field lists only
   matching pages, including a project `SettingsProvider`, and highlights
   matching labels on the shown page; an agent reads the same page's fields
   through MCP. (EDT-006, EDT-002)
2. Changing Time's Fixed Timestep dirties the time settings file, Save Project
   writes it in Unity-readable form, and undo restores the previous value.
   (EDT-002, EDT-005, PRJ-007)
3. Changing a Preferences color, including the play-mode tint, persists across
   reopening the project and dirties no project content. (EDT-006)
4. On the Player page, a scripting define added for one target tab shows as an
   override beside the default and states that it recompiles on Apply.
   (EDT-002, PRJ-007)
5. Reordering scripts on the Script Execution Order page and pressing Apply
   writes each changed script's `.meta` file, and Revert discards unapplied
   changes. (EDT-002, PRJ-007, RUN-003)
6. Removing a layer used by objects lists those objects before committing.
   (EDT-001, EDT-002)
7. Adding a package by git URL in the Package Manager states the dependency
   changes before committing, updates the manifest and lock file, and an
   unreachable URL shows an error banner with Retry and changes nothing.
   (EDT-006, PRJ-006)
8. Importing a package sample copies it under `Assets/Samples/` and the
   Project window shows it. (EDT-006, PRJ-006)
9. Dragging two scenes into Scenes In Build, disabling the first, and
   reordering updates the build indices so that only enabled scenes are
   numbered, in order. (EDT-006, BLD-001)
10. Switch Platform to another installed target reimports target-dependent
    assets with progress and, when cancelled, keeps the previous target
    active. (EDT-006, BLD-001)
11. A package change made by an agent session shows that session's badge in
    the Package Manager and appears in its change set as a risk-flagged
    package manifest item; under require approval, `Packages/manifest.json`
    is unchanged until approval. (EDT-008)
12. An agent session's request to change the review policy on the unitas
    Project Settings page returns a permission outcome and changes nothing,
    and a person changes it there. (EDT-008, AUT-007)
13. Ctrl/Cmd+Shift+B opens Build Settings and Ctrl/Cmd+B starts Build And
    Run for the active target; through MCP, `File/Build Settings...` and
    `File/Build And Run` are invocable by their menu paths. (EDT-006)
