using System;
using System.Collections.Generic;

namespace UnityEngine.InputSystem
{
    public enum Key
    {
        None = 0,
        Space = 1,
        Enter = 2,
        Tab = 3,
        Backquote = 4,
        Quote = 5,
        Semicolon = 6,
        Comma = 7,
        Period = 8,
        Slash = 9,
        Backslash = 10,
        LeftBracket = 11,
        RightBracket = 12,
        Minus = 13,
        Equals = 14,
        A = 15,
        B = 16,
        C = 17,
        D = 18,
        E = 19,
        F = 20,
        G = 21,
        H = 22,
        I = 23,
        J = 24,
        K = 25,
        L = 26,
        M = 27,
        N = 28,
        O = 29,
        P = 30,
        Q = 31,
        R = 32,
        S = 33,
        T = 34,
        U = 35,
        V = 36,
        W = 37,
        X = 38,
        Y = 39,
        Z = 40,
        Digit1 = 41,
        Digit2 = 42,
        Digit3 = 43,
        Digit4 = 44,
        Digit5 = 45,
        Digit6 = 46,
        Digit7 = 47,
        Digit8 = 48,
        Digit9 = 49,
        Digit0 = 50,
        Alpha1 = Digit1,
        Alpha2 = Digit2,
        Alpha3 = Digit3,
        Alpha4 = Digit4,
        Alpha5 = Digit5,
        Alpha6 = Digit6,
        Alpha7 = Digit7,
        Alpha8 = Digit8,
        Alpha9 = Digit9,
        Alpha0 = Digit0,
        LeftShift = 51,
        RightShift = 52,
        LeftAlt = 53,
        RightAlt = 54,
        LeftCtrl = 55,
        RightCtrl = 56,
        LeftArrow = 57,
        RightArrow = 58,
        UpArrow = 59,
        DownArrow = 60,
        Escape = 61,
        Backspace = 62,
        Delete = 63,
    }

    public sealed class Keyboard : InputDevice
    {
        private static Keyboard _defaultInstance = new Keyboard();
        private static Keyboard? _customCurrent;
        private readonly Dictionary<Key, KeyControl> _keysByEnum = new Dictionary<Key, KeyControl>();
        private readonly List<KeyControl> _keyControls = new List<KeyControl>();

