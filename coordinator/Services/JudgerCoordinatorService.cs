using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Dynamic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Channels;
using System.Threading.Tasks;
using AsyncPrimitives;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using StackExchange.Redis;
using Z.EntityFramework.Plus;

namespace Karenia.Rurikawa.Coordinator.Services {
    using JudgerWebsocketWrapperTy = JsonWebsocketWrapper<ClientMsg, ServerMsg>;

    /// <summary>
    /// A single-point coordinator for judgers.
    /// </summary>
    public class JudgerCoordinatorService {
        public JudgerCoordinatorService(
            JsonSerializerOptions jsonSerializerOptions,
            IServiceScopeFactory serviceProvider,
            FrontendUpdateService frontendService,
            RedisService redis,
            ILogger<JudgerCoordinatorService> logger,
            ILogger<JudgerWebsocketWrapperTy> wsLogger) {
            this.jsonSerializerOptions = jsonSerializerOptions;
            this.scopeProvider = serviceProvider;
            this.frontendService = frontendService;
            this.redis = redis;
            this.logger = logger;
            this.wsLogger = wsLogger;
        }


        /// <summary>
        /// The collection of runners, with tokens as keys.
        /// </summary>
        readonly Dictionary<string, Judger> connections = new Dictionary<string, Judger>();

        /// <summary>
        /// A mutex lock on connections and the status of connections inside it.
        /// Any changes on `Judger.ActiveTaskCount` and `Judger.CanAcceptNewTask`
        /// requires this lock to be acquired.
        /// </summary>
        readonly SemaphoreSlim connectionLock = new SemaphoreSlim(1);
        private readonly JsonSerializerOptions jsonSerializerOptions;
        private readonly IServiceScopeFactory scopeProvider;
        private readonly FrontendUpdateService frontendService;
        private readonly RedisService redis;
        private readonly ILogger<JudgerCoordinatorService> logger;
        private readonly ILogger<JudgerWebsocketWrapperTy> wsLogger;

        /// <summary>
        /// Get database inside a scoped connection.
        /// </summary>
        /// <param name="scope">The scope requested</param>
        private RurikawaDb GetDb(IServiceScope scope) =>
            scope.ServiceProvider.GetRequiredService<RurikawaDb>();

        /// <summary>
        /// A channel indicating finished judgers' Id.
        /// </summary>
        private LinkedList<string> JudgerQueue { get; } = new LinkedList<string>();

        /// <summary>
        /// A mutex lock on judger queue.
        /// </summary>
        /// <returns></returns>
        readonly SemaphoreSlim queueLock = new SemaphoreSlim(1);


