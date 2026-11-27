using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public partial class MonoBehaviour : Behaviour
    {
        private static readonly List<(Func<MonoBehaviour, bool> Matcher, int Order)> s_executionOrderOverrides =
            new List<(Func<MonoBehaviour, bool> Matcher, int Order)>();

        private static readonly List<(Func<MonoBehaviour, bool> Matcher, int Order)> s_registeredDefaultExecutionOrders =
            new List<(Func<MonoBehaviour, bool> Matcher, int Order)>();

        public MonoBehaviour()
        {
        }

        internal IUnitasMessageHandler? MessageHandler { get; set; }

        public virtual int DefaultExecutionOrderValue => 0;

        public static void RegisterDefaultExecutionOrder<TScript>(int order)
            where TScript : MonoBehaviour
        {
            s_registeredDefaultExecutionOrders.Add((instance => instance is TScript, order));
        }

        public static void SetExecutionOrderOverride<TScript>(int order)
            where TScript : MonoBehaviour
        {
            s_executionOrderOverrides.Add((instance => instance is TScript, order));
        }

        public static void SetExecutionOrderOverride(Type scriptType, int order)
        {
            if (scriptType == null)
            {
                throw new ArgumentNullException(nameof(scriptType));
            }

            if (!typeof(MonoBehaviour).IsAssignableFrom(scriptType))
            {
                throw new ArgumentException("Type must derive from UnityEngine.MonoBehaviour.", nameof(scriptType));
            }

            s_executionOrderOverrides.Add((instance => scriptType.IsInstanceOfType(instance), order));
        }

        internal static void SetExecutionOrderOverride(Func<MonoBehaviour, bool> matcher, int order)
        {
            if (matcher == null)
            {
                throw new ArgumentNullException(nameof(matcher));
            }

            s_executionOrderOverrides.Add((matcher, order));
        }

        internal static bool RemoveExecutionOrderOverride(Func<MonoBehaviour, bool> matcher)
        {
            if (matcher == null)
            {
                throw new ArgumentNullException(nameof(matcher));
            }

            for (int index = s_executionOrderOverrides.Count - 1; index >= 0; index--)
            {
                if (s_executionOrderOverrides[index].Matcher == matcher)
                {
                    s_executionOrderOverrides.RemoveAt(index);
                    return true;
                }
            }

            return false;
        }

        public static void ClearExecutionOrderOverrides()
        {
            s_executionOrderOverrides.Clear();
        }

        public static void ClearRegisteredDefaultExecutionOrders()
        {
            s_registeredDefaultExecutionOrders.Clear();
        }

        public static int GetEffectiveExecutionOrder(MonoBehaviour script)
        {
            if (script is null)
            {
                throw new ArgumentNullException(nameof(script));
            }

            for (int i = s_executionOrderOverrides.Count - 1; i >= 0; i--)
            {
                if (s_executionOrderOverrides[i].Matcher(script))
                {
                    return s_executionOrderOverrides[i].Order;
                }
            }

            for (int i = s_registeredDefaultExecutionOrders.Count - 1; i >= 0; i--)
            {
                if (s_registeredDefaultExecutionOrders[i].Matcher(script))
                {
                    return s_registeredDefaultExecutionOrders[i].Order;
                }
            }

            return script.DefaultExecutionOrderValue;
        }

        public static MonoBehaviour[] OrderByExecutionOrder(IReadOnlyList<MonoBehaviour> scripts)
        {
            if (scripts == null)
            {
                throw new ArgumentNullException(nameof(scripts));
            }

            List<(MonoBehaviour Script, int Order, int Index)> indexed =
                new List<(MonoBehaviour Script, int Order, int Index)>(scripts.Count);
            for (int i = 0; i < scripts.Count; i++)
            {
                MonoBehaviour candidate = scripts[i];
                if (candidate is not null)
                {
                    indexed.Add((candidate, GetEffectiveExecutionOrder(candidate), i));
                }
            }

            indexed.Sort((left, right) =>
            {
                int cmp = left.Order.CompareTo(right.Order);
                return cmp != 0 ? cmp : left.Index.CompareTo(right.Index);
            });

            MonoBehaviour[] ordered = new MonoBehaviour[indexed.Count];
            for (int i = 0; i < indexed.Count; i++)
            {
                ordered[i] = indexed[i].Script;
            }

            return ordered;
        }

        public static void RunLifecycleBatch(IReadOnlyList<MonoBehaviour> scripts, bool runFixedUpdate = false)
        {
            MonoBehaviour[] ordered = OrderByExecutionOrder(scripts);

            for (int i = 0; i < ordered.Length; i++)
            {
                MonoBehaviour s = ordered[i];
                if (!s.IsDestroyed && s.gameObject != null && s.gameObject.activeInHierarchy)
                {
                    s.Awake();
                }
            }

            for (int i = 0; i < ordered.Length; i++)
            {
                MonoBehaviour s = ordered[i];
                if (!s.IsDestroyed && s.isActiveAndEnabled)
                {
                    s.OnEnable();
                }
            }

            for (int i = 0; i < ordered.Length; i++)
            {
                MonoBehaviour s = ordered[i];
                if (!s.IsDestroyed && s.isActiveAndEnabled)
                {
                    s.Start();
                }
            }

            if (runFixedUpdate)
            {
                for (int i = 0; i < ordered.Length; i++)
                {
                    MonoBehaviour s = ordered[i];
                    if (!s.IsDestroyed && s.isActiveAndEnabled)
                    {
                        s.FixedUpdate();
                    }
                }
            }

            for (int i = 0; i < ordered.Length; i++)
            {
                MonoBehaviour s = ordered[i];
                if (!s.IsDestroyed && s.isActiveAndEnabled)
                {
                    s.Update();
                }
            }

            for (int i = 0; i < ordered.Length; i++)
            {
                MonoBehaviour s = ordered[i];
                if (!s.IsDestroyed && s.isActiveAndEnabled)
                {
                    s.LateUpdate();
                }
            }
        }

        public Coroutine StartCoroutine(System.Collections.IEnumerator routine)
        {
            return CoroutineScheduler.Start(this, routine);
        }

        public Coroutine? StartCoroutine(string methodName)
        {
            return CoroutineScheduler.StartByName(this, methodName);
        }

        public Coroutine? StartCoroutine<T>(string methodName, T value)
        {
            return CoroutineScheduler.StartByName(this, methodName, value);
        }

        public void StopCoroutine(Coroutine coroutine)
        {
            CoroutineScheduler.Stop(coroutine);
        }

        public void StopCoroutine(System.Collections.IEnumerator routine)
        {
            CoroutineScheduler.Stop(this, routine);
        }

        public void StopCoroutine(string methodName)
        {
            CoroutineScheduler.StopByName(this, methodName);
        }

        public void StopAllCoroutines()
        {
            CoroutineScheduler.StopAll(this);
        }

        public virtual System.Collections.IEnumerator? ResolveNamedCoroutine(string methodName)
        {
            _ = methodName;
            return null;
        }

        public virtual System.Collections.IEnumerator? ResolveNamedCoroutine<T>(string methodName, T value)
        {
            _ = value;
            return ResolveNamedCoroutine(methodName);
        }

        public void RegisterNamedCoroutine(string methodName, Func<System.Collections.IEnumerator> factory)
        {
            CoroutineScheduler.RegisterMethod(this, methodName, factory);
        }

        public void RegisterNamedCoroutine<T>(string methodName, Func<T, System.Collections.IEnumerator> factory)
        {
            CoroutineScheduler.RegisterMethod(this, methodName, factory);
        }

        // A Unity-canonical script declares the magic methods private and
        // non-virtual, so only the generated glue handler reaches them. The base
        // virtual forwards the message to that handler. The glue emits no
        // handler for an override, so even an override that calls the base
        // virtual runs its body exactly once.
        private string? _dispatchingMessage;

        private IUnitasMessageHandler? ResolveScriptMessageHandler()
        {
            return MessageHandler ?? GameObject.ResolveMessageHandler(this);
        }

        private bool DispatchScriptMessage(string message)
        {
            if (_dispatchingMessage == message)
            {
                return false;
            }

            IUnitasMessageHandler? handler = ResolveScriptMessageHandler();
            if (handler == null)
            {
                return false;
            }

            string? previous = _dispatchingMessage;
            _dispatchingMessage = message;
            try
            {
                return handler.Dispatch(message);
            }
            finally
            {
                _dispatchingMessage = previous;
            }
        }

        private bool DispatchScriptMessage<T>(string message, T argument)
        {
            if (_dispatchingMessage == message)
            {
                return false;
            }

            IUnitasMessageHandler? handler = ResolveScriptMessageHandler();
            if (handler == null)
            {
                return false;
            }

            string? previous = _dispatchingMessage;
            _dispatchingMessage = message;
            try
            {
                return handler.Dispatch(message, argument);
            }
            finally
            {
                _dispatchingMessage = previous;
            }
        }

        public virtual void Awake()
        {
            DispatchScriptMessage("Awake");
        }

        public virtual void OnEnable()
        {
            DispatchScriptMessage("OnEnable");
        }

        public virtual void Start()
        {
            DispatchScriptMessage("Start");
        }

        public virtual void FixedUpdate()
        {
            DispatchScriptMessage("FixedUpdate");
        }

        public virtual void Update()
        {
            DispatchScriptMessage("Update");
        }

        public virtual void LateUpdate()
        {
            DispatchScriptMessage("LateUpdate");
        }

        public virtual void OnDisable()
        {
            DispatchScriptMessage("OnDisable");
        }

        public virtual void OnDestroy()
        {
            DispatchScriptMessage("OnDestroy");
        }

        public virtual void OnCollisionEnter(Collision collision)
        {
            DispatchScriptMessage("OnCollisionEnter", collision);
        }

        public virtual void OnCollisionStay(Collision collision)
        {
            DispatchScriptMessage("OnCollisionStay", collision);
        }

        public virtual void OnCollisionExit(Collision collision)
        {
            DispatchScriptMessage("OnCollisionExit", collision);
        }

        public virtual void OnTriggerEnter(Collider? other)
        {
            DispatchScriptMessage("OnTriggerEnter", other);
        }

        public virtual void OnTriggerStay(Collider? other)
        {
            DispatchScriptMessage("OnTriggerStay", other);
        }

        public virtual void OnTriggerExit(Collider? other)
        {
            DispatchScriptMessage("OnTriggerExit", other);
        }

        public virtual void OnCollisionEnter2D(Collision2D collision)
        {
            DispatchScriptMessage("OnCollisionEnter2D", collision);
        }

        public virtual void OnCollisionStay2D(Collision2D collision)
        {
            DispatchScriptMessage("OnCollisionStay2D", collision);
        }

        public virtual void OnCollisionExit2D(Collision2D collision)
        {
            DispatchScriptMessage("OnCollisionExit2D", collision);
        }

        public virtual void OnTriggerEnter2D(Collider2D? other)
        {
            DispatchScriptMessage("OnTriggerEnter2D", other);
        }

        public virtual void OnTriggerStay2D(Collider2D? other)
        {
            DispatchScriptMessage("OnTriggerStay2D", other);
        }

        public virtual void OnTriggerExit2D(Collider2D? other)
        {
            DispatchScriptMessage("OnTriggerExit2D", other);
        }

        public virtual void OnApplicationFocus(bool hasFocus)
        {
            DispatchScriptMessage("OnApplicationFocus", hasFocus);
        }

        public virtual void OnApplicationPause(bool pauseStatus)
        {
            DispatchScriptMessage("OnApplicationPause", pauseStatus);
        }

        public virtual void OnApplicationQuit()
        {
            DispatchScriptMessage("OnApplicationQuit");
        }

        public virtual void OnBecameVisible()
        {
        }

        public virtual void OnBecameInvisible()
        {
        }

        public virtual void OnMouseDown()
        {
        }

        public virtual void OnMouseUp()
        {
        }

        public virtual void OnMouseUpAsButton()
        {
        }

        public virtual void OnMouseEnter()
        {
        }

        public virtual void OnMouseExit()
        {
        }

        public virtual void OnMouseOver()
        {
        }

        public virtual void OnMouseDrag()
        {
        }

        public virtual void OnTransformParentChanged()
        {
        }

        public virtual void OnTransformChildrenChanged()
        {
        }
    }
}
