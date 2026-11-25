using System;
using UnityEngine;

public class AppRecorderS1 : MonoBehaviour
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
        Debug.Log("APP_TRACE:S1:Focus:" + (hasFocus ? "True" : "False"));
        if (ThrowOnFocus)
        {
            throw new InvalidOperationException("S1 focus boom");
        }
    }

    public override void OnApplicationPause(bool pauseStatus)
    {
        Debug.Log("APP_TRACE:S1:Pause:" + (pauseStatus ? "True" : "False"));
        if (ThrowOnPause)
        {
            throw new InvalidOperationException("S1 pause boom");
        }
    }

    public override void OnApplicationQuit()
    {
        Debug.Log("APP_TRACE:S1:Quit");
        if (ThrowOnQuit)
        {
            throw new InvalidOperationException("S1 quit boom");
        }
    }

    public override void OnDisable()
    {
        Debug.Log("APP_TRACE:S1:OnDisable");
    }

    public override void OnEnable()
    {
        Debug.Log("APP_TRACE:S1:OnEnable");
    }

    public override void OnDestroy()
    {
        Debug.Log("APP_TRACE:S1:OnDestroy");
    }
}
