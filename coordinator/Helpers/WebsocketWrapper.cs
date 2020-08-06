using System;
using System.Net.WebSockets;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using System.Reactive;
using System.Collections.Generic;
using System.Threading;
using System.Reactive.Subjects;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Helpers
{
    public class JsonWebsocketWrapper<TRecvMessage, TSendMessage>
    {
        public JsonWebsocketWrapper(
            WebSocket socket,
            JsonSerializerOptions serializerOptions = null,
            int defaultBufferSize = 8192,
            ILogger<JsonWebsocketWrapper<TRecvMessage, TSendMessage>> logger = null)
        {
            this.socket = socket;
            this.serializerOptions = serializerOptions;
            this.recvBuffer = new byte[defaultBufferSize];
            this.logger = logger;
        }

        readonly WebSocket socket;
        readonly CancellationToken closeToken = new CancellationToken();
        readonly JsonSerializerOptions serializerOptions;
        ILogger<JsonWebsocketWrapper<TRecvMessage, TSendMessage>> logger;

        byte[] recvBuffer;

        public Subject<TRecvMessage> Messages { get; } = new Subject<TRecvMessage>();
        public Subject<Exception> Errors { get; } = new Subject<Exception>();

        protected void DoubleRecvCapacity()
        {
            var newBuffer = new byte[this.recvBuffer.Length * 2];
            this.recvBuffer.CopyTo(new Span<byte>(newBuffer));
            this.recvBuffer = newBuffer;
        }

        protected async Task EventLoop()
        {
            while (true)
            {
                try
                {
                    if (this.socket.State == WebSocketState.Closed)
                    {
                        this.Messages.OnCompleted();
                        this.Errors.OnCompleted();
                        return;
                    }
                    var result = await this.socket.ReceiveAsync(new ArraySegment<byte>(recvBuffer), this.closeToken);
                    var writtenBytes = 0;
                    while (!result.EndOfMessage)
                    {
                        writtenBytes += result.Count;
                        this.DoubleRecvCapacity();
                        result = await this.socket.ReceiveAsync(new ArraySegment<byte>(
                            recvBuffer,
                            writtenBytes,
                            this.recvBuffer.Length - writtenBytes), this.closeToken);
                    }
                    writtenBytes += result.Count;

                    this.logger?.LogInformation($"Received message with {writtenBytes} bytes.");

                    switch (result.MessageType)
                    {
                        case WebSocketMessageType.Text:
                            var message = JsonSerializer.Deserialize<TRecvMessage>(new ArraySegment<byte>(this.recvBuffer, 0, writtenBytes), serializerOptions);
                            this.Messages.OnNext(message);
                            break;
                        case WebSocketMessageType.Binary:
                            this.Errors.OnNext(new UnexpectedBinaryMessageException());
                            break;
                        case WebSocketMessageType.Close:
                            this.Messages.OnCompleted();
                            return;
                    }
                }
                catch (Exception e)
                {
                    this.Errors.OnNext(e);
                }
            }
        }

        public async Task Close(WebSocketCloseStatus status, string message, CancellationToken cancellation)
        {
            await this.socket.CloseAsync(status, message, cancellation);
        }

        public async Task WaitUntilClose()
        {
            await this.EventLoop();
        }

        public async Task SendMessage(TSendMessage message)
        {
            var buffer = JsonSerializer.SerializeToUtf8Bytes<TSendMessage>(message, this.serializerOptions);
            await this.socket.SendAsync(buffer, WebSocketMessageType.Text, true, this.closeToken);
        }

        public class UnexpectedBinaryMessageException : Exception { }
    }

}