        /// <summary>
        /// A channel indicating the incoming Jobs.
        /// </summary>
        private Channel<Job> JobQueue { get; } = Channel.CreateUnbounded<Job>();

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
            if (ctx.Request.Query.TryGetValue("token", out var auth)) {
                var tokenEntry = await Authenticate(auth);
                if (tokenEntry != null) {
                    var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                    var wrapper = new JudgerWebsocketWrapperTy(
                        ws,
                        jsonSerializerOptions,
                        4096,
                        wsLogger);
                    var judger = new Judger(auth, tokenEntry, wrapper);
                    {
                        using var _ = await connectionLock.LockAsync();
                        connections.Add(auth, judger);
                    }
                    logger.LogInformation($"Connected to judger {auth}");

                    /*
                     * Note:
                     *
                     * We do not add judger to judger queue upon creation,
                     * although it should be available. On the contrary, we rely
                     * on the judger to send a ClientStatusMessage to declare it
                     * is ready, and add it to queue at that time.
                     */

                    try {
                        using (var conn = judger.Socket.Messages.Connect())
                        using (var subscription = AssignObservables(auth, judger.Socket)) {
                            await wrapper.WaitUntilClose();
                        }
                    } catch (Exception e) {
                        logger.LogError(e, $"Aborted connection to judger {auth}");
                    }
                    logger.LogInformation($"Disconnected from judger {auth}");
                    {
                        using var __ = await queueLock.LockAsync();
                        if (JudgerQueue.First != null) {
                            var curr = JudgerQueue.First;
                            while (curr != null) {
                                if (curr.Value == auth) {
                                    JudgerQueue.Remove(curr);
                                }
                                curr = curr.Next;
                            }
                        }
                    }
                    {
                        using var _ = await connectionLock.LockAsync();
                        connections.Remove(auth);
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

        IDisposable AssignObservables(string clientId, JudgerWebsocketWrapperTy client) {
            return client.Messages.Subscribe((msg) => {
                logger.LogInformation($"Judger {clientId} sent message of type {msg.GetType().Name}");
                switch (msg) {
                    case JobResultMsg msg1:
                        OnJobResultMessage(clientId, msg1); break;
                    case JobProgressMsg msg1:
                        OnJobProgressMessage(clientId, msg1); break;
                    case PartialResultMsg msg1:
                        OnPartialResultMessage(clientId, msg1); break;
                    case ClientStatusMsg msg1:
                        OnJudgerStatusUpdateMessage(clientId, msg1); break;
                    case JobOutputMsg msg1:
                        OnJobOutputMessage(clientId, msg1); break;
                    default:
                        logger.LogCritical("Unable to handle message type {0}", msg.GetType().Name);
                        break;
                }
            });
        }

        async void OnJudgerStatusUpdateMessage(string clientId, ClientStatusMsg msg) {
            using (await queueLock.LockAsync()) {
                // should we dispatch a new job for this judger?
                var remainingDispatches = msg.RequestForNewTask;
                using (await connectionLock.LockAsync()) {
                    if (connections.TryGetValue(clientId, out var conn)) {
                        conn.CanAcceptNewTask = msg.CanAcceptNewTask;
                        conn.ActiveTaskCount = msg.ActiveTaskCount;
                        while (remainingDispatches > 0) {
                            if (await TryDispatchJobFromDatabase(conn)) {
                                remainingDispatches--;
                            } else {
                                break;
                            }
                        }
                    }
                }
                for (ulong i = 0; i < remainingDispatches; i++)
                    JudgerQueue.AddLast(clientId);
                logger.LogInformation("Status::Judger: {0}", DEBUG_LogEnumerator(JudgerQueue));
            }
        }

        async void OnJobProgressMessage(string clientId, JobProgressMsg msg) {
            // TODO: Send job progress to web clients
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);

            var job = await db.Jobs.Where(j => j.Id == msg.JobId).FirstOrDefaultAsync();
            if (job == null) {
                logger.LogError("Cannot find job {0}, error?", msg.JobId);
                return;
            }

            frontendService.OnJobStautsUpdate(msg.JobId, new Models.WebsocketApi.JobStatusUpdateMsg
            {
                JobId = msg.JobId,
                Stage = msg.Stage
            });

            if (job.Stage != msg.Stage) {
                job.Stage = msg.Stage;
                await db.SaveChangesAsync();
            }
        }

        public async void OnJobResultMessage(string clientId, JobResultMsg msg) {
            using var scope = scopeProvider.CreateScope();

            var buildResultFilename = await UploadJobBuildOutput(msg.JobId);

            var db = GetDb(scope);
            using (var tx = await db.Database.BeginTransactionAsync()) {
                var job = await db.Jobs.Where(job => job.Id == msg.JobId).SingleOrDefaultAsync();
                if (job == null) {
                    logger.LogError("Unable to find job {0} ({1}) in database! Please recheck", msg.JobId, msg.JobId.Num);
                    return;
                }

                frontendService.OnJobStautsUpdate(msg.JobId, new Models.WebsocketApi.JobStatusUpdateMsg
                {
                    JobId = msg.JobId,
                    Stage = JobStage.Finished,
                    JobResult = msg.JobResult,
                    TestResult = msg.Results
                });

                job.BuildOutputFile = buildResultFilename;
                job.Results = msg.Results ?? new Dictionary<string, TestResult>();
                job.Stage = JobStage.Finished;
                job.ResultKind = msg.JobResult;
                job.ResultMessage = msg.Message;
                await db.SaveChangesAsync();
                await tx.CommitAsync();
            }
        }

        async Task<string> UploadJobBuildOutput(FlowSnake jobId) {
            var db = await redis.GetDatabase();
            // 2MiB
            const int maxLength = 2 * 1024 * 1024;
            string? buildOutput = await db.StringGetRangeAsync(FormatJobStdout(jobId), -maxLength, -1);
            string? buildError = await db.StringGetRangeAsync(FormatJobStdout(jobId), -maxLength, -1);
            var res = new JobBuildOutput
            {
                Output = buildOutput,
                Error = buildError
            };
            var stringified = JsonSerializer.SerializeToUtf8Bytes(res, jsonSerializerOptions);
            using var scope = scopeProvider.CreateScope();
            var fileBucket = scope.ServiceProvider.GetService<SingleBucketFileStorageService>();

            var filename = $"job/{jobId}/build_output.json";

            await fileBucket.UploadFile(filename, new MemoryStream(stringified), stringified.LongLength);

            return filename;
        }

        async void OnPartialResultMessage(string clientId, PartialResultMsg msg) {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);

            var job = await db.Jobs.Where(j => j.Id == msg.JobId).SingleAsync();
            job.Results.Add(msg.TestId, msg.TestResult);
            await db.SaveChangesAsync();
        }

        async void OnJobOutputMessage(string clientId, JobOutputMsg msg) {
            var db = await this.redis.GetDatabase();
            var values = new List<NameValueEntry>();

            if (msg.Stream != null)
                await db.StringAppendAsync(
                    FormatJobStdout(msg.JobId),
                    msg.Stream,
                    flags: CommandFlags.FireAndForget);

            if (msg.Error != null)
                await db.StringAppendAsync(
                    FormatJobError(msg.JobId),
                    msg.Error,
                    flags: CommandFlags.FireAndForget);

            if (msg.Stream != null)
                values.Add(new NameValueEntry("stream", msg.Stream));
            if (msg.Error != null)
                values.Add(new NameValueEntry("error", msg.Error));

            await db.StreamAddAsync(
                FormatJobStdout(msg.JobId),
                values.ToArray(),
                maxLength: 2000,
                flags: StackExchange.Redis.CommandFlags.FireAndForget);
        }

        public static string FormatJobError(FlowSnake id) => $"job:{id}:error";
        public static string FormatJobStdout(FlowSnake id) => $"job:{id}:stream";

        /// <summary>
        /// Check if the authorization header is valid. 
        /// </summary>
        async ValueTask<JudgerEntry?> Authenticate(string tokenString) {
            using var scope = scopeProvider.CreateScope();
            var judgerService = scope.ServiceProvider.GetService<JudgerService>();
            return await judgerService.GetJudgerByToken(tokenString);
        }

        private async Task<Judger?> TryGetNextUsableJudger(bool blockNewTasks) {
            using (await connectionLock.LockAsync()) {
                while (JudgerQueue.First != null) {
                    var nextJudger = JudgerQueue.First;
                    JudgerQueue.RemoveFirst();
                    if (connections.TryGetValue(nextJudger.Value, out var conn)) {
                        if (conn.CanAcceptNewTask) {
                            // Change the status to false, until the judger reports
                            // it can accept new tasks again
                            conn.CanAcceptNewTask &= !blockNewTasks;
                            return conn;
                        }
                    }
                }
            }
            return null;
        }

        /// <summary>
        /// Dispatch a single job to the given judger
        /// </summary>
        protected async Task DispatchJob(Judger judger, Job job) {
            var redis = await this.redis.GetDatabase();
            await redis.StringSetAsync(FormatJobStdout(job.Id), "", expiry: TimeSpan.FromHours(2), flags: CommandFlags.FireAndForget);
            await redis.StringSetAsync(FormatJobError(job.Id), "", expiry: TimeSpan.FromHours(2), flags: CommandFlags.FireAndForget);

            await judger.Socket.SendMessage(new NewJobServerMsg()
            {
                Job = job
            });
            job.Stage = JobStage.Dispatched;
        }

        protected async Task<Job?> GetLastUndispatchedJobFromDatabase(RurikawaDb db) {
            var res = await db.Jobs.Where(j => j.Stage == JobStage.Queued)
                .OrderBy(j => j.Id)
                .FirstOrDefaultAsync();
            return res;
        }

        protected async ValueTask<bool> TryDispatchJobFromDatabase(Judger judger) {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);
            var job = await GetLastUndispatchedJobFromDatabase(db);
            if (job == null) return false;
            job.Stage = JobStage.Dispatched;
            await db.SaveChangesAsync();
            try {
                await DispatchJob(judger, job);
                return true;
            } catch {
                job.Stage = JobStage.Queued;
                await db.SaveChangesAsync();
                return false;
            }
        }

