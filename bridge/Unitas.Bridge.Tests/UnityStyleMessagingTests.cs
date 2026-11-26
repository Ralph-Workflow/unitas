using System.Collections.Generic;
using System.Runtime.CompilerServices;
using Unitas.Bridge.Messaging;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    // Unity-canonical style: the magic methods are private and non-virtual, so
    // only the generated glue handler can reach them.
    public sealed class UnityStylePrivateScript : MonoBehaviour
    {
        public static List<string>? SharedTrace { get; set; }

        public string TagName { get; set; } = "Private";

        private new void Awake() => SharedTrace?.Add(TagName + ":Awake");

        private new void OnEnable() => SharedTrace?.Add(TagName + ":OnEnable");

        private new void Start() => SharedTrace?.Add(TagName + ":Start");

        private new void FixedUpdate() => SharedTrace?.Add(TagName + ":FixedUpdate");

        private new void Update() => SharedTrace?.Add(TagName + ":Update");

        private new void LateUpdate() => SharedTrace?.Add(TagName + ":LateUpdate");

        private new void OnTriggerEnter(Collider other) => SharedTrace?.Add(TagName + ":OnTriggerEnter:" + other.gameObject.name);
    }

    // The override style the engine already supports.
    public sealed class UnityStyleOverrideScript : MonoBehaviour
    {
        public static List<string>? SharedTrace { get; set; }

        public string TagName { get; set; } = "Override";

        public override void Awake() => SharedTrace?.Add(TagName + ":Awake");

        public override void OnEnable() => SharedTrace?.Add(TagName + ":OnEnable");

        public override void Start() => SharedTrace?.Add(TagName + ":Start");

        public override void FixedUpdate() => SharedTrace?.Add(TagName + ":FixedUpdate");

        public override void Update() => SharedTrace?.Add(TagName + ":Update");

        public override void LateUpdate() => SharedTrace?.Add(TagName + ":LateUpdate");
    }

    // Mirrors the generated glue handler: a private method is reached through
    // an accessor, a public one is called directly.
    public sealed class UnityStylePrivateHandler : IUnitasMessageHandler
    {
        private readonly UnityStylePrivateScript _target;

        public UnityStylePrivateHandler(UnityStylePrivateScript target)
        {
            _target = target;
        }

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "Awake")]
        private static extern void Call0_Awake(UnityStylePrivateScript target);

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "OnEnable")]
        private static extern void Call0_OnEnable(UnityStylePrivateScript target);

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "Start")]
        private static extern void Call0_Start(UnityStylePrivateScript target);

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "FixedUpdate")]
        private static extern void Call0_FixedUpdate(UnityStylePrivateScript target);

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "Update")]
        private static extern void Call0_Update(UnityStylePrivateScript target);

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "LateUpdate")]
        private static extern void Call0_LateUpdate(UnityStylePrivateScript target);

        [UnsafeAccessor(UnsafeAccessorKind.Method, Name = "OnTriggerEnter")]
        private static extern void Call1_OnTriggerEnter(UnityStylePrivateScript target, Collider arg);

        public bool Dispatch(string message)
        {
            switch (message)
            {
                case "Awake":
                    Call0_Awake(_target);
                    return true;
                case "OnEnable":
                    Call0_OnEnable(_target);
                    return true;
                case "Start":
                    Call0_Start(_target);
                    return true;
                case "FixedUpdate":
                    Call0_FixedUpdate(_target);
                    return true;
                case "Update":
                    Call0_Update(_target);
                    return true;
                case "LateUpdate":
                    Call0_LateUpdate(_target);
                    return true;
                default:
                    return false;
            }
        }

        public bool Dispatch<T>(string message, T argument)
        {
            if (message == "OnTriggerEnter" && argument is Collider typed)
            {
                Call1_OnTriggerEnter(_target, typed);
                return true;
            }

            return Dispatch(message);
        }
    }

    public sealed class UnityStyleOverrideHandler : IUnitasMessageHandler
    {
        private readonly UnityStyleOverrideScript _target;

        public UnityStyleOverrideHandler(UnityStyleOverrideScript target)
        {
            _target = target;
        }

        public bool Dispatch(string message)
        {
            switch (message)
            {
                case "Awake":
                    _target.Awake();
                    return true;
                case "OnEnable":
                    _target.OnEnable();
                    return true;
                case "Start":
                    _target.Start();
                    return true;
                case "FixedUpdate":
                    _target.FixedUpdate();
                    return true;
                case "Update":
                    _target.Update();
                    return true;
                case "LateUpdate":
                    _target.LateUpdate();
                    return true;
                default:
                    return false;
            }
        }

        public bool Dispatch<T>(string message, T argument) => Dispatch(message);
    }

    public static class UnityStyleMessagingTests
    {
        public static void Run()
        {
            TestUnityStyleScriptsReceiveTheUnityMessageOrder();
            TestPrivateTriggerMessageCarriesTheCollider();
            TestGlueEmitsNoDispatchCaseForOverrideMethods();
        }

        private static void TestGlueEmitsNoDispatchCaseForOverrideMethods()
        {
            string tempDir = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                "unitas_glue_override_" + System.Guid.NewGuid().ToString("N"));
            System.IO.Directory.CreateDirectory(tempDir);
            try
            {
                string scriptPath = System.IO.Path.Combine(tempDir, "MixedStyles.cs");
                System.IO.File.WriteAllText(
                    scriptPath,
                    "using UnityEngine;\n" +
                    "public class OverrideStyle : MonoBehaviour { public override void Update() { base.Update(); } }\n" +
                    "public class CanonicalStyle : MonoBehaviour { private new void Update() { } }\n");

                HashSet<string> defines = new HashSet<string>(System.StringComparer.Ordinal);
                Dictionary<string, string> pathToGuid = new Dictionary<string, string>(System.StringComparer.Ordinal)
                {
                    [scriptPath] = "40000000000000000000000000000078"
                };

                List<Unitas.Scripting.ScriptGuidBinding> bindings =
                    Unitas.Scripting.ScriptAnalyzerAndGlueGenerator.DiscoverScriptBindingsForAssembly(
                        "Assembly-CSharp",
                        new[] { scriptPath },
                        defines,
                        pathToGuid);
                string glueSource = Unitas.Scripting.ScriptAnalyzerAndGlueGenerator.GenerateGlueSource(
                    bindings,
                    new List<Unitas.Scripting.SerializableTypeBinding>());

                int updateCases = 0;
                int searchFrom = 0;
                while (true)
                {
                    int found = glueSource.IndexOf("case \"Update\":", searchFrom, System.StringComparison.Ordinal);
                    if (found < 0)
                    {
                        break;
                    }

                    updateCases++;
                    searchFrom = found + 1;
                }

                Check.Equal(
                    "Glue emits a dispatch case only for the non-override Update, so an override that calls base runs once",
                    1,
                    updateCases);
                Check.That(
                    "Glue still reaches the canonical private Update through an accessor",
                    glueSource.Contains("Call0_Update", System.StringComparison.Ordinal));
            }
            finally
            {
                if (System.IO.Directory.Exists(tempDir))
                {
                    System.IO.Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestUnityStyleScriptsReceiveTheUnityMessageOrder()
        {
            List<string> trace = new List<string>();
            UnityStylePrivateScript.SharedTrace = trace;
            UnityStyleOverrideScript.SharedTrace = trace;

            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject privateHost = Track(created, new GameObject("UnityStylePrivateHost"));
                GameObject overrideHost = Track(created, new GameObject("UnityStyleOverrideHost"));

                UnityStylePrivateScript privateScript = privateHost.AddComponent<UnityStylePrivateScript>();
                privateScript.TagName = "Private";
                UnityStyleOverrideScript overrideScript = overrideHost.AddComponent<UnityStyleOverrideScript>();
                overrideScript.TagName = "Override";

                // The generated glue registers its handler this way.
                MessageDispatcher.RegisterHandler(privateScript, new UnityStylePrivateHandler(privateScript));
                MessageDispatcher.RegisterHandler(overrideScript, new UnityStyleOverrideHandler(overrideScript));

                MonoBehaviour.RunLifecycleBatch(new MonoBehaviour[] { privateScript, overrideScript }, runFixedUpdate: true);

                Check.Equal(
                    "A Unity-canonical private script receives Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate in Unity order",
                    "Awake,OnEnable,Start,FixedUpdate,Update,LateUpdate",
                    JoinTag(trace, "Private"));
                Check.Equal(
                    "An override script still receives Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate in Unity order",
                    "Awake,OnEnable,Start,FixedUpdate,Update,LateUpdate",
                    JoinTag(trace, "Override"));
                Check.Equal(
                    "A Unity-canonical private script receives Awake exactly once",
                    1,
                    CountTag(trace, "Private:Awake"));
                Check.Equal(
                    "A Unity-canonical private script receives Update exactly once",
                    1,
                    CountTag(trace, "Private:Update"));
                Check.Equal(
                    "An override script receives Awake exactly once",
                    1,
                    CountTag(trace, "Override:Awake"));
                Check.Equal(
                    "An override script receives Update exactly once",
                    1,
                    CountTag(trace, "Override:Update"));
                Check.Equal(
                    "A Unity-canonical private script runs FixedUpdate in the fixed phase before Update",
                    true,
                    trace.IndexOf("Private:FixedUpdate") < trace.IndexOf("Private:Update"));
            }
            finally
            {
                UnityStylePrivateScript.SharedTrace = null;
                UnityStyleOverrideScript.SharedTrace = null;
                DestroyAll(created);
            }
        }

        private static void TestPrivateTriggerMessageCarriesTheCollider()
        {
            List<string> trace = new List<string>();
            UnityStylePrivateScript.SharedTrace = trace;

            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject host = Track(created, new GameObject("UnityStyleTriggerHost"));
                GameObject other = Track(created, new GameObject("TriggerOther"));

                UnityStylePrivateScript script = host.AddComponent<UnityStylePrivateScript>();
                script.TagName = "Private";
                MessageDispatcher.RegisterHandler(script, new UnityStylePrivateHandler(script));

                Collider collider = other.AddComponent<SphereCollider>();
                script.OnTriggerEnter(collider);

                Check.Equal(
                    "A Unity-canonical private OnTriggerEnter receives the trigger collider",
                    "Private:OnTriggerEnter:TriggerOther",
                    string.Join(",", trace));
                Check.Equal(
                    "A Unity-canonical private OnTriggerEnter is called exactly once",
                    1,
                    trace.Count);
            }
            finally
            {
                UnityStylePrivateScript.SharedTrace = null;
                DestroyAll(created);
            }
        }

        private static string JoinTag(List<string> trace, string tag)
        {
            List<string> entries = new List<string>();
            for (int i = 0; i < trace.Count; i++)
            {
                if (trace[i].StartsWith(tag + ":", System.StringComparison.Ordinal))
                {
                    entries.Add(trace[i].Substring(tag.Length + 1));
                }
            }

            return string.Join(",", entries);
        }

        private static int CountTag(List<string> trace, string entry)
        {
            int count = 0;
            for (int i = 0; i < trace.Count; i++)
            {
                if (trace[i] == entry)
                {
                    count++;
                }
            }

            return count;
        }

        private static GameObject Track(List<UnityEngine.Object> list, GameObject instance)
        {
            list.Add(instance);
            return instance;
        }

        private static void DestroyAll(List<UnityEngine.Object> list)
        {
            for (int i = list.Count - 1; i >= 0; i--)
            {
                UnityEngine.Object.Destroy(list[i]);
            }
        }
    }
}