        public Keyboard()
            : base("Keyboard", "/Keyboard")
        {
            spaceKey = AddKey("space", Key.Space, KeyCode.Space, "spaceKey");
            enterKey = AddKey("enter", Key.Enter, KeyCode.Return, "enterKey", "return");
            tabKey = AddKey("tab", Key.Tab, KeyCode.Tab, "tabKey");
            backquoteKey = AddKey("backquote", Key.Backquote, KeyCode.None, "backquoteKey");
            quoteKey = AddKey("quote", Key.Quote, KeyCode.None, "quoteKey");
            semicolonKey = AddKey("semicolon", Key.Semicolon, KeyCode.None, "semicolonKey");
            commaKey = AddKey("comma", Key.Comma, KeyCode.None, "commaKey");
            periodKey = AddKey("period", Key.Period, KeyCode.None, "periodKey");
            slashKey = AddKey("slash", Key.Slash, KeyCode.None, "slashKey");
            backslashKey = AddKey("backslash", Key.Backslash, KeyCode.None, "backslashKey");
            leftBracketKey = AddKey("leftBracket", Key.LeftBracket, KeyCode.None, "leftBracketKey");
            rightBracketKey = AddKey("rightBracket", Key.RightBracket, KeyCode.None, "rightBracketKey");
            minusKey = AddKey("minus", Key.Minus, KeyCode.None, "minusKey");
            equalsKey = AddKey("equals", Key.Equals, KeyCode.None, "equalsKey");

            aKey = AddKey("a", Key.A, KeyCode.A, "aKey");
            bKey = AddKey("b", Key.B, KeyCode.B, "bKey");
            cKey = AddKey("c", Key.C, KeyCode.C, "cKey");
            dKey = AddKey("d", Key.D, KeyCode.D, "dKey");
            eKey = AddKey("e", Key.E, KeyCode.E, "eKey");
            fKey = AddKey("f", Key.F, KeyCode.F, "fKey");
            gKey = AddKey("g", Key.G, KeyCode.G, "gKey");
            hKey = AddKey("h", Key.H, KeyCode.H, "hKey");
            iKey = AddKey("i", Key.I, KeyCode.I, "iKey");
            jKey = AddKey("j", Key.J, KeyCode.J, "jKey");
            kKey = AddKey("k", Key.K, KeyCode.K, "kKey");
            lKey = AddKey("l", Key.L, KeyCode.L, "lKey");
            mKey = AddKey("m", Key.M, KeyCode.M, "mKey");
            nKey = AddKey("n", Key.N, KeyCode.N, "nKey");
            oKey = AddKey("o", Key.O, KeyCode.O, "oKey");
            pKey = AddKey("p", Key.P, KeyCode.P, "pKey");
            qKey = AddKey("q", Key.Q, KeyCode.Q, "qKey");
            rKey = AddKey("r", Key.R, KeyCode.R, "rKey");
            sKey = AddKey("s", Key.S, KeyCode.S, "sKey");
            tKey = AddKey("t", Key.T, KeyCode.T, "tKey");
            uKey = AddKey("u", Key.U, KeyCode.U, "uKey");
            vKey = AddKey("v", Key.V, KeyCode.V, "vKey");
            wKey = AddKey("w", Key.W, KeyCode.W, "wKey");
            xKey = AddKey("x", Key.X, KeyCode.X, "xKey");
            yKey = AddKey("y", Key.Y, KeyCode.Y, "yKey");
            zKey = AddKey("z", Key.Z, KeyCode.Z, "zKey");

            digit0Key = AddKey("0", Key.Digit0, KeyCode.Alpha0, "digit0Key", "digit0", "alpha0");
            digit1Key = AddKey("1", Key.Digit1, KeyCode.Alpha1, "digit1Key", "digit1", "alpha1");
            digit2Key = AddKey("2", Key.Digit2, KeyCode.Alpha2, "digit2Key", "digit2", "alpha2");
            digit3Key = AddKey("3", Key.Digit3, KeyCode.Alpha3, "digit3Key", "digit3", "alpha3");
            digit4Key = AddKey("4", Key.Digit4, KeyCode.Alpha4, "digit4Key", "digit4", "alpha4");
            digit5Key = AddKey("5", Key.Digit5, KeyCode.Alpha5, "digit5Key", "digit5", "alpha5");
            digit6Key = AddKey("6", Key.Digit6, KeyCode.Alpha6, "digit6Key", "digit6", "alpha6");
            digit7Key = AddKey("7", Key.Digit7, KeyCode.Alpha7, "digit7Key", "digit7", "alpha7");
            digit8Key = AddKey("8", Key.Digit8, KeyCode.Alpha8, "digit8Key", "digit8", "alpha8");
            digit9Key = AddKey("9", Key.Digit9, KeyCode.Alpha9, "digit9Key", "digit9", "alpha9");

            leftShiftKey = AddKey("leftShift", Key.LeftShift, KeyCode.LeftShift, "leftShiftKey");
            rightShiftKey = AddKey("rightShift", Key.RightShift, KeyCode.RightShift, "rightShiftKey");
            leftAltKey = AddKey("leftAlt", Key.LeftAlt, KeyCode.LeftAlt, "leftAltKey");
            rightAltKey = AddKey("rightAlt", Key.RightAlt, KeyCode.RightAlt, "rightAltKey");
            leftCtrlKey = AddKey("leftCtrl", Key.LeftCtrl, KeyCode.LeftControl, "leftCtrlKey", "leftControl", "leftControlKey");
            rightCtrlKey = AddKey("rightCtrl", Key.RightCtrl, KeyCode.RightControl, "rightCtrlKey", "rightControl", "rightControlKey");
            leftArrowKey = AddKey("leftArrow", Key.LeftArrow, KeyCode.LeftArrow, "leftArrowKey");
            rightArrowKey = AddKey("rightArrow", Key.RightArrow, KeyCode.RightArrow, "rightArrowKey");
            upArrowKey = AddKey("upArrow", Key.UpArrow, KeyCode.UpArrow, "upArrowKey");
            downArrowKey = AddKey("downArrow", Key.DownArrow, KeyCode.DownArrow, "downArrowKey");
            escapeKey = AddKey("escape", Key.Escape, KeyCode.Escape, "escapeKey", "esc");
            backspaceKey = AddKey("backspace", Key.Backspace, KeyCode.Backspace, "backspaceKey");
            deleteKey = AddKey("delete", Key.Delete, KeyCode.Delete, "deleteKey");

            anyKey = RegisterControl(
                new ButtonControl(
                    "anyKey",
                    "/Keyboard/anyKey",
                    this,
                    IsAnyKeyPressed,
                    WasAnyKeyPressedThisFrame,
                    WasAnyKeyReleasedThisFrame));
        }

