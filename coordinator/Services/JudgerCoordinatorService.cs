using System.Collections.Generic;
using System.Collections.Concurrent;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Judger;
using System.Threading.Tasks;
using System.Threading;
using System.Linq;
using AsyncPrimitives;

namespace Karenia.Rurikawa.Coordinator.Services
{
    /// <summary>
    /// A single-point coordinator for judgers
    /// </summary>
    public class JudgerCoordinatorService
    {
        readonly Dictionary<string, JudgerState> connections = new Dictionary<string, JudgerState>();
        readonly AsyncReaderWriterLock connectionLock = new AsyncReaderWriterLock();


        // readonly HashSet<string> vacantJudgers = new HashSet<string>();

        public async Task TryUseConnection(Microsoft.AspNetCore.Http.HttpContext ctx)
        {
            if (ctx.Request.Headers.TryGetValue("Authorization", out var auth))
            {
                if (await CheckAuth(auth))
                {
                    var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                    var wrapper = new JsonWebsocketWrapper<ClientMsg, ServerMsg>(ws);
                    var judger = new JudgerState(wrapper);
                    using (await connectionLock.OpenWriter())
                    {
                        connections.Add(auth, judger);
                    }

                    await wrapper.WaitUntilClose();

                    using (await connectionLock.OpenWriter())
                    {
                        connections.Remove(auth);
                    }
                }
                else
                {
                    ctx.Response.StatusCode = 401; // unauthorized
                }
            }
            else
            {
                ctx.Response.StatusCode = 401; // unauthorized
            }
        }

        ValueTask<bool> CheckAuth(string authHeader)
        {
            return new ValueTask<bool>(true);
        }

        public async Task AddNewJob()
        {
            try
            {
                using (await connectionLock.OpenWriter())
                {
                    var judger = connections.Where(x => x.Value.CanAcceptNewTask).First();
                    judger.Value.CanAcceptNewTask = false;
                    // TODO: Send task to this judger
                }
            }
            catch
            {
                // No vacant judger
                // TODO: Send task to database
            }
        }
    }
}

class JudgerState
{
    public JudgerState(JsonWebsocketWrapper<ClientMsg, ServerMsg> Socket)
    {
        this.Socket = Socket;
    }

    public int ActiveTaskCount { get; set; } = 0;
    public bool CanAcceptNewTask { get; set; } = true;
    public JsonWebsocketWrapper<ClientMsg, ServerMsg> Socket { get; }

    public async Task SendJob() { }
}

