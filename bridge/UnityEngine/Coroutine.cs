using System;

namespace UnityEngine
{
    public class YieldInstruction
    {
    }

    public sealed class WaitForSeconds : YieldInstruction
    {
        public WaitForSeconds(float seconds)
        {
            Seconds = seconds;
        }

        public float Seconds { get; }
    }

    public sealed class WaitForSecondsRealtime : YieldInstruction
    {
        public WaitForSecondsRealtime(float seconds)
        {
            Seconds = seconds;
        }

        public float Seconds { get; }
    }

    public sealed class WaitForFixedUpdate : YieldInstruction
    {
    }

    public sealed class WaitForEndOfFrame : YieldInstruction
    {
    }

    public sealed class WaitUntil : YieldInstruction
    {
        public WaitUntil(Func<bool> predicate)
        {
            if (predicate == null)
            {
                throw new ArgumentNullException(nameof(predicate));
            }

            Predicate = predicate;
        }

        public Func<bool> Predicate { get; }
    }

    public sealed class Coroutine : YieldInstruction
    {
        internal bool IsCompleted { get; set; }
    }
}
