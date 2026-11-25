namespace UnityEngine
{
    public interface IUnitasMessageHandler
    {
        bool Dispatch(string message);

        bool Dispatch<T>(string message, T argument);
    }
}
