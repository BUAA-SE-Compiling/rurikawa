using System;
using System.Collections.Generic;
using System.Net.WebSockets;
using System.Reactive;
using System.Reactive.Concurrency;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Connections;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Helpers {
    public class JsonWebsocketWrapper<TRecvMessage, TSendMessage> {
        public JsonWebsocketWrapper(
            WebSocket socket,
            JsonSerializerOptions? serializerOptions = null,
            int defaultBufferSize = 8192,
            ILogger<JsonWebsocketWrapper<TRecvMessage, TSendMessage>>? logger = null
        ) {
            this.socket = socket;
            this.serializerOptions = serializerOptions;
            this.recvBuffer = new byte[defaultBufferSize];
            this.logger = logger;
            this.Messages = messages.ObserveOn(Scheduler.Default);
            this.Errors = errors.ObserveOn(Scheduler.Default);
        }

        readonly WebSocket socket;
        readonly CancellationToken closeToken = new();
        readonly JsonSerializerOptions? serializerOptions;
        readonly ILogger<JsonWebsocketWrapper<TRecvMessage, TSendMessage>>? logger;

        readonly SemaphoreSlim sendLock = new(1);

        byte[] recvBuffer;

        private readonly Subject<TRecvMessage> messages = new();
        public IObservable<TRecvMessage> Messages { get; }
        private readonly Subject<Exception> errors = new();
        public IObservable<Exception> Errors { get; }

        protected void DoubleRecvCapacity() {
            var newBuffer = new byte[recvBuffer.Length * 2];
            recvBuffer.CopyTo(new Span<byte>(newBuffer));
            recvBuffer = newBuffer;
        }

        protected async Task EventLoop() {
            while (true) {
                try {
                    if (socket.State == WebSocketState.Closed) {
                        messages.OnCompleted();
                        errors.OnCompleted();
                        return;
                    }
                    var result = await socket.ReceiveAsync(new ArraySegment<byte>(recvBuffer), closeToken);
                    var writtenBytes = 0;
                    while (!result.EndOfMessage) {
                        writtenBytes += result.Count;
                        DoubleRecvCapacity();
                        result = await socket.ReceiveAsync(
                            new ArraySegment<byte>(recvBuffer, writtenBytes, recvBuffer.Length - writtenBytes),
                            closeToken
                        );
                    }
                    writtenBytes += result.Count;

                    switch (result.MessageType) {
                        case WebSocketMessageType.Text:
                            var message = JsonSerializer.Deserialize<TRecvMessage>(
                                new ArraySegment<byte>(recvBuffer, 0, writtenBytes),
                                serializerOptions
                            );
                            if (message is null) {
                                continue;
                            }
                            messages.OnNext(message);
                            break;
                        case WebSocketMessageType.Binary:
                            errors.OnNext(new UnexpectedBinaryMessageException());
                            break;
                        case WebSocketMessageType.Close:
                            messages.OnCompleted();
                            return;
                    }
                } catch (WebSocketException e) {
                    if (e.WebSocketErrorCode is WebSocketError.ConnectionClosedPrematurely or WebSocketError.NativeError or WebSocketError.InvalidMessageType) {
                        break;
                    }
                } catch (ConnectionAbortedException) {
                    break;
                } catch (OperationCanceledException) {
                    break;
                } catch (Exception e) {
                    logger?.LogError(e, "Failed to receive message");
                    errors.OnNext(e);
                }
            }
        }

        public async Task Close(
            WebSocketCloseStatus status,
            string message,
            CancellationToken cancellation
        ) => await socket.CloseAsync(status, message, cancellation);

        public async Task WaitUntilClose() => await EventLoop();

        public async Task SendMessage(TSendMessage message) {
            using var lockHandle = await sendLock.LockAsync();
            var buffer = JsonSerializer.SerializeToUtf8Bytes(message, serializerOptions);
            await socket.SendAsync(buffer, WebSocketMessageType.Text, true, closeToken);
        }

        public class UnexpectedBinaryMessageException : Exception { }
    }
}
