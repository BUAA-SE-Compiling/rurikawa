using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Concurrency;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Security.Claims;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.WebsocketApi;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using StackExchange.Redis;

namespace Karenia.Rurikawa.Coordinator.Services {
    using FrontendWebsocketWrapperTy = JsonWebsocketWrapper<WsApiClientMsg, WsApiServerMsg>;

    public class FrontendConnection {
        public string Username { get; }
        public FrontendWebsocketWrapperTy Conn { get; }

        public FrontendConnection(FrontendWebsocketWrapperTy conn, string username) {
            Conn = conn;
            Username = username;
        }

        public ConcurrentDictionary<FlowSnake, ChannelMessageQueue> Sub = new();
    }

    public class FrontendUpdateService {
        private readonly JsonSerializerOptions jsonSerializerOptions;
        private readonly IServiceScopeFactory scopeProvider;
        private readonly RedisService redis;
        private readonly ILogger<FrontendUpdateService> logger;
        private readonly ILogger<FrontendWebsocketWrapperTy> wsLogger;

        public FrontendUpdateService(
            JsonSerializerOptions jsonSerializerOptions,
            IServiceScopeFactory serviceProvider,
            RedisService redis,
            ILogger<FrontendUpdateService> logger,
            ILogger<FrontendWebsocketWrapperTy> wsLogger) {
            this.jsonSerializerOptions = jsonSerializerOptions;
            this.scopeProvider = serviceProvider;
            this.redis = redis;
            this.logger = logger;
            this.wsLogger = wsLogger;
        }

        /// <summary>
        /// This is basically a concurrent hashset, but since C# doesn't support ZST, 
        /// we use a dummy int as value.
        /// </summary>
        private readonly ConcurrentDictionary<FrontendConnection, int> connectionHandles = new();

        /// <summary>
        /// Try to use the provided HTTP connection to create a WebSocket connection
        /// between coordinator and frontend. 
        /// </summary>
        /// <param name="ctx">
        ///     The provided connection. Must be upgradable into websocket.
        /// </param>
        /// <returns>
        ///     True if the websocket connection was made.
        /// </returns>
        public async ValueTask<bool> TryUseConnection(HttpContext ctx) {
            var scope = scopeProvider.CreateScope();
            var username = await Authorize(scope, ctx);

            if (username != null) {
                var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                var wrapper = new FrontendWebsocketWrapperTy(
                    ws,
                    jsonSerializerOptions,
                    4096,
                    wsLogger);
                var conn = new FrontendConnection(wrapper, username);

                connectionHandles.TryAdd(conn, 0);

                try {
                    using var _ = SetupObservables(conn);
                    await conn.Conn.WaitUntilClose();
                } catch (Exception) {
                }

                connectionHandles.TryRemove(conn, out var _);
                UnsubscribeAll(conn);

                return true;
            } else {
                ctx.Response.StatusCode = 401; // unauthorized
            }
            return false;
        }

        private static ValueTask<string?> Authorize(
            IServiceScope scope,
            HttpContext ctx) {
            var account = scope.ServiceProvider.GetService<AccountService>()!;
            if (ctx.Request.Query.TryGetValue("token", out var tokens)) {
                var token = tokens.First()!;
                var username = account.VerifyShortLivingToken(token);
                return new ValueTask<string?>(username);
            } else {
                return new ValueTask<string?>((string?)null);
            }
        }

        private IDisposable SetupObservables(FrontendConnection conn) {
            // conn.Conn.Messages.Connect();
            return conn.Conn.Messages.Subscribe((val) => {
                switch (val) {
                    case SubscribeMsg msg:
                        this.HandleSubscribeMsg(msg, conn);
                        break;
                    default:
                        logger.LogWarning("Unknown message: {0}", val);
                        break;
                }
            });
        }

        private void HandleSubscribeMsg(SubscribeMsg msg, FrontendConnection conn) {
            if (msg.Sub) {
                if (msg.Jobs != null) {
                    foreach (var job in msg.Jobs) {
                        this.SubscribeToJob(job, conn);
                    }
                }
            } else {
                if (msg.Jobs != null) {
                    foreach (var job in msg.Jobs) {
                        this.UnsubscribeJob(job, conn);
                    }
                }
            }
        }

        public void UnsubscribeJob(FlowSnake id, FrontendConnection conn) {
            if (conn.Sub.Remove(id, out var sub)) {
                sub.Unsubscribe(CommandFlags.FireAndForget);
            }
        }

        public async void SubscribeToJob(FlowSnake id, FrontendConnection conn) {
            var sub = await redis.GetSubscriber();
            var subscription = sub.Subscribe(JobSubscribeChannel(id));

            conn.Sub.TryAdd(id, subscription);

            subscription.OnMessage(async (val) => {
                try {
                    var update = JsonSerializer.Deserialize<JobStatusUpdateMsg>(val.Message, this.jsonSerializerOptions);
                    if (update == null) return;
                    await conn.Conn.SendMessage(update);
                } catch (Exception e) {
                    logger.LogError(e.ToString());
                }
            });
        }

        public void UnsubscribeAll(FrontendConnection conn) {
            foreach (var val in conn.Sub.Values) {
                val.Unsubscribe(CommandFlags.FireAndForget);
            }
        }

        public async void OnJobStautsUpdate(FlowSnake id, JobStatusUpdateMsg msg) {
            var sub = await this.redis.GetSubscriber();
            sub.Publish(JobSubscribeChannel(id), JsonSerializer.Serialize(msg, this.jsonSerializerOptions));
        }

        static string JobSubscribeChannel(FlowSnake id) => $"sub:job:{id}";
    }
}
