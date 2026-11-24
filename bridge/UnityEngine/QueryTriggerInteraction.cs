namespace UnityEngine
{
    public enum QueryTriggerInteraction
    {
        UseGlobal,
        Ignore,
        Collide
    }

    public static class QueryTriggerInteractionExtensions
    {
        public static bool IncludesTriggers(this QueryTriggerInteraction queryTriggerInteraction)
        {
            return queryTriggerInteraction switch
            {
                QueryTriggerInteraction.Ignore => false,
                QueryTriggerInteraction.Collide => true,
                _ => Physics.queriesHitTriggers,
            };
        }
    }
}
