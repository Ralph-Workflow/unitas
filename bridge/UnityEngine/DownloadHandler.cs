using System;
using System.IO;
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
            ReceivePayload(rawData ?? Array.Empty<byte>());
        }

        internal virtual void ReceivePayload(byte[] rawData)
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

    public sealed class DownloadHandlerFile : DownloadHandler
    {
        private readonly string _filePath;
        private readonly bool _append;

        public DownloadHandlerFile(string filePath)
            : this(filePath, false)
        {
        }

        public DownloadHandlerFile(string filePath, bool append)
        {
            if (string.IsNullOrWhiteSpace(filePath))
            {
                throw new ArgumentException("File path cannot be null or empty.", nameof(filePath));
            }

            _filePath = filePath;
            _append = append;
        }

        public bool removeFileOnAbort { get; set; }

        internal override void ReceivePayload(byte[] rawData)
        {
            base.ReceivePayload(rawData);
            string? parentDir = Path.GetDirectoryName(_filePath);
            if (!string.IsNullOrEmpty(parentDir) && !Directory.Exists(parentDir))
            {
                Directory.CreateDirectory(parentDir);
            }

            if (_append && File.Exists(_filePath))
            {
                using FileStream stream = new FileStream(_filePath, FileMode.Append, FileAccess.Write, FileShare.Read);
                stream.Write(rawData, 0, rawData.Length);
            }
            else
            {
                File.WriteAllBytes(_filePath, rawData);
            }
        }
    }
}
