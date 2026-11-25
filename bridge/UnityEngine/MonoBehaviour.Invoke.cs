namespace UnityEngine
{
    public partial class MonoBehaviour
    {
        public void Invoke(string methodName, float time)
        {
            InvokeScheduler.Schedule(this, methodName, time);
        }

        public void InvokeRepeating(string methodName, float time, float repeatRate)
        {
            InvokeScheduler.ScheduleRepeating(this, methodName, time, repeatRate);
        }

        public void CancelInvoke()
        {
            InvokeScheduler.CancelAll(this);
        }

        public void CancelInvoke(string methodName)
        {
            InvokeScheduler.Cancel(this, methodName);
        }

        public bool IsInvoking()
        {
            return InvokeScheduler.IsInvoking(this);
        }

        public bool IsInvoking(string methodName)
        {
            return InvokeScheduler.IsInvoking(this, methodName);
        }
    }
}
