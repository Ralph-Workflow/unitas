using System;
using System.Collections;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;
using UnityEngine.Networking;

namespace Unitas.Bridge.Tests
{
    public static class UnityWebRequestTests
    {
        public static void Run()
        {
            RunAll();
        }

        public static void RunAll()
        {
            TestGet200SuccessAndDownloadHandlerTextAndData();
            TestGet404ProtocolErrorWithStatusCode();
            TestUnreachableHostReportsConnectionError();
            TestTimeoutReportsConnectionError();
            TestCrossOriginDenialReportsPlatformLimited();
            TestCoroutineYieldAndAsyncAwaitAndCredentialHeaderRedaction();
        }

        private static void TestGet200SuccessAndDownloadHandlerTextAndData()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            using LoopbackHttpServer server = new LoopbackHttpServer((string path, Stream stream) =>
            {
                WriteHttpResponse(stream, 200, "OK", "text/plain; charset=utf-8", "hello-unitas:" + path);
            });

            string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/items/42";
            using UnityWebRequest request = UnityWebRequest.Get(url);
            request.timeout = 5;
            UnityWebRequestAsyncOperation op = request.SendWebRequest();

            Check.Equal("GET 200 operation isDone", true, op.isDone);
            Check.Equal("GET 200 request isDone", true, request.isDone);
            Check.Equal("GET 200 result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
            Check.Equal("GET 200 responseCode is 200", 200, (int)request.responseCode);
            Check.That("GET 200 error is null", request.error == null);
            Check.That("GET 200 downloadHandler is present", request.downloadHandler != null);
            Check.Equal("GET 200 downloadHandler.text matches body", "hello-unitas:/items/42", request.downloadHandler!.text);
            Check.Equal("GET 200 DownloadHandlerBuffer.GetContent matches body", "hello-unitas:/items/42", DownloadHandlerBuffer.GetContent(request));
            Check.Equal("GET 200 downloadHandler.data byte length", Encoding.UTF8.GetByteCount("hello-unitas:/items/42"), request.downloadHandler.data.Length);
        }

        private static void TestGet404ProtocolErrorWithStatusCode()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            using LoopbackHttpServer server = new LoopbackHttpServer((string path, Stream stream) =>
            {
                _ = path;
                WriteHttpResponse(stream, 404, "Not Found", "text/plain; charset=utf-8", "missing-resource");
            });

            string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/missing";
            using UnityWebRequest request = UnityWebRequest.Get(url);
            request.timeout = 5;
            request.SendWebRequest();

            Check.Equal("GET 404 request isDone", true, request.isDone);
            Check.Equal("GET 404 result is ProtocolError", (int)UnityWebRequest.Result.ProtocolError, (int)request.result);
            Check.Equal("GET 404 responseCode is 404", 404, (int)request.responseCode);
            Check.That("GET 404 error contains 404", request.error != null && request.error.Contains("404", StringComparison.Ordinal));
            Check.Equal("GET 404 error body still accessible", "missing-resource", request.downloadHandler!.text);
        }

        private static void TestUnreachableHostReportsConnectionError()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            int closedPort = ReserveAndCloseEphemeralPort();
            string url = "http://127.0.0.1:" + closedPort.ToString(CultureInfo.InvariantCulture) + "/unreachable";

            using UnityWebRequest request = UnityWebRequest.Get(url);
            request.timeout = 2;
            request.SendWebRequest();

