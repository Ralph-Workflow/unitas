using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using UnityEngine;
using UnityEngine.Networking;

namespace Unitas.Bridge.Tests
{
    public static class UnityWebRequestMethodTests
    {
        public static void Run()
        {
            RunAll();
        }

        public static void RunAll()
        {
            UnityWebRequestTests.Run();
            TestPostStringBodyAndContentType();
            TestPostFormFieldsAndWwwForm();
            TestPostMultipartFormSections();
            TestPutByteArrayAndString();
            TestUploadHandlerRawWithCustomRequestHeadersAndDispose();
            TestPostAndPutErrorStatusCodes404And500();
            TestUploadHandlerFileAndDownloadHandlerFile();
        }

        private static void TestPostStringBodyAndContentType()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            CapturedHttpRequest? captured = null;
            using LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                captured = req;
                WriteHttpResponse(stream, 201, "Created", "application/json", "{\"created\":true}");
            });

            string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/api/create";
            string jsonPayload = "{\"title\":\"unitas\",\"count\":7}";
            using UnityWebRequest request = UnityWebRequest.Post(url, jsonPayload, "application/json");
            request.timeout = 5;
            UnityWebRequestAsyncOperation op = request.SendWebRequest();

            Check.Equal("POST JSON operation isDone", true, op.isDone);
            Check.Equal("POST JSON result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
            Check.Equal("POST JSON responseCode is 201", 201, (int)request.responseCode);
            Check.Equal("POST JSON response body", "{\"created\":true}", request.downloadHandler!.text);
            Check.That("POST JSON server received request", captured != null);
            Check.Equal("POST JSON server observed method", "POST", captured!.Method);
            Check.Equal("POST JSON server observed path", "/api/create", captured.Path);
            Check.Equal("POST JSON server observed body", jsonPayload, captured.BodyText);
            Check.That(
                "POST JSON server observed Content-Type header",
                captured.Headers.TryGetValue("Content-Type", out string? ct)
                && ct.Contains("application/json", StringComparison.OrdinalIgnoreCase));
            Check.Equal("POST JSON uploadedBytes matches payload length", (int)Encoding.UTF8.GetByteCount(jsonPayload), (int)request.uploadedBytes);
            Check.Equal("POST JSON uploadProgress is 1", 1f, request.uploadProgress);
        }

        private static void TestPostFormFieldsAndWwwForm()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            CapturedHttpRequest? capturedDict = null;
            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                capturedDict = req;
                WriteHttpResponse(stream, 200, "OK", "text/plain", "form-ok");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/form/login";
                Dictionary<string, string> fields = new Dictionary<string, string>
                {
                    { "username", "alice" },
                    { "role", "admin user" },
                };

                using UnityWebRequest request = UnityWebRequest.Post(url, fields);
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("POST form fields result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
                Check.Equal("POST form fields responseCode is 200", 200, (int)request.responseCode);
                Check.That("POST form fields captured", capturedDict != null);
                Check.Equal("POST form fields method", "POST", capturedDict!.Method);
                Check.Equal("POST form fields encoded body", "username=alice&role=admin%20user", capturedDict.BodyText);
                Check.That(
                    "POST form fields Content-Type is urlencoded",
                    capturedDict.Headers.TryGetValue("Content-Type", out string? ct)
                    && ct.Contains("application/x-www-form-urlencoded", StringComparison.OrdinalIgnoreCase));
            }

            CapturedHttpRequest? capturedWwwForm = null;
            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                capturedWwwForm = req;
                WriteHttpResponse(stream, 200, "OK", "text/plain", "wwwform-ok");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/form/www";
                WWWForm form = new WWWForm();
                form.AddField("player", "bob");
                form.AddField("score", 42);

                using UnityWebRequest request = UnityWebRequest.Post(url, form);
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("POST WWWForm result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
                Check.That("POST WWWForm captured", capturedWwwForm != null);
                Check.Equal("POST WWWForm encoded body", "player=bob&score=42", capturedWwwForm!.BodyText);
            }

            CapturedHttpRequest? capturedPostWwwForm = null;
            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                capturedPostWwwForm = req;
                WriteHttpResponse(stream, 200, "OK", "text/plain", "postwwwform-ok");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/form/escaped";
                using UnityWebRequest request = UnityWebRequest.PostWwwForm(url, "hello world+unitas");
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("PostWwwForm result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
                Check.That("PostWwwForm captured", capturedPostWwwForm != null);
                Check.Equal("PostWwwForm URL-escaped body", "hello%20world%2Bunitas", capturedPostWwwForm!.BodyText);
            }
        }

        private static void TestPostMultipartFormSections()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            CapturedHttpRequest? captured = null;
            using LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                captured = req;
                WriteHttpResponse(stream, 200, "OK", "text/plain", "multipart-ok");
            });

            string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/upload/multipart";
            List<IMultipartFormSection> sections = new List<IMultipartFormSection>
            {
                new MultipartFormDataSection("meta", "level-1"),
                new MultipartFormFileSection("savefile", Encoding.UTF8.GetBytes("SAVE_BYTES"), "save.dat", "application/octet-stream"),
            };

            using UnityWebRequest request = UnityWebRequest.Post(url, sections);
            request.timeout = 5;
            request.SendWebRequest();

            Check.Equal("POST multipart result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
            Check.That("POST multipart captured", captured != null);
            Check.That(
                "POST multipart Content-Type has boundary",
                captured!.Headers.TryGetValue("Content-Type", out string? ct)
                && ct.Contains("multipart/form-data", StringComparison.OrdinalIgnoreCase)
                && ct.Contains("boundary=", StringComparison.OrdinalIgnoreCase));
            Check.That("POST multipart body contains field section", captured.BodyText.Contains("name=\"meta\"", StringComparison.Ordinal));
            Check.That("POST multipart body contains field value", captured.BodyText.Contains("level-1", StringComparison.Ordinal));
            Check.That("POST multipart body contains file section", captured.BodyText.Contains("filename=\"save.dat\"", StringComparison.Ordinal));
            Check.That("POST multipart body contains file payload", captured.BodyText.Contains("SAVE_BYTES", StringComparison.Ordinal));
        }

        private static void TestPutByteArrayAndString()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            CapturedHttpRequest? capturedBytes = null;
            byte[] rawPayload = new byte[] { 0x10, 0x20, 0x30, 0x40, 0x55 };
            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                capturedBytes = req;
                WriteHttpResponse(stream, 200, "OK", "text/plain", "put-bytes-ok");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/resource/bin";
                using UnityWebRequest request = UnityWebRequest.Put(url, rawPayload);
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("PUT byte[] result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
                Check.Equal("PUT byte[] responseCode is 200", 200, (int)request.responseCode);
                Check.That("PUT byte[] captured", capturedBytes != null);
                Check.Equal("PUT byte[] method is PUT", "PUT", capturedBytes!.Method);
                Check.Equal("PUT byte[] payload length", rawPayload.Length, capturedBytes.BodyBytes.Length);
                for (int i = 0; i < rawPayload.Length; i++)
                {
                    Check.Equal("PUT byte[] element " + i.ToString(CultureInfo.InvariantCulture), (int)rawPayload[i], (int)capturedBytes.BodyBytes[i]);
                }
            }

            CapturedHttpRequest? capturedString = null;
            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                capturedString = req;
                WriteHttpResponse(stream, 200, "OK", "text/plain", "put-string-ok");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/resource/text";
                using UnityWebRequest request = UnityWebRequest.Put(url, "updated-state-payload");
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("PUT string result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
                Check.That("PUT string captured", capturedString != null);
                Check.Equal("PUT string method is PUT", "PUT", capturedString!.Method);
                Check.Equal("PUT string body matches", "updated-state-payload", capturedString.BodyText);
                Check.Equal("PUT string response text", "put-string-ok", request.downloadHandler!.text);
            }
        }

        private static void TestUploadHandlerRawWithCustomRequestHeadersAndDispose()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            CapturedHttpRequest? captured = null;
            using LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                captured = req;
                WriteHttpResponse(stream, 200, "OK", "application/json", "{\"status\":\"accepted\"}");
            });

            string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/custom/raw";
            byte[] payload = Encoding.UTF8.GetBytes("{\"op\":\"patch\"}");
            UploadHandlerRaw uploadHandler = new UploadHandlerRaw(payload)
            {
                contentType = "application/merge-patch+json",
            };

            UnityWebRequest request = new UnityWebRequest(url, UnityWebRequest.kHttpVerbPOST, new DownloadHandlerBuffer(), uploadHandler);
            request.SetRequestHeader("X-Custom-Trace", "trace-99");
            request.SetRequestHeader("X-Api-Key", "top-secret-key");
            request.timeout = 5;
            request.SendWebRequest();

            Check.Equal("Custom UploadHandlerRaw result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
            Check.Equal("Custom UploadHandlerRaw progress is 1", 1f, uploadHandler.progress);
            Check.That("Custom UploadHandlerRaw captured", captured != null);
            Check.Equal("Custom UploadHandlerRaw header X-Custom-Trace", "trace-99", captured!.Headers["X-Custom-Trace"]);
            Check.That(
                "Custom UploadHandlerRaw contentType transmitted",
                captured.Headers.TryGetValue("Content-Type", out string? ct)
                && ct.Contains("application/merge-patch+json", StringComparison.OrdinalIgnoreCase));
            Check.Equal("Custom UploadHandlerRaw body matches", "{\"op\":\"patch\"}", captured.BodyText);

            UnityWebRequestDiagnostic diag = UnityWebRequest.Diagnostics[UnityWebRequest.Diagnostics.Count - 1];
            Check.Equal("Custom UploadHandlerRaw X-Api-Key redacted in diagnostics", "[REDACTED]", diag.RequestHeaders["X-Api-Key"]);
            Check.Equal("Custom UploadHandlerRaw X-Custom-Trace preserved in diagnostics", "trace-99", diag.RequestHeaders["X-Custom-Trace"]);

            request.Dispose();
            bool threwAfterDispose = false;
            try
            {
                _ = uploadHandler.data;
            }
            catch (ObjectDisposedException)
            {
                threwAfterDispose = true;
            }

            Check.Equal("UploadHandler disposed when UnityWebRequest disposed", true, threwAfterDispose);
        }

        private static void TestPostAndPutErrorStatusCodes404And500()
        {
            UnityWebRequest.ClearDiagnostics();
            UnityWebRequest.CrossOriginDenialRule = null;
            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                _ = req;
                WriteHttpResponse(stream, 404, "Not Found", "application/json", "{\"error\":\"not_found\"}");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/missing/post";
                using UnityWebRequest request = UnityWebRequest.Post(url, "{\"x\":1}", "application/json");
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("POST 404 isDone", true, request.isDone);
                Check.Equal("POST 404 result is ProtocolError", (int)UnityWebRequest.Result.ProtocolError, (int)request.result);
                Check.Equal("POST 404 responseCode is 404", 404, (int)request.responseCode);
                Check.That("POST 404 error contains 404", request.error != null && request.error.Contains("404", StringComparison.Ordinal));
                Check.Equal("POST 404 error body preserved", "{\"error\":\"not_found\"}", request.downloadHandler!.text);
            }

            using (LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
            {
                _ = req;
                WriteHttpResponse(stream, 500, "Internal Server Error", "text/plain", "server-fault");
            }))
            {
                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/fault/put";
                using UnityWebRequest request = UnityWebRequest.Put(url, "payload");
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("PUT 500 isDone", true, request.isDone);
                Check.Equal("PUT 500 result is ProtocolError", (int)UnityWebRequest.Result.ProtocolError, (int)request.result);
                Check.Equal("PUT 500 responseCode is 500", 500, (int)request.responseCode);
                Check.That("PUT 500 error contains 500", request.error != null && request.error.Contains("500", StringComparison.Ordinal));
                Check.Equal("PUT 500 error body preserved", "server-fault", request.downloadHandler!.text);
            }
        }

        private static void TestUploadHandlerFileAndDownloadHandlerFile()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas-uwr-file-" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(tempDir);
            string uploadFilePath = Path.Combine(tempDir, "upload.bin");
            string downloadFilePath = Path.Combine(tempDir, "download.bin");
            try
            {
                File.WriteAllBytes(uploadFilePath, Encoding.UTF8.GetBytes("file-upload-contents"));
                CapturedHttpRequest? captured = null;
                using LoopbackMethodServer server = new LoopbackMethodServer((CapturedHttpRequest req, Stream stream) =>
                {
                    captured = req;
                    WriteHttpResponse(stream, 200, "OK", "application/octet-stream", "confirmed:" + req.BodyText);
                });

                string url = "http://127.0.0.1:" + server.Port.ToString(CultureInfo.InvariantCulture) + "/files/sync";
                using UnityWebRequest request = new UnityWebRequest(
                    url,
                    UnityWebRequest.kHttpVerbPUT,
                    new DownloadHandlerFile(downloadFilePath),
                    new UploadHandlerFile(uploadFilePath));
                request.timeout = 5;
                request.SendWebRequest();

                Check.Equal("File handlers result is Success", (int)UnityWebRequest.Result.Success, (int)request.result);
                Check.That("File upload captured by server", captured != null);
                Check.Equal("File upload body matches file contents", "file-upload-contents", captured!.BodyText);
                Check.Equal("DownloadHandlerFile wrote file to disk", true, File.Exists(downloadFilePath));
                Check.Equal("DownloadHandlerFile content matches", "confirmed:file-upload-contents", File.ReadAllText(downloadFilePath, Encoding.UTF8));
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
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

        private sealed class CapturedHttpRequest
        {
            public CapturedHttpRequest(
                string method,
                string path,
                Dictionary<string, string> headers,
                byte[] bodyBytes)
            {
                Method = method;
                Path = path;
                Headers = headers;
                BodyBytes = bodyBytes;
                BodyText = Encoding.UTF8.GetString(bodyBytes);
            }

            public string Method { get; }

            public string Path { get; }

            public Dictionary<string, string> Headers { get; }

            public byte[] BodyBytes { get; }

            public string BodyText { get; }
        }

        private sealed class LoopbackMethodServer : IDisposable
        {
            private readonly Socket _listenSocket;
            private readonly Action<CapturedHttpRequest, Stream> _handler;
            private readonly Thread _acceptThread;
            private volatile bool _stopped;

            public LoopbackMethodServer(Action<CapturedHttpRequest, Stream> handler)
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
                        string requestLine = ReadLineAscii(stream);
                        ParseRequestLine(requestLine, out string method, out string path);
                        Dictionary<string, string> headers = ReadHeaders(stream);
                        byte[] bodyBytes = ReadBody(stream, headers);
                        CapturedHttpRequest captured = new CapturedHttpRequest(method, path, headers, bodyBytes);
                        _handler(captured, stream);
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

            private static string ReadLineAscii(Stream stream)
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

            private static void ParseRequestLine(string requestLine, out string method, out string path)
            {
                if (string.IsNullOrEmpty(requestLine))
                {
                    method = "GET";
                    path = "/";
                    return;
                }

                int firstSpace = requestLine.IndexOf(' ', StringComparison.Ordinal);
                if (firstSpace < 0)
                {
                    method = requestLine;
                    path = "/";
                    return;
                }

                method = requestLine.Substring(0, firstSpace);
                int secondSpace = requestLine.IndexOf(' ', firstSpace + 1);
                if (secondSpace < 0)
                {
                    path = requestLine.Substring(firstSpace + 1);
                    return;
                }

                path = requestLine.Substring(firstSpace + 1, secondSpace - firstSpace - 1);
            }

            private static Dictionary<string, string> ReadHeaders(Stream stream)
            {
                Dictionary<string, string> headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                while (true)
                {
                    string line = ReadLineAscii(stream);
                    if (string.IsNullOrEmpty(line))
                    {
                        break;
                    }

                    int colon = line.IndexOf(':', StringComparison.Ordinal);
                    if (colon > 0)
                    {
                        string key = line.Substring(0, colon).Trim();
                        string val = line.Substring(colon + 1).Trim();
                        headers[key] = val;
                    }
                }

                return headers;
            }

            private static byte[] ReadBody(Stream stream, Dictionary<string, string> headers)
            {
                if (!headers.TryGetValue("Content-Length", out string? lengthStr)
                    || !int.TryParse(lengthStr, NumberStyles.Integer, CultureInfo.InvariantCulture, out int contentLength)
                    || contentLength <= 0)
                {
                    return Array.Empty<byte>();
                }

                byte[] buffer = new byte[contentLength];
                int offset = 0;
                while (offset < contentLength)
                {
                    int read = stream.Read(buffer, offset, contentLength - offset);
                    if (read <= 0)
                    {
                        break;
                    }

                    offset += read;
                }

                if (offset == contentLength)
                {
                    return buffer;
                }

                byte[] trimmed = new byte[offset];
                Buffer.BlockCopy(buffer, 0, trimmed, 0, offset);
                return trimmed;
            }
        }
    }
}
