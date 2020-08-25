using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Channels;
using System.Threading.Tasks;
using AsyncPrimitives;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Services {
    using JudgerWebsocketWrapperTy = JsonWebsocketWrapper<ClientMsg, ServerMsg>;

    /// <summary>
    /// A single-point coordinator for judgers.
    /// </summary>
    public class JudgerCoordinatorService {
        public JudgerCoordinatorService(
            IServiceScopeFactory serviceProvider,
            ILogger<JudgerCoordinatorService> logger
        ) {
            this.serviceProvider = serviceProvider;
            this.logger = logger;
        }


        /// <summary>
        /// The collection of runners, with token as keys.
        /// </summary>
        readonly Dictionary<string, Judger> connections = new Dictionary<string, Judger>();
        /// <summary>
        /// A mutex lock on connections and the status of connections inside it.
        /// Any changes on `Judger.ActiveTaskCount` and `Judger.CanAcceptNewTask`
        /// requires this lock to be acquired.
        /// </summary>
        readonly SemaphoreSlim connectionLock = new SemaphoreSlim(1);

        private readonly IServiceScopeFactory serviceProvider;
        private readonly ILogger<JudgerCoordinatorService> logger;

        /// <summary>
        /// Get database inside a scoped connection.
        /// </summary>
        /// <param name="scope">The scope requested</param>
        private RurikawaDb GetDb(IServiceScope scope) =>
            scope.ServiceProvider.GetRequiredService<RurikawaDb>();

        /// <summary>
        /// A channel indicating finished judgers' Id.
        /// </summary>
        private Channel<string> JudgerQueue { get; } = Channel.CreateUnbounded<string>();

        // readonly HashSet<string> vacantJudgers = new HashSet<string>();

        /// <summary>
        /// Try to use the provided HTTP connection to create a WebSocket connection
        /// between coordinator and judger. 
        /// </summary>
        /// <param name="ctx">
        ///     The provided connection. Must be upgradable into websocket.
        /// </param>
        /// <returns>
        ///     True if the websocket connection was made.
        /// </returns>
        public async ValueTask<bool> TryUseConnection(Microsoft.AspNetCore.Http.HttpContext ctx) {
            if (ctx.Request.Headers.TryGetValue("Authorization", out var auth)) {
                if (await CheckAuth(auth)) {
                    var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                    var wrapper = new JudgerWebsocketWrapperTy(ws);
                    var judger = new Judger(auth, wrapper);
                    // Mark the judger as available.
                    await judger.Finish();
                    {
                        await connectionLock.WaitAsync();
                        connections.Add(auth, judger);
                        connectionLock.Release();
                    }

                    await wrapper.WaitUntilClose();

                    {
                        await connectionLock.WaitAsync();
                        connections.Remove(auth);
                        connectionLock.Release();
                    }
                    return true;
                } else {
                    ctx.Response.StatusCode = 401; // unauthorized
                }
            } else {
                ctx.Response.StatusCode = 401; // unauthorized
            }
            return false;
        }

        async Task AssignObservables(string clientId, JudgerWebsocketWrapperTy client) {

        }

        async void OnJudgerStatusUpdateMessage(string clientId, ClientStatusMsg msg) {

        }

        /// <summary>
        /// Check if the authorization header is valid. 
        /// </summary>
        ValueTask<bool> CheckAuth(string authHeader) {
            return new ValueTask<bool>(true);
        }

        private async Task<Judger?> TryGetNextUsableJudger(bool blockNewTasks) {
            await connectionLock.WaitAsync();
            while (JudgerQueue.Reader.TryRead(out var nextJudger)) {
                if (connections.TryGetValue(nextJudger, out var conn)) {
                    if (conn.CanAcceptNewTask) {
                        // Change the status to false, until the judger reports
                        // it can accept new tasks again
                        conn.CanAcceptNewTask &= !blockNewTasks;
                        return conn;
                    }
                }
            }
            connectionLock.Release();
            return null;
        }

        /// <summary>
        /// Handle a single job 
        /// </summary>
        /// <param name="job"></param>
        /// <returns></returns>
        public async Task ScheduleJob(Job job) {
            try {
                var judger = await TryGetNextUsableJudger(true);
                if (judger != null) {
                    // TODO: schedule job to judger & change job status
                    await judger.Socket.SendMessage(new NewJobServerMsg());
                }
                // TODO: put job into database
            } catch {
                // TODO: Other cases
                throw new NotImplementedException();
            }
        }
    }
}
