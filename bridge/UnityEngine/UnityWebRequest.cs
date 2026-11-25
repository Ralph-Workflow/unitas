using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Net.Http;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace UnityEngine.Networking
{
    public sealed class UnityWebRequestAsyncOperation : AsyncOperation
    {
        private readonly Task<UnityWebRequest> _completionTask;

        internal UnityWebRequestAsyncOperation(UnityWebRequest webRequest, Task<UnityWebRequest> completionTask)
        {
            this.webRequest = webRequest ?? throw new ArgumentNullException(nameof(webRequest));
            _completionTask = completionTask ?? throw new ArgumentNullException(nameof(completionTask));
        }

        public UnityWebRequest webRequest { get; }

        public TaskAwaiter<UnityWebRequest> GetAwaiter()
        {
            return _completionTask.GetAwaiter();
        }
    }

    public sealed class UnityWebRequestDiagnostic
    {
        public UnityWebRequestDiagnostic(
            string url,
            string method,
            long responseCode,
            UnityWebRequest.Result result,
            string error,
            double durationMilliseconds,
            bool isPlatformLimited,
            string platformLimitReason,
            IReadOnlyDictionary<string, string> requestHeaders)
        {
            Url = url ?? string.Empty;
            Method = method ?? string.Empty;
            ResponseCode = responseCode;
            Result = result;
            Error = error ?? string.Empty;
            DurationMilliseconds = durationMilliseconds;
            IsPlatformLimited = isPlatformLimited;
            PlatformLimitReason = platformLimitReason ?? string.Empty;
            RequestHeaders = requestHeaders;
        }

        public string Url { get; }

        public string Method { get; }

        public long ResponseCode { get; }

        public UnityWebRequest.Result Result { get; }

        public string Error { get; }

        public double DurationMilliseconds { get; }

        public bool IsPlatformLimited { get; }

        public string PlatformLimitReason { get; }

        public IReadOnlyDictionary<string, string> RequestHeaders { get; }
    }

    public sealed class UnityWebRequest : IDisposable
    {
        public const string kHttpVerbGET = "GET";
        public const string kHttpVerbHEAD = "HEAD";
        public const string kHttpVerbPOST = "POST";
        public const string kHttpVerbPUT = "PUT";
        public const string kHttpVerbCREATE = "CREATE";
        public const string kHttpVerbDELETE = "DELETE";

        public enum Result
        {
            InProgress = 0,
            Success = 1,
            ConnectionError = 2,
            ProtocolError = 3,
            DataProcessingError = 4,
        }

        private static readonly HttpClient SharedHttpClient = CreateSharedHttpClient();
        private static readonly List<UnityWebRequestDiagnostic> RecordedDiagnostics = new List<UnityWebRequestDiagnostic>();
        private static Func<Uri, string?>? _crossOriginDenialRule;

        private readonly Dictionary<string, string> _requestHeaders = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        private readonly Dictionary<string, string> _responseHeaders = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        private CancellationTokenSource? _activeCts;
        private string _url;
        private string _method;
        private int _timeoutSeconds;
        private int _timeoutMillisecondsOverride;
        private int _redirectLimit = 32;
        private bool _disposed;
        private bool _sent;
        private float _downloadProgress;
        private ulong _downloadedBytes;

        public UnityWebRequest()
            : this(string.Empty, kHttpVerbGET, null)
        {
        }

        public UnityWebRequest(string url)
            : this(url, kHttpVerbGET, null)
        {
        }

        public UnityWebRequest(string url, string method)
            : this(url, method, null)
        {
        }

        public UnityWebRequest(string url, string method, DownloadHandler? downloadHandler)
        {
            _url = url ?? string.Empty;
            _method = string.IsNullOrWhiteSpace(method) ? kHttpVerbGET : method;
            this.downloadHandler = downloadHandler;
            disposeDownloadHandlerOnDispose = true;
            result = Result.InProgress;
            error = null;
        }

        public static IReadOnlyList<UnityWebRequestDiagnostic> Diagnostics => RecordedDiagnostics;

        public static Func<Uri, string?>? CrossOriginDenialRule
        {
            get => _crossOriginDenialRule;
            set => _crossOriginDenialRule = value;
        }

        public string url
        {
            get => _url;
            set
            {
                EnsureNotSent();
                _url = value ?? string.Empty;
                uri = TryCreateUri(_url);
            }
        }

        public Uri? uri { get; private set; }

        public string method
        {
            get => _method;
            set
            {
                EnsureNotSent();
                if (string.IsNullOrWhiteSpace(value))
                {
                    throw new ArgumentException("Method cannot be null or empty.", nameof(value));
                }

                _method = value;
            }
        }

        public int timeout
        {
            get => _timeoutSeconds;
            set
            {
                EnsureNotSent();
                if (value < 0)
                {
                    throw new ArgumentOutOfRangeException(nameof(value), "Timeout cannot be negative.");
                }

                _timeoutSeconds = value;
            }
        }

        public int TimeoutMilliseconds
        {
            get => _timeoutMillisecondsOverride > 0 ? _timeoutMillisecondsOverride : _timeoutSeconds * 1000;
            set
            {
                EnsureNotSent();
                if (value < 0)
                {
                    throw new ArgumentOutOfRangeException(nameof(value), "TimeoutMilliseconds cannot be negative.");
                }

                _timeoutMillisecondsOverride = value;
            }
        }

        public int redirectLimit
        {
            get => _redirectLimit;
            set
            {
                EnsureNotSent();
                _redirectLimit = value;
            }
        }

        public DownloadHandler? downloadHandler { get; set; }

        public bool disposeDownloadHandlerOnDispose { get; set; }

        public Result result { get; private set; }

        public long responseCode { get; private set; }

        public string? error { get; private set; }

        public bool isDone { get; private set; }

        public bool isModifiable => !_sent && !_disposed;

        public bool IsPlatformLimited { get; private set; }

        public string PlatformLimitReason { get; private set; } = string.Empty;

        public float downloadProgress => isDone ? 1f : _downloadProgress;

        public ulong downloadedBytes => _downloadedBytes;

        public static UnityWebRequest Get(string uri)
        {
            return new UnityWebRequest(uri, kHttpVerbGET, new DownloadHandlerBuffer());
        }

        public static UnityWebRequest Head(string uri)
        {
            return new UnityWebRequest(uri, kHttpVerbHEAD, null);
        }

        public static UnityWebRequest Delete(string uri)
        {
            return new UnityWebRequest(uri, kHttpVerbDELETE, null);
        }

        public static void ClearDiagnostics()
        {
            RecordedDiagnostics.Clear();
        }

        public static void ReportCrossOriginDenial(UnityWebRequest request, string? reason = null)
        {
            if (request == null)
            {
                throw new ArgumentNullException(nameof(request));
            }

            string effectiveReason = string.IsNullOrWhiteSpace(reason)
                ? "Cross-origin request blocked by browser CORS policy"
                : reason!;
            request.ApplyPlatformLimitedFailure(effectiveReason, 0);
        }

        public void SetRequestHeader(string name, string value)
        {
            EnsureNotDisposed();
            EnsureNotSent();
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("Header name cannot be null or empty.", nameof(name));
            }

            if (value == null)
            {
                throw new ArgumentNullException(nameof(value));
            }

            _requestHeaders[name] = value;
        }

        public string? GetRequestHeader(string name)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                return null;
            }

            return _requestHeaders.TryGetValue(name, out string? value) ? value : null;
        }

        public string? GetResponseHeader(string name)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                return null;
            }

            return _responseHeaders.TryGetValue(name, out string? value) ? value : null;
        }

        public Dictionary<string, string>? GetResponseHeaders()
        {
            if (!isDone)
            {
                return null;
            }

            return new Dictionary<string, string>(_responseHeaders, StringComparer.OrdinalIgnoreCase);
        }

        public UnityWebRequestAsyncOperation SendWebRequest()
        {
            EnsureNotDisposed();
            EnsureNotSent();
            _sent = true;
            uri = TryCreateUri(_url);

            TaskCompletionSource<UnityWebRequest> tcs = new TaskCompletionSource<UnityWebRequest>(TaskCreationOptions.RunContinuationsAsynchronously);
            UnityWebRequestAsyncOperation operation = new UnityWebRequestAsyncOperation(this, tcs.Task);
            ExecuteRequestSync(operation, tcs);
            return operation;
        }

        public void Abort()
        {
            if (_disposed || isDone)
            {
                return;
            }

            CancellationTokenSource? cts = _activeCts;
            if (cts != null)
            {
                try
                {
                    cts.Cancel();
                }
                catch (ObjectDisposedException)
                {
                }
            }

            result = Result.ConnectionError;
            error = "Request aborted";
            isDone = true;
        }

        public void Dispose()
        {
            if (_disposed)
            {
                return;
            }

            Abort();
            if (disposeDownloadHandlerOnDispose && downloadHandler != null)
            {
                downloadHandler.Dispose();
                downloadHandler = null;
            }

            CancellationTokenSource? cts = _activeCts;
            _activeCts = null;
            cts?.Dispose();
            _disposed = true;
        }

        private static HttpClient CreateSharedHttpClient()
        {
            HttpClient client = new HttpClient();
            client.Timeout = Timeout.InfiniteTimeSpan;
            return client;
        }

        private static Uri? TryCreateUri(string rawUrl)
        {
            if (Uri.TryCreate(rawUrl, UriKind.Absolute, out Uri? parsed))
            {
                return parsed;
            }

            return null;
        }

        private static bool IsCredentialHeader(string headerName)
        {
            return string.Equals(headerName, "Authorization", StringComparison.OrdinalIgnoreCase)
                || string.Equals(headerName, "Proxy-Authorization", StringComparison.OrdinalIgnoreCase)
                || string.Equals(headerName, "Cookie", StringComparison.OrdinalIgnoreCase)
                || string.Equals(headerName, "Set-Cookie", StringComparison.OrdinalIgnoreCase)
                || string.Equals(headerName, "X-Api-Key", StringComparison.OrdinalIgnoreCase);
        }

        private void ExecuteRequestSync(UnityWebRequestAsyncOperation operation, TaskCompletionSource<UnityWebRequest> tcs)
        {
            Stopwatch stopwatch = Stopwatch.StartNew();
            if (uri == null || (!string.Equals(uri.Scheme, "http", StringComparison.OrdinalIgnoreCase)
                && !string.Equals(uri.Scheme, "https", StringComparison.OrdinalIgnoreCase)))
            {
                stopwatch.Stop();
                result = Result.ConnectionError;
                responseCode = 0;
                error = "Unsupported or invalid URL: " + _url;
                FinalizeOperation(operation, tcs, stopwatch.Elapsed.TotalMilliseconds);
                return;
            }

            Func<Uri, string?>? corsRule = _crossOriginDenialRule;
            if (corsRule != null)
            {
                string? denialReason = corsRule(uri);
                if (!string.IsNullOrEmpty(denialReason))
                {
                    stopwatch.Stop();
                    ApplyPlatformLimitedFailure(denialReason!, stopwatch.Elapsed.TotalMilliseconds);
                    operation.MarkComplete();
                    tcs.TrySetResult(this);
                    return;
                }
            }

            int effectiveTimeoutMs = TimeoutMilliseconds;
            using CancellationTokenSource cts = new CancellationTokenSource();
            _activeCts = cts;
            if (effectiveTimeoutMs > 0)
            {
                cts.CancelAfter(effectiveTimeoutMs);
            }

            try
            {
                using HttpRequestMessage requestMessage = new HttpRequestMessage(new HttpMethod(_method), uri);
                foreach (KeyValuePair<string, string> header in _requestHeaders)
                {
                    requestMessage.Headers.TryAddWithoutValidation(header.Key, header.Value);
                }

                using HttpResponseMessage responseMessage = SharedHttpClient
                    .SendAsync(requestMessage, HttpCompletionOption.ResponseContentRead, cts.Token)
                    .GetAwaiter()
                    .GetResult();

                responseCode = (long)responseMessage.StatusCode;
                CopyResponseHeaders(responseMessage);

                byte[] bodyBytes = responseMessage.Content != null
                    ? responseMessage.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult()
                    : Array.Empty<byte>();

                _downloadedBytes = (ulong)bodyBytes.Length;
                _downloadProgress = 1f;
                if (downloadHandler != null)
                {
                    downloadHandler.SetData(bodyBytes);
                    downloadHandler.Complete();
                }

                if ((int)responseMessage.StatusCode >= 200 && (int)responseMessage.StatusCode < 300)
                {
                    result = Result.Success;
                    error = null;
                }
                else
                {
                    result = Result.ProtocolError;
                    string reasonPhrase = string.IsNullOrWhiteSpace(responseMessage.ReasonPhrase)
                        ? responseMessage.StatusCode.ToString()
                        : responseMessage.ReasonPhrase!;
                    error = "HTTP/1.1 " + responseCode.ToString(CultureInfo.InvariantCulture) + " " + reasonPhrase;
                }
            }
            catch (OperationCanceledException)
            {
                result = Result.ConnectionError;
                responseCode = 0;
                error = effectiveTimeoutMs > 0 ? "Request timeout" : "Request aborted";
            }
            catch (HttpRequestException ex)
            {
                result = Result.ConnectionError;
                responseCode = 0;
                error = string.IsNullOrWhiteSpace(ex.Message) ? "Cannot connect to destination host" : ex.Message;
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                result = Result.ConnectionError;
                responseCode = 0;
                error = ex.Message;
            }
            finally
            {
                _activeCts = null;
                stopwatch.Stop();
            }

            FinalizeOperation(operation, tcs, stopwatch.Elapsed.TotalMilliseconds);
        }

        private void ApplyPlatformLimitedFailure(string reason, double durationMs)
        {
            IsPlatformLimited = true;
            PlatformLimitReason = reason;
            result = Result.ConnectionError;
            responseCode = 0;
            error = "Platform-limited: " + _method + " " + _url + " (" + reason + ")";
            isDone = true;
            Debug.LogError(error);
            RecordDiagnostic(durationMs);
        }

        private void FinalizeOperation(
            UnityWebRequestAsyncOperation operation,
            TaskCompletionSource<UnityWebRequest> tcs,
            double durationMs)
        {
            isDone = true;
            RecordDiagnostic(durationMs);
            operation.MarkComplete();
            tcs.TrySetResult(this);
        }

        private void CopyResponseHeaders(HttpResponseMessage responseMessage)
        {
            _responseHeaders.Clear();
            foreach (KeyValuePair<string, IEnumerable<string>> header in responseMessage.Headers)
            {
                _responseHeaders[header.Key] = JoinHeaderValues(header.Value);
            }

            if (responseMessage.Content != null)
            {
                foreach (KeyValuePair<string, IEnumerable<string>> header in responseMessage.Content.Headers)
                {
                    _responseHeaders[header.Key] = JoinHeaderValues(header.Value);
                }
            }
        }

        private static string JoinHeaderValues(IEnumerable<string> values)
        {
            StringBuilder builder = new StringBuilder();
            bool first = true;
            foreach (string val in values)
            {
                if (!first)
                {
                    builder.Append(", ");
                }

                builder.Append(val);
                first = false;
            }

            return builder.ToString();
        }

        private void RecordDiagnostic(double durationMs)
        {
            Dictionary<string, string> redactedHeaders = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (KeyValuePair<string, string> pair in _requestHeaders)
            {
                redactedHeaders[pair.Key] = IsCredentialHeader(pair.Key) ? "[REDACTED]" : pair.Value;
            }

            UnityWebRequestDiagnostic diagnostic = new UnityWebRequestDiagnostic(
                _url,
                _method,
                responseCode,
                result,
                error ?? string.Empty,
                durationMs,
                IsPlatformLimited,
                PlatformLimitReason,
                redactedHeaders);
            RecordedDiagnostics.Add(diagnostic);
        }

        private void EnsureNotDisposed()
        {
            if (_disposed)
            {
                throw new ObjectDisposedException(nameof(UnityWebRequest));
            }
        }

        private void EnsureNotSent()
        {
            if (_sent)
            {
                throw new InvalidOperationException("UnityWebRequest has already been sent and is not modifiable.");
            }
        }
    }
}
