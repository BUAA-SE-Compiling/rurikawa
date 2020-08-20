using System;
using System.Collections.Generic;
using System.Threading.Channels;
using System.Threading.Tasks;
using AsyncPrimitives;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Judger;

namespace Karenia.Rurikawa.Coordinator.Services {
    /// <summary>
    /// A single-point coordinator for judgers.
    /// </summary>
    public class JudgerCoordinatorService {
        /// <summary>
        /// The collection of runners, with token as keys.
        /// </summary>
        readonly Dictionary<string, Judger> connections = new Dictionary<string, Judger>();
        readonly AsyncReaderWriterLock connectionLock = new AsyncReaderWriterLock();

        /// <summary>
        /// A channel indicating finished judgers' Id.
        /// </summary>
        private Channel<string> FinishedMsg { get; } = Channel.CreateUnbounded<string>();


        // readonly HashSet<string> vacantJudgers = new HashSet<string>();

        public async Task TryUseConnection(Microsoft.AspNetCore.Http.HttpContext ctx) {
            if (ctx.Request.Headers.TryGetValue("Authorization", out var auth)) {
                if (await CheckAuth(auth)) {
                    var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                    var wrapper = new JsonWebsocketWrapper<ClientMsg, ServerMsg>(ws);
                    var judger = new Judger(auth, wrapper, this.FinishedMsg);
                    // Mark the judger as available.
                    await judger.Finish();

                    using (await connectionLock.OpenWriter()) {
                        connections.Add(auth, judger);
                    }
                    await wrapper.WaitUntilClose();
                    using (await connectionLock.OpenWriter()) {
                        connections.Remove(auth);
                    }
                } else {
                    ctx.Response.StatusCode = 401; // unauthorized
                }
            } else {
                ctx.Response.StatusCode = 401; // unauthorized
            }
        }

        ValueTask<bool> CheckAuth(string authHeader) {
            return new ValueTask<bool>(true);
        }

        public async Task<int> HandleJob() {
            try {
                using (await connectionLock.OpenWriter()) {
                    // Get an Id for a judger that is finished AND available.
                    string judgerId;
                    do {
                        judgerId = await this.FinishedMsg.Reader.ReadAsync();
                    } while (!connections.ContainsKey(judgerId));

                    using (await connectionLock.OpenReader()) {
                        var judger = connections[judgerId];
                        // TODO: Send task to judger
                        var res = await judger.Run();
                        return res;
                    }
                }
            } catch {
                // TODO: Other cases
                throw new NotImplementedException();
            }
        }
    }
}
