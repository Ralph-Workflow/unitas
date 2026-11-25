using System;
using UnityEngine;

public class AppRecorderB : MonoBehaviour
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
        Debug.Log("APP_TRACE:B:Focus:" + (hasFocus ? "True" : "False"));
        if (ThrowOnFocus)
        {
            throw new InvalidOperationException("B focus boom");
        }
    }

    public override void OnApplicationPause(bool pauseStatus)
    {
        Debug.Log("APP_TRACE:B:Pause:" + (pauseStatus ? "True" : "False"));
        if (ThrowOnPause)
        {
            throw new InvalidOperationException("B pause boom");
        }
    }

    public override void OnApplicationQuit()
    {
        Debug.Log("APP_TRACE:B:Quit");
        if (ThrowOnQuit)
        {
            throw new InvalidOperationException("B quit boom");
        }
    }

    public override void OnDisable()
    {
        Debug.Log("APP_TRACE:B:OnDisable");
    }

    public override void OnEnable()
    {
        Debug.Log("APP_TRACE:B:OnEnable");
    }

    public override void OnDestroy()
    {
        Debug.Log("APP_TRACE:B:OnDestroy");
    }
}
