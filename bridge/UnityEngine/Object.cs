using System.Threading;

namespace UnityEngine
{
    public class Object
    {
        private static int s_nextInstanceId = 1;
        private readonly int _instanceId = Interlocked.Increment(ref s_nextInstanceId);
        private string _name = string.Empty;

        public virtual string name
        {
            get => _name;
            set => _name = value;
        }

        public int GetInstanceID() => _instanceId;

        public override string ToString() => name;
    }
}