        protected static string DEBUG_LogEnumerator<T>(IEnumerable<T> x) {
            var sb = new StringBuilder();
            var first = true;
            int idx = 0;
            foreach (var v in x) {
                if (!first) {
                    sb.Append(", ");
                } else {
                    first = false;
                }
                sb.Append(idx).Append(": ");
                sb.Append(v);
                idx++;
            }
            return sb.ToString();
        }

        public async Task ScheduleJob(Job job) {
            // Save this job to database
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);

            var suite = await db.TestSuites.Where(x => x.Id == job.TestSuite).SingleOrDefaultAsync();
            if (suite == null) {
                throw new KeyNotFoundException();
            } else if (suite.EndTime < DateTime.Now) {
                // WARN: We don't check about submitting BEFORE active; this is intentional.
                throw new OutOfActiveTimeException();
            }

            job.Stage = JobStage.Queued;
            bool success = false;

            // Lock queue so no one can write to it, leading to race conditions
            using (await queueLock.LockAsync()) {
                while (!success) {
                    // Get the first usable judger
                    var judger = await TryGetNextUsableJudger(true);
                    if (judger == null) break;
                    try {
                        await DispatchJob(judger, job);
                        success = true;
                    } catch {
                        // If any exception occurs (including but not limited 
                        // to connection closed, web error, etc.), this try is 
                        // considered as unsuccessful.
                        success = false;
                    }
                    // If this try is unsuccessful, try a next one until
                    // no judger is usable
                }
            }

            db.Jobs.Add(job);
            await db.SaveChangesAsync();
        }

        public async ValueTask RevertJobStatus() {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);
            dynamic updatedObject = new ExpandoObject();
            updatedObject.Stage = JobStage.Queued;
            await db.Jobs
                .Where(j => j.Stage != JobStage.Finished && j.Stage != JobStage.Cancelled)
                .UpdateFromQueryAsync(j => updatedObject);
        }

        public void Stop() {
            JobQueue.Writer.Complete();
        }

        /// <summary>
        /// Get information about connected judgers
        /// </summary>
        /// <returns>(connectedCount, runningCount)</returns>
        public async Task<(int, int)> GetConnectedJudgerInfo() {
            await connectionLock.WaitAsync();
            var result = connections.Aggregate((0, 0), (cnt, val) => {
                if (val.Value.ActiveTaskCount > 0) {
                    return (cnt.Item1 + 1, cnt.Item2 + 1);
                } else {
                    return (cnt.Item1 + 1, cnt.Item2);
                }
            });
            connectionLock.Release();
            return result;
        }
    }

    public class OutOfActiveTimeException : Exception { }
}