            Check.Equal("Unreachable host request isDone", true, request.isDone);
            Check.Equal("Unreachable host result is ConnectionError", (int)UnityWebRequest.Result.ConnectionError, (int)request.result);
            Check.Equal("Unreachable host responseCode is 0", 0, (int)request.responseCode);
            Check.That("Unreachable host error is populated", !string.IsNullOrEmpty(request.error));
        }

        private static void TestTimeoutReportsConnectionError()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            using ManualResetEventSlim releaseServer = new ManualResetEventSlim(false);
            using LoopbackHttpServer server = new LoopbackHttpServer((string path, Stream stream) =>
            {
                _ = path;
                _ = stream;
                releaseServer.Wait(500);
            });

            try
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/slow";
                using UnityWebRequest request = UnityWebRequest.Get(url);
                request.TimeoutMilliseconds = 15;
                request.SendWebRequest();

                Check.Equal("Timeout request isDone", true, request.isDone);
                Check.Equal("Timeout result is ConnectionError", (int)UnityWebRequest.Result.ConnectionError, (int)request.result);
                Check.Equal("Timeout responseCode is 0", 0, (int)request.responseCode);
                Check.That(
                    "Timeout error mentions timeout",
                    request.error != null && request.error.Contains("timeout", StringComparison.OrdinalIgnoreCase));
            }
            finally
            {
                releaseServer.Set();
            }
        }

        private static void TestCrossOriginDenialReportsPlatformLimited()
        {
            UnityWebRequest.ClearDiagnostics();
            LogRecorder.Clear();
            UnityWebRequest.CrossOriginDenialRule = (Uri targetUri) =>
            {
                if (string.Equals(targetUri.Host, "cross-origin.example.test", StringComparison.OrdinalIgnoreCase))
                {
                    return "Blocked by browser CORS policy: missing Access-Control-Allow-Origin header";
                }

                return null;
            };

            try
            {
                using UnityWebRequest request = UnityWebRequest.Get("https://cross-origin.example.test/api/state");
                request.SendWebRequest();

                Check.Equal("CORS denial request isDone", true, request.isDone);
                Check.Equal("CORS denial result is ConnectionError", (int)UnityWebRequest.Result.ConnectionError, (int)request.result);
                Check.Equal("CORS denial IsPlatformLimited is true", true, request.IsPlatformLimited);
                Check.That(
                    "CORS denial error contains Platform-limited",
                    request.error != null && request.error.Contains("Platform-limited", StringComparison.Ordinal));
                Check.That(
                    "CORS denial logged Platform-limited error",
                    LogRecorder.Search("Platform-limited").Count >= 1);
                Check.Equal("CORS denial recorded in diagnostics", 1, UnityWebRequest.Diagnostics.Count);
                Check.Equal("CORS diagnostic IsPlatformLimited", true, UnityWebRequest.Diagnostics[0].IsPlatformLimited);
            }
            finally
            {
                UnityWebRequest.CrossOriginDenialRule = null;
            }
        }

        private static void TestCoroutineYieldAndAsyncAwaitAndCredentialHeaderRedaction()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            using LoopbackHttpServer server = new LoopbackHttpServer((string path, Stream stream) =>
            {
                WriteHttpResponse(stream, 200, "OK", "application/json", "{\"ok\":true,\"path\":\"" + path + "\"}");
            });

            string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/ping";
            GameObject hostObject = new GameObject("WebRequestHost");
            WebRequestProbeBehaviour probe = hostObject.AddComponent<WebRequestProbeBehaviour>();
            probe.TargetUrl = url;
            probe.StartCoroutine(probe.FetchRoutine());
            CoroutineScheduler.PumpFrame();

            Check.Equal("Coroutine yield completed after PumpFrame", true, probe.Completed);
            Check.Equal("Coroutine yield observed 200", 200, probe.StatusCode);
            Check.Equal("Coroutine yield observed payload", "{\"ok\":true,\"path\":\"/ping\"}", probe.ResponseBody);

            UnityWebRequest awaitedResult = FetchAsync(url).GetAwaiter().GetResult();
            Check.Equal("Async await observed Success", (int)UnityWebRequest.Result.Success, (int)awaitedResult.result);
            awaitedResult.Dispose();

            Check.That("Diagnostics recorded requests", UnityWebRequest.Diagnostics.Count >= 1);
            UnityWebRequestDiagnostic firstDiagnostic = UnityWebRequest.Diagnostics[0];
            Check.Equal(
                "Authorization header redacted in diagnostics",
                "[REDACTED]",
                firstDiagnostic.RequestHeaders["Authorization"]);
            Check.Equal(
                "Non-credential header preserved in diagnostics",
                "unitas-test",
                firstDiagnostic.RequestHeaders["X-Client"]);

            UnityEngine.Object.Destroy(hostObject);
        }

        private static async Task<UnityWebRequest> FetchAsync(string url)
        {
            UnityWebRequest request = UnityWebRequest.Get(url);
            return await request.SendWebRequest();
        }

        private static int ReserveAndCloseEphemeralPort()
        {
            using Socket socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
            socket.Bind(new IPEndPoint(IPAddress.Loopback, 0));
            return ((IPEndPoint)socket.LocalEndPoint!).Port;
        }

        private static void WriteHttpResponse(
            Stream stream,
            int statusCode,
            string reasonPhrase,
            string contentType,
            string body)
        {
            byte[] payload = Encoding.UTF8.GetBytes(body);
            string header =
                "HTTP/1.1 " + statusCode.ToString(CultureInfo.InvariantCulture) + " " + reasonPhrase + "\r\n" +
                "Content-Type: " + contentType + "\r\n" +
                "Content-Length: " + payload.Length.ToString(CultureInfo.InvariantCulture) + "\r\n" +
                "Connection: close\r\n\r\n";
            byte[] headerBytes = Encoding.ASCII.GetBytes(header);
            stream.Write(headerBytes, 0, headerBytes.Length);
            stream.Write(payload, 0, payload.Length);
            stream.Flush();
        }

        private sealed class WebRequestProbeBehaviour : MonoBehaviour
        {
            public string TargetUrl { get; set; } = string.Empty;

            public bool Completed { get; private set; }

            public int StatusCode { get; private set; }

            public string ResponseBody { get; private set; } = string.Empty;

            public IEnumerator FetchRoutine()
            {
                using UnityWebRequest request = UnityWebRequest.Get(TargetUrl);
                request.SetRequestHeader("Authorization", "Bearer secret-token");
                request.SetRequestHeader("X-Client", "unitas-test");
                yield return request.SendWebRequest();
                Completed = request.isDone;
                StatusCode = (int)request.responseCode;
                ResponseBody = request.downloadHandler != null ? request.downloadHandler.text : string.Empty;
            }
        }

        private sealed class LoopbackHttpServer : IDisposable
        {
            private readonly Socket _listenSocket;
            private readonly Action<string, Stream> _handler;
            private readonly Thread _acceptThread;
            private volatile bool _stopped;

            public LoopbackHttpServer(Action<string, Stream> handler)
            {
                _handler = handler ?? throw new ArgumentNullException(nameof(handler));
                _listenSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
                _listenSocket.Bind(new IPEndPoint(IPAddress.Loopback, 0));
                _listenSocket.Listen(8);
                Port = ((IPEndPoint)_listenSocket.LocalEndPoint!).Port;
                _acceptThread = new Thread(AcceptLoop);
                _acceptThread.IsBackground = true;
                _acceptThread.Start();
            }

            public int Port { get; }

            public void Dispose()
            {
                _stopped = true;
                _listenSocket.Dispose();
            }

            private void AcceptLoop()
            {
                while (!_stopped)
                {
                    try
                    {
                        using Socket clientSocket = _listenSocket.Accept();
                        using NetworkStream stream = new NetworkStream(clientSocket, false);
                        string requestLine = ReadRequestLine(stream);
                        string path = ParseRequestPath(requestLine);
                        DrainHeaders(stream);
                        _handler(path, stream);
                    }
                    catch (SocketException)
                    {
                        if (_stopped)
                        {
                            return;
                        }
                    }
                    catch (ObjectDisposedException)
                    {
                        return;
                    }
                    catch (IOException)
                    {
                    }
                }
            }

            private static string ReadRequestLine(Stream stream)
            {
                StringBuilder builder = new StringBuilder();
                int previous = -1;
                while (true)
                {
                    int current = stream.ReadByte();
                    if (current < 0)
                    {
                        break;
                    }

                    if (previous == '\r' && current == '\n')
                    {
                        if (builder.Length > 0 && builder[builder.Length - 1] == '\r')
                        {
                            builder.Length -= 1;
                        }

                        break;
                    }

                    builder.Append((char)current);
                    previous = current;
                }

                return builder.ToString();
            }

            private static void DrainHeaders(Stream stream)
            {
                int matchState = 0;
                while (true)
                {
                    int b = stream.ReadByte();
                    if (b < 0)
                    {
                        return;
                    }

                    if ((matchState == 0 || matchState == 2) && b == '\r')
                    {
                        matchState++;
                    }
                    else if ((matchState == 1 || matchState == 3) && b == '\n')
                    {
                        matchState++;
                        if (matchState == 4)
                        {
                            return;
                        }
                    }
                    else
                    {
                        matchState = 0;
                    }
                }
            }

            private static string ParseRequestPath(string requestLine)
            {
                if (string.IsNullOrEmpty(requestLine))
                {
                    return "/";
                }

                int firstSpace = requestLine.IndexOf(' ', StringComparison.Ordinal);
                if (firstSpace < 0)
                {
                    return "/";
                }

                int secondSpace = requestLine.IndexOf(' ', firstSpace + 1);
                if (secondSpace < 0)
                {
                    return requestLine.Substring(firstSpace + 1);
                }

                return requestLine.Substring(firstSpace + 1, secondSpace - firstSpace - 1);
            }
        }
    }
}
