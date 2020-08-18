using System;
using System.Collections.Generic;
using System.Collections.Concurrent;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Judger;
using System.Threading.Tasks;
using System.Threading;
using System.Linq;
using AsyncPrimitives;
using System.Threading.Channels;

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
        private Channel<string> FinishedJudgers { get; } = Channel.CreateUnbounded<string>();


        // readonly HashSet<string> vacantJudgers = new HashSet<string>();

        public async Task TryUseConnection(Microsoft.AspNetCore.Http.HttpContext ctx) {
            if (ctx.Request.Headers.TryGetValue("Authorization", out var auth)) {
                if (await CheckAuth(auth)) {
                    var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                    var wrapper = new JsonWebsocketWrapper<ClientMsg, ServerMsg>(ws);
                    var judger = new Judger(auth, wrapper, this.FinishedJudgers);
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
                        judgerId = await this.FinishedJudgers.Reader.ReadAsync();
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


/// <summary>
/// A runner of a specific testing task.
/// </summary>
class Judger {

    // public int ActiveTaskCount { get; set; } = 0;
    // public bool CanAcceptNewTask { get; set; } = true;
    public JsonWebsocketWrapper<ClientMsg, ServerMsg> Socket { get; }

    /// <summary>
    /// A channel to communicate with `JudgerState`,
    /// and to indicate if this judger is finished.
    /// </summary>
    public Channel<string> Chan { get; }

    public string Id { get => _id; }
    private string _id;

    public Judger(
        string id,
        JsonWebsocketWrapper<ClientMsg, ServerMsg> socket,
        Channel<string> chan
    ) {
        this._id = id;
        this.Socket = socket;
        this.Chan = chan;
    }

    /// <summary>
    /// Run a judger and get results.
    /// </summary>
    public async Task<int> Run() {
        // TODO: Actually run the judger.
        var rand = new Random();
        var dur = rand.Next(2000);
        // Run an expensive job.
        await Task.Delay(dur);
        // Send a signal to the channel when finished,
        // indicating availability.
        await this.Finish();
        return 0;
    }

    /// <summary>
    /// Tell the channel that the job is done.
    /// </summary>
    public async Task Finish() {
        await this.Chan.Writer.WriteAsync(this.Id);
    }
}