        public static Keyboard? current
        {
            get => _customCurrent ?? _defaultInstance;
            internal set => _customCurrent = value;
        }

        internal static Keyboard DefaultInstance => _defaultInstance;

        public KeyControl spaceKey { get; }
        public KeyControl enterKey { get; }
        public KeyControl tabKey { get; }
        public KeyControl backquoteKey { get; }
        public KeyControl quoteKey { get; }
        public KeyControl semicolonKey { get; }
        public KeyControl commaKey { get; }
        public KeyControl periodKey { get; }
        public KeyControl slashKey { get; }
        public KeyControl backslashKey { get; }
        public KeyControl leftBracketKey { get; }
        public KeyControl rightBracketKey { get; }
        public KeyControl minusKey { get; }
        public KeyControl equalsKey { get; }

        public KeyControl aKey { get; }
        public KeyControl bKey { get; }
        public KeyControl cKey { get; }
        public KeyControl dKey { get; }
        public KeyControl eKey { get; }
        public KeyControl fKey { get; }
        public KeyControl gKey { get; }
        public KeyControl hKey { get; }
        public KeyControl iKey { get; }
        public KeyControl jKey { get; }
        public KeyControl kKey { get; }
        public KeyControl lKey { get; }
        public KeyControl mKey { get; }
        public KeyControl nKey { get; }
        public KeyControl oKey { get; }
        public KeyControl pKey { get; }
        public KeyControl qKey { get; }
        public KeyControl rKey { get; }
        public KeyControl sKey { get; }
        public KeyControl tKey { get; }
        public KeyControl uKey { get; }
        public KeyControl vKey { get; }
        public KeyControl wKey { get; }
        public KeyControl xKey { get; }
        public KeyControl yKey { get; }
        public KeyControl zKey { get; }

        public KeyControl digit0Key { get; }
        public KeyControl digit1Key { get; }
        public KeyControl digit2Key { get; }
        public KeyControl digit3Key { get; }
        public KeyControl digit4Key { get; }
        public KeyControl digit5Key { get; }
        public KeyControl digit6Key { get; }
        public KeyControl digit7Key { get; }
        public KeyControl digit8Key { get; }
        public KeyControl digit9Key { get; }

        public KeyControl leftShiftKey { get; }
        public KeyControl rightShiftKey { get; }
        public KeyControl leftAltKey { get; }
        public KeyControl rightAltKey { get; }
        public KeyControl leftCtrlKey { get; }
        public KeyControl rightCtrlKey { get; }
        public KeyControl leftArrowKey { get; }
        public KeyControl rightArrowKey { get; }
        public KeyControl upArrowKey { get; }
        public KeyControl downArrowKey { get; }
        public KeyControl escapeKey { get; }
        public KeyControl backspaceKey { get; }
        public KeyControl deleteKey { get; }

        public ButtonControl anyKey { get; }

        public KeyControl this[Key key]
        {
            get
            {
                if (_keysByEnum.TryGetValue(key, out KeyControl? control))
                {
                    return control;
                }
                throw new ArgumentOutOfRangeException(nameof(key), key, "Unsupported Key value.");
            }
        }

        internal static void ResetDefault()
        {
            _customCurrent = null;
            _defaultInstance = new Keyboard();
        }

        private KeyControl AddKey(string controlName, Key key, KeyCode code, params string[] aliases)
        {
            KeyControl control = RegisterControl(
                new KeyControl(controlName, "/Keyboard/" + controlName, this, key, code),
                aliases);
            _keysByEnum[key] = control;
            if (code != KeyCode.None)
            {
                _keyControls.Add(control);
            }
            return control;
        }

        private bool IsAnyKeyPressed()
        {
            for (int i = 0; i < _keyControls.Count; i++)
            {
                if (_keyControls[i].isPressed)
                {
                    return true;
                }
            }
            return false;
        }

        private bool WasAnyKeyPressedThisFrame()
        {
            for (int i = 0; i < _keyControls.Count; i++)
            {
                if (_keyControls[i].wasPressedThisFrame)
                {
                    return true;
                }
            }
            return false;
        }

        private bool WasAnyKeyReleasedThisFrame()
        {
            for (int i = 0; i < _keyControls.Count; i++)
            {
                if (_keyControls[i].wasReleasedThisFrame)
                {
                    return true;
                }
            }
            return false;
        }
    }
}
