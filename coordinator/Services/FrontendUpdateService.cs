using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Concurrency;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Security.Claims;
using System.Text.Json;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.WebsocketApi;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Services {
    using FrontendWebsocketWrapperTy = JsonWebsocketWrapper<WsApiClientMsg, WsApiServerMsg>;

    public class FrontendConnection {
        public string Username { get; }
        public FrontendWebsocketWrapperTy Conn { get; }

        public FrontendConnection(FrontendWebsocketWrapperTy conn, string username) {
            Conn = conn;
            Username = username;
        }

        public Dictionary<FlowSnake, IDisposable> JobSubscriptions { get; } = new Dictionary<FlowSnake, IDisposable>();
    }

    public class FrontendUpdateService {
        private readonly JsonSerializerOptions jsonSerializerOptions;
        private readonly IServiceScopeFactory scopeProvider;
        private readonly ILogger<FrontendUpdateService> logger;
        private readonly ILogger<FrontendWebsocketWrapperTy> wsLogger;

        public FrontendUpdateService(
            JsonSerializerOptions jsonSerializerOptions,
            IServiceScopeFactory serviceProvider,
            ILogger<FrontendUpdateService> logger,
            ILogger<FrontendWebsocketWrapperTy> wsLogger) {
            this.jsonSerializerOptions = jsonSerializerOptions;
            this.scopeProvider = serviceProvider;
            this.logger = logger;
            this.wsLogger = wsLogger;
        }

        /// <summary>
        /// This is basically a concurrent hashset, but since C# doesn't support ZST, 
        /// we use a dummy int as value.
        /// </summary>
        private readonly ConcurrentDictionary<FrontendConnection, int> connectionHandles = new ConcurrentDictionary<FrontendConnection, int>();

        private readonly ConcurrentDictionary<FlowSnake, (Subject<JobStatusUpdateMsg>, IObservable<JobStatusUpdateMsg>)> jobUpdateListeners =
            new ConcurrentDictionary<FlowSnake, (Subject<JobStatusUpdateMsg>, IObservable<JobStatusUpdateMsg>)>();

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

                connectionHandles.TryRemove(conn, out _);

                return true;
            } else {
                ctx.Response.StatusCode = 401; // unauthorized
            }
            return false;
        }

        private ValueTask<string?> Authorize(
            IServiceScope scope,
            HttpContext ctx) {
            var account = scope.ServiceProvider.GetService<AccountService>();
            if (ctx.Request.Query.TryGetValue("token", out var tokens)) {
                var token = tokens.First();
                var username = account.VerifyShortLivingToken(token);
                return new ValueTask<string?>(username);
            } else {
                return new ValueTask<string?>((string?)null);
            }
        }

        private IDisposable SetupObservables(FrontendConnection conn) {
            conn.Conn.Messages.Connect();
            return conn.Conn.Messages.Subscribe((val) => {
                switch (val) {
                    case SubscribeMsg msg: {
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
                        break;
                    }
                }
            });
        }

        public void UnsubscribeJob(FlowSnake id, FrontendConnection conn) {
            conn.JobSubscriptions.Remove(id, out var val);
            val?.Dispose();
        }

        public void SubscribeToJob(FlowSnake id, FrontendConnection conn) {
            logger.LogInformation("Subscribe to {0}", id);
            var sub = jobUpdateListeners.GetOrAdd(
                id,
                _x => {
                    var subject = new Subject<JobStatusUpdateMsg>();
                    var res = new RefCountFusedObservable<JobStatusUpdateMsg>(
                    subject.ObserveOn(Scheduler.Default),
                    () => {
                        jobUpdateListeners.TryRemove(id, out _);
                    });
                    return (subject, res);
                });

            var subscripton = sub.Item2.Subscribe(async (msg) => {
                logger.LogInformation("Update message on {0}", msg.JobId);
                await conn.Conn.SendMessage(msg);
            });
            conn.JobSubscriptions.Add(id, subscripton);
        }

        public void OnJobStautsUpdate(FlowSnake id, JobStatusUpdateMsg msg) {
            logger.LogInformation("Status Update: {0}", id);
            if (jobUpdateListeners.TryGetValue(id, out var val)) {
                logger.LogInformation("Status Update triggered: {0}", id);
                val.Item1.OnNext(msg);
            }
        }

        public void ClearNotifications(FrontendConnection conn) {
            foreach (var sub in conn.JobSubscriptions) {
                sub.Value.Dispose();
            }
        }
    }
}
