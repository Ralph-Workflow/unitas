using System;
using UnityEngine;

public class AppRecorderA : MonoBehaviour
{
    public bool ThrowOnFocus;
    public bool ThrowOnPause;
    public bool ThrowOnQuit;

    public void SetThrowOnFocus(bool value)
    {
        ThrowOnFocus = value;
    }

    public void SetThrowOnPause(bool value)
    {
        ThrowOnPause = value;
    }

    public void SetThrowOnQuit(bool value)
    {
        ThrowOnQuit = value;
    }

    public override void OnApplicationFocus(bool hasFocus)
    {
        Debug.Log("APP_TRACE:A:Focus:" + (hasFocus ? "True" : "False"));
        if (ThrowOnFocus)
        {
            throw new InvalidOperationException("A focus boom");
        }
    }

    public override void OnApplicationPause(bool pauseStatus)
    {
        Debug.Log("APP_TRACE:A:Pause:" + (pauseStatus ? "True" : "False"));
        if (ThrowOnPause)
        {
            throw new InvalidOperationException("A pause boom");
        }
    }

    public override void OnApplicationQuit()
    {
        Debug.Log("APP_TRACE:A:Quit");
        if (ThrowOnQuit)
        {
            throw new InvalidOperationException("A quit boom");
        }
    }

    public override void OnDisable()
    {
        Debug.Log("APP_TRACE:A:OnDisable");
    }

    public override void OnEnable()
    {
        Debug.Log("APP_TRACE:A:OnEnable");
    }

    public override void OnDestroy()
    {
        Debug.Log("APP_TRACE:A:OnDestroy");
    }
}
