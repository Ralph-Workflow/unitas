using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace UnityEngine
{
    public sealed class WWWForm
    {
        private readonly List<KeyValuePair<string, string>> _fields = new List<KeyValuePair<string, string>>();
        private readonly Dictionary<string, string> _headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            { "Content-Type", "application/x-www-form-urlencoded" },
        };

        public Dictionary<string, string> headers => new Dictionary<string, string>(_headers, StringComparer.OrdinalIgnoreCase);

        public byte[] data => Encoding.UTF8.GetBytes(BuildFormEncodedString());

        public void AddField(string fieldName, string value)
        {
            AddField(fieldName, value, Encoding.UTF8);
        }

        public void AddField(string fieldName, string value, Encoding e)
        {
            if (string.IsNullOrEmpty(fieldName))
            {
                throw new ArgumentException("Field name cannot be null or empty.", nameof(fieldName));
            }

            _ = e ?? throw new ArgumentNullException(nameof(e));
            _fields.Add(new KeyValuePair<string, string>(fieldName, value ?? string.Empty));
        }

        public void AddField(string fieldName, int i)
        {
            AddField(fieldName, i.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        internal string BuildFormEncodedString()
        {
            StringBuilder builder = new StringBuilder();
            for (int index = 0; index < _fields.Count; index++)
            {
                if (index > 0)
                {
                    builder.Append('&');
                }

                KeyValuePair<string, string> pair = _fields[index];
                builder.Append(Uri.EscapeDataString(pair.Key));
                builder.Append('=');
                builder.Append(Uri.EscapeDataString(pair.Value));
            }

            return builder.ToString();
        }
    }
}

namespace UnityEngine.Networking
{
    public interface IMultipartFormSection
    {
        string? sectionName { get; }

        byte[] sectionData { get; }

        string? fileName { get; }

        string? contentType { get; }
    }

    public sealed class MultipartFormDataSection : IMultipartFormSection
    {
        private readonly byte[] _data;

        public MultipartFormDataSection(string name, string data)
            : this(name, data, Encoding.UTF8, "text/plain")
        {
        }

        public MultipartFormDataSection(string name, string data, string contentType)
            : this(name, data, Encoding.UTF8, contentType)
        {
        }

        public MultipartFormDataSection(string name, string data, Encoding dataEncoding, string contentType)
        {
            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentException("Section name cannot be null or empty.", nameof(name));
            }

            if (dataEncoding == null)
            {
                throw new ArgumentNullException(nameof(dataEncoding));
            }

            sectionName = name;
            _data = dataEncoding.GetBytes(data ?? string.Empty);
            fileName = null;
            this.contentType = string.IsNullOrWhiteSpace(contentType)
                ? "text/plain; charset=" + dataEncoding.WebName
                : contentType;
        }

        public MultipartFormDataSection(string name, byte[] data, string contentType)
        {
            if (data == null || data.Length == 0)
            {
                throw new ArgumentException("Section data cannot be null or empty.", nameof(data));
            }

            sectionName = name;
            _data = (byte[])data.Clone();
            fileName = null;
            this.contentType = string.IsNullOrWhiteSpace(contentType) ? "application/octet-stream" : contentType;
        }

        public MultipartFormDataSection(byte[] data)
            : this(string.Empty, data, "application/octet-stream")
        {
        }

        public MultipartFormDataSection(string data)
            : this("field", data)
        {
        }

        public string? sectionName { get; }

        public byte[] sectionData => _data;

        public string? fileName { get; }

        public string? contentType { get; }
    }

    public sealed class MultipartFormFileSection : IMultipartFormSection
    {
        private readonly byte[] _data;

        public MultipartFormFileSection(string name, byte[] data, string fileName, string contentType)
        {
            if (data == null || data.Length == 0)
            {
                throw new ArgumentException("File section data cannot be null or empty.", nameof(data));
            }

            sectionName = string.IsNullOrWhiteSpace(name) ? "file" : name;
            _data = (byte[])data.Clone();
            this.fileName = string.IsNullOrWhiteSpace(fileName) ? "file.dat" : fileName;
            this.contentType = string.IsNullOrWhiteSpace(contentType) ? "application/octet-stream" : contentType;
        }

        public MultipartFormFileSection(byte[] data)
            : this("file", data, "file.dat", "application/octet-stream")
        {
        }

        public MultipartFormFileSection(string fileName, byte[] data)
            : this("file", data, fileName, "application/octet-stream")
        {
        }

        public MultipartFormFileSection(string name, string data, Encoding dataEncoding, string fileName)
        {
            if (dataEncoding == null)
            {
                throw new ArgumentNullException(nameof(dataEncoding));
            }

            byte[] encoded = dataEncoding.GetBytes(data ?? string.Empty);
            if (encoded.Length == 0)
            {
                throw new ArgumentException("File section data cannot be empty.", nameof(data));
            }

            sectionName = string.IsNullOrWhiteSpace(name) ? "file" : name;
            _data = encoded;
            this.fileName = string.IsNullOrWhiteSpace(fileName) ? "file.txt" : fileName;
            contentType = "text/plain; charset=" + dataEncoding.WebName;
        }

        public string? sectionName { get; }

        public byte[] sectionData => _data;

        public string? fileName { get; }

        public string? contentType { get; }
    }

    public class UploadHandler : IDisposable
    {
        private byte[] _data;
        private string _contentType;
        private float _progress;
        private bool _disposed;

        public UploadHandler()
            : this(Array.Empty<byte>(), "application/octet-stream")
        {
        }

        internal UploadHandler(byte[]? initialData, string? initialContentType)
        {
            _data = initialData ?? Array.Empty<byte>();
            _contentType = string.IsNullOrWhiteSpace(initialContentType)
                ? "application/octet-stream"
                : initialContentType!;
            _progress = 0f;
        }

        public byte[] data
        {
            get
            {
                EnsureNotDisposed();
                return GetData() ?? Array.Empty<byte>();
            }
        }

        public string contentType
        {
            get
            {
                EnsureNotDisposed();
                return GetContentType();
            }
            set
            {
                EnsureNotDisposed();
                SetContentType(value);
            }
        }

        public float progress
        {
            get
            {
                EnsureNotDisposed();
                return GetProgress();
            }
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

        protected virtual string GetContentType()
        {
            return _contentType;
        }

        protected virtual void SetContentType(string newContentType)
        {
            _contentType = string.IsNullOrWhiteSpace(newContentType)
                ? "application/octet-stream"
                : newContentType;
        }

        protected virtual float GetProgress()
        {
            return _progress;
        }

        internal void MarkSent()
        {
            _progress = 1f;
        }

        protected void EnsureNotDisposed()
        {
            if (_disposed)
            {
                throw new ObjectDisposedException(nameof(UploadHandler));
            }
        }
    }

    public sealed class UploadHandlerRaw : UploadHandler
    {
        public UploadHandlerRaw(byte[]? data)
            : base(CloneBuffer(data), "application/octet-stream")
        {
        }

        public UploadHandlerRaw(string text)
            : this(text, "text/plain; charset=utf-8")
        {
        }

        public UploadHandlerRaw(string text, string contentType)
            : base(Encoding.UTF8.GetBytes(text ?? string.Empty), contentType)
        {
        }

        private static byte[] CloneBuffer(byte[]? source)
        {
            if (source == null || source.Length == 0)
            {
                return Array.Empty<byte>();
            }

            byte[] copy = new byte[source.Length];
            Buffer.BlockCopy(source, 0, copy, 0, source.Length);
            return copy;
        }
    }

    public sealed class UploadHandlerForm : UploadHandler
    {
        public UploadHandlerForm(IDictionary<string, string> formFields)
            : base(EncodeFormFields(formFields), "application/x-www-form-urlencoded")
        {
        }

        public UploadHandlerForm(WWWForm form)
            : base(
                form != null ? form.data : throw new ArgumentNullException(nameof(form)),
                ResolveFormContentType(form))
        {
        }

        internal static byte[] EncodeFormFields(IDictionary<string, string> formFields)
        {
            if (formFields == null)
            {
                throw new ArgumentNullException(nameof(formFields));
            }

            StringBuilder builder = new StringBuilder();
            bool first = true;
            foreach (KeyValuePair<string, string> pair in formFields)
            {
                if (!first)
                {
                    builder.Append('&');
                }

                builder.Append(Uri.EscapeDataString(pair.Key ?? string.Empty));
                builder.Append('=');
                builder.Append(Uri.EscapeDataString(pair.Value ?? string.Empty));
                first = false;
            }

            return Encoding.UTF8.GetBytes(builder.ToString());
        }

        private static string ResolveFormContentType(WWWForm form)
        {
            if (form.headers.TryGetValue("Content-Type", out string? headerValue)
                && !string.IsNullOrWhiteSpace(headerValue))
            {
                return headerValue!;
            }

            return "application/x-www-form-urlencoded";
        }
    }

    public sealed class UploadHandlerFile : UploadHandler
    {
        private readonly string _filePath;

        public UploadHandlerFile(string filePath)
            : base(ReadFileBytes(filePath), "application/octet-stream")
        {
            _filePath = filePath;
        }

        public string filePath
        {
            get
            {
                EnsureNotDisposed();
                return _filePath;
            }
        }

        private static byte[] ReadFileBytes(string filePath)
        {
            if (string.IsNullOrWhiteSpace(filePath))
            {
                throw new ArgumentException("File path cannot be null or empty.", nameof(filePath));
            }

            return File.ReadAllBytes(filePath);
        }
    }
}
