using System;
using System.Text;

namespace UnityEngine.Networking
{
    public class DownloadHandler : IDisposable
    {
        private byte[] _data;
        private bool _disposed;

        public DownloadHandler()
        {
            _data = Array.Empty<byte>();
        }

        internal DownloadHandler(byte[]? initialData)
        {
            _data = initialData ?? Array.Empty<byte>();
        }

        public bool isDone { get; internal set; }

        public string error { get; internal set; } = string.Empty;

        public byte[] data
        {
            get
            {
                EnsureNotDisposed();
                return GetData() ?? Array.Empty<byte>();
            }
        }

        public string text
        {
            get
            {
                EnsureNotDisposed();
                return GetText();
            }
        }

        public static string GetContent(UnityWebRequest www)
        {
            if (www == null)
            {
                throw new ArgumentNullException(nameof(www));
            }

            if (www.downloadHandler == null)
            {
                return string.Empty;
            }

            return www.downloadHandler.text;
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (_disposed)
            {
                return;
            }

            if (disposing)
            {
                _data = Array.Empty<byte>();
            }

            _disposed = true;
        }

        protected virtual byte[] GetData()
        {
            return _data;
        }

        protected virtual string GetText()
        {
            byte[] raw = GetData();
            if (raw == null || raw.Length == 0)
            {
                return string.Empty;
            }

            return Encoding.UTF8.GetString(raw);
        }

        protected virtual float GetProgress()
        {
            return isDone ? 1f : 0f;
        }

        internal void SetData(byte[]? rawData)
        {
            _data = rawData ?? Array.Empty<byte>();
        }

        internal void Complete()
        {
            isDone = true;
        }

        private void EnsureNotDisposed()
        {
            if (_disposed)
            {
                throw new ObjectDisposedException(nameof(DownloadHandler));
            }
        }
    }

    public sealed class DownloadHandlerBuffer : DownloadHandler
    {
        public DownloadHandlerBuffer()
        {
        }

        internal DownloadHandlerBuffer(byte[]? initialData)
            : base(initialData)
        {
        }

        public static new string GetContent(UnityWebRequest www)
        {
            if (www == null)
            {
                throw new ArgumentNullException(nameof(www));
            }

            if (www.downloadHandler == null)
            {
                return string.Empty;
            }

            return www.downloadHandler.text;
        }
    }
}
