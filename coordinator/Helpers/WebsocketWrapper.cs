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
            this.Messages = messages.ObserveOn(Scheduler.Default).Publish();
            this.Errors = errors.ObserveOn(Scheduler.Default).Publish();
        }

        readonly WebSocket socket;
        readonly CancellationToken closeToken = new CancellationToken();
        readonly JsonSerializerOptions? serializerOptions;
        readonly ILogger<JsonWebsocketWrapper<TRecvMessage, TSendMessage>>? logger;

        byte[] recvBuffer;

        private readonly Subject<TRecvMessage> messages = new Subject<TRecvMessage>();
        public IConnectableObservable<TRecvMessage> Messages { get; }
        private readonly Subject<Exception> errors = new Subject<Exception>();
        public IConnectableObservable<Exception> Errors { get; }

        protected void DoubleRecvCapacity() {
            var newBuffer = new byte[this.recvBuffer.Length * 2];
            this.recvBuffer.CopyTo(new Span<byte>(newBuffer));
            this.recvBuffer = newBuffer;
        }

        protected async Task EventLoop() {
            while (true) {
                try {
                    if (this.socket.State == WebSocketState.Closed) {
                        this.messages.OnCompleted();
                        this.errors.OnCompleted();
                        return;
                    }
                    var result = await this.socket.ReceiveAsync(new ArraySegment<byte>(recvBuffer), this.closeToken);
                    var writtenBytes = 0;
                    while (!result.EndOfMessage) {
                        writtenBytes += result.Count;
                        this.DoubleRecvCapacity();
                        result = await this.socket.ReceiveAsync(new ArraySegment<byte>(
                            recvBuffer,
                            writtenBytes,
                            this.recvBuffer.Length - writtenBytes), this.closeToken);
                    }
                    writtenBytes += result.Count;

                    logger?.LogInformation($"Received message with {writtenBytes} bytes, type {result.MessageType}");

                    switch (result.MessageType) {
                        case WebSocketMessageType.Text:
                            var message = JsonSerializer.Deserialize<TRecvMessage>(
                                new ArraySegment<byte>(this.recvBuffer, 0, writtenBytes),
                                serializerOptions
                            );
                            logger?.LogInformation("{0}", message);
                            this.messages.OnNext(message);
                            break;
                        case WebSocketMessageType.Binary:
                            this.errors.OnNext(new UnexpectedBinaryMessageException());
                            break;
                        case WebSocketMessageType.Close:
                            this.messages.OnCompleted();
                            return;
                    }
                } catch (ConnectionAbortedException) {
                    break;
                } catch (OperationCanceledException) {
                    break;
                } catch (Exception e) {
                    logger?.LogError(e, "Failed to receive message");
                    this.errors.OnNext(e);
                }
            }
        }

        public async Task Close(
            WebSocketCloseStatus status,
            string message,
            CancellationToken cancellation
        ) {
            await this.socket.CloseAsync(status, message, cancellation);
        }

        public async Task WaitUntilClose() {
            await this.EventLoop();
        }

        public async Task SendMessage(TSendMessage message) {
            var buffer = JsonSerializer.SerializeToUtf8Bytes<TSendMessage>(message, this.serializerOptions);
            await this.socket.SendAsync(buffer, WebSocketMessageType.Text, true, this.closeToken);
        }

        public class UnexpectedBinaryMessageException : Exception { }
    }

}
