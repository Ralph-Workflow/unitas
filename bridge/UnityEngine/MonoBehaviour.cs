namespace UnityEngine
{
    public class MonoBehaviour : Behaviour
    {
        public MonoBehaviour()
        {
            GameObject owner = new GameObject("Scripted Game Object");
            owner.AttachComponent(this);
            owner.AddComponent<SphereCollider>();
        }

        public Coroutine StartCoroutine(System.Collections.IEnumerator routine)
        {
            return CoroutineScheduler.Start(this, routine);
        }

        public void StopCoroutine(Coroutine coroutine)
        {
            CoroutineScheduler.Stop(coroutine);
        }

        public void StopAllCoroutines()
        {
            CoroutineScheduler.StopAll(this);
        }

        public virtual void Awake()
        {
        }

        public virtual void OnEnable()
        {
        }

        public virtual void Start()
        {
        }

        public virtual void FixedUpdate()
        {
        }

        public virtual void Update()
        {
        }

        public virtual void LateUpdate()
        {
        }

        public virtual void OnDisable()
        {
        }

        public virtual void OnDestroy()
        {
        }

        public virtual void OnCollisionEnter(Collision collision)
        {
        }

        public virtual void OnCollisionStay(Collision collision)
        {
        }

        public virtual void OnCollisionExit(Collision collision)
        {
        }

        public virtual void OnTriggerEnter(Collider? other)
        {
        }

        public virtual void OnTriggerStay(Collider? other)
        {
        }

        public virtual void OnTriggerExit(Collider? other)
        {
        }
    }
}
