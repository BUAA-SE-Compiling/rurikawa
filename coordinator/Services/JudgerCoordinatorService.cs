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
using Microsoft.AspNetCore.Http;
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
        public async ValueTask<bool> TryUseConnection(HttpContext ctx) {
            if (ctx.Request.Query.TryGetValue("token", out var authStrings)) {
                var auth = authStrings.FirstOrDefault();
                var tokenEntry = await Authenticate(auth);
                if (tokenEntry != null) {
                    // A connection id is passed to ensure that the client can safely
                    // replace a previous unfinished connection created by itself.
                    ctx.Request.Query.TryGetValue("conn", out var connId_);
                    var connId = connId_.FirstOrDefault();

                    var connLock = await connectionLock.LockAsync();
                    if (connections.TryGetValue(auth, out var lastConn)) {
                        if (lastConn.ConnectionId != null && connId != null && lastConn.ConnectionId == connId) {
                            // replace this session
                            await lastConn.Socket.Close(System.Net.WebSockets.WebSocketCloseStatus.PolicyViolation, "Duplicate connection", CancellationToken.None);
                            connections.Remove(auth);
                        } else {
                            ctx.Response.StatusCode = StatusCodes.Status409Conflict;
                            connLock.Dispose();
                            return false;
                        }
                    }
                    connLock.Dispose();

                    var ws = await ctx.WebSockets.AcceptWebSocketAsync();
                    var wrapper = new JudgerWebsocketWrapperTy(
                        ws,
                        jsonSerializerOptions,
                        4096);
                    var judger = new Judger(auth, tokenEntry, wrapper, connId);
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
                    case JobRequestMsg msg1:
                        OnJobRequestMessage(clientId, msg1); break;
                    case JobOutputMsg msg1:
                        OnJobOutputMessage(clientId, msg1); break;
                    default:
                        logger.LogCritical("Unable to handle message type {0}", msg.GetType().Name);
                        break;
                }
            });
        }

        /// <summary>
        /// Process a <c>ClientStatusMsg</c>. This function is present only for
        /// compatibility reasons.
        /// </summary>
        /// <param name="clientId"></param>
        /// <param name="msg"></param>
        /// <returns></returns>
        async void OnJudgerStatusUpdateMessage(string clientId, ClientStatusMsg msg) {
            using (await connectionLock.LockAsync()) {
                if (connections.TryGetValue(clientId, out var conn)) {
                    conn.CanAcceptNewTask = msg.CanAcceptNewTask;
                    conn.ActiveTaskCount = msg.ActiveTaskCount;
                }
            }
        }

        async void OnJobRequestMessage(string clientId, JobRequestMsg msg) {
            using (await queueLock.LockAsync()) {
                // should we dispatch a new job for this judger?
                using (await connectionLock.LockAsync()) {
                    if (connections.TryGetValue(clientId, out var conn)) {
                        conn.CanAcceptNewTask = msg.ActiveTaskCount > 0;
                        conn.ActiveTaskCount = msg.ActiveTaskCount;

                        logger.LogInformation("Judger {0} asked for {1} jobs", clientId, msg.RequestForNewTask);

                        var dispatchedCount = await TryDispatchJobFromDatabase(conn, msg.RequestForNewTask);

                        logger.LogInformation("Sent {1} jobs to {0}", clientId, msg.RequestForNewTask);
                    }
                }

            }
        }

        async void OnJobProgressMessage(string clientId, JobProgressMsg msg) {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);

            FlowSnake jobId = msg.JobId;
            var job = await db.Jobs.Where(j => j.Id == jobId).FirstOrDefaultAsync();
            if (job == null) {
                logger.LogError("Cannot find job {0}, error?", jobId);
                return;
            }

            frontendService.OnJobStautsUpdate(jobId, new Models.WebsocketApi.JobStatusUpdateMsg
            {
                JobId = jobId,
                Stage = msg.Stage
            });

            if (job.Stage != msg.Stage) {
                job.Stage = msg.Stage;
                await db.SaveChangesAsync();
            }

            // Clear output when job gets cancelled
            if (job.Stage == JobStage.Aborted || job.Stage == JobStage.Cancelled) {
                var redis = scope.ServiceProvider.GetService<RedisService>()!;
                var redisDb = await redis.GetDatabase();
                await redisDb.KeyDeleteAsync(
                    new RedisKey[] { FormatJobStdout(jobId), FormatJobError(jobId) },
                    flags: CommandFlags.FireAndForget);
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
                    BuildOutputFile = buildResultFilename,
                    Stage = JobStage.Finished,
                    JobResult = msg.JobResult,
                    TestResult = msg.Results
                });

                job.BuildOutputFile = buildResultFilename;
                job.Results = msg.Results ?? new Dictionary<string, TestResult>();
                job.Stage = JobStage.Finished;
                job.ResultKind = msg.JobResult;
                job.ResultMessage = msg.Message;
                job.FinishTime = DateTimeOffset.Now;
                await db.SaveChangesAsync();
                await tx.CommitAsync();
            }
        }

        async Task<string> UploadJobBuildOutput(FlowSnake jobId) {
            var db = await redis.GetDatabase();
            // 2MiB
            const int maxLength = 2 * 1024 * 1024;
            string? buildOutput = await db.StringGetRangeAsync(FormatJobStdout(jobId), -maxLength, -1);
            string? buildError = await db.StringGetRangeAsync(FormatJobError(jobId), -maxLength, -1);
            var res = new JobBuildOutput
            {
                Output = buildOutput,
                Error = buildError
            };
            var stringified = JsonSerializer.SerializeToUtf8Bytes(res, jsonSerializerOptions);
            using var scope = scopeProvider.CreateScope();
            var fileBucket = scope.ServiceProvider.GetService<SingleBucketFileStorageService>();

            var filename = $"job/{jobId}/build_output.json";

            await fileBucket!.UploadFile(filename, new MemoryStream(stringified), stringified.LongLength);

            await db.KeyDeleteAsync(
                new RedisKey[] { FormatJobStdout(jobId), FormatJobError(jobId) },
                flags: CommandFlags.FireAndForget);

            return filename;
        }

        async void OnPartialResultMessage(string clientId, PartialResultMsg msg) {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);

            var job = await db.Jobs.Where(j => j.Id == msg.JobId).SingleAsync();
            job.Results.Add(msg.TestId, msg.TestResult);
            await db.SaveChangesAsync();

            frontendService.OnJobStautsUpdate(msg.JobId, new Models.WebsocketApi.JobStatusUpdateMsg
            {
                JobId = msg.JobId,
                Stage = JobStage.Running,
                TestResult = new Dictionary<string, TestResult>()
                {
                    [msg.TestId] = msg.TestResult
                }
            });
        }

        async void OnJobOutputMessage(string clientId, JobOutputMsg msg) {
            var db = await this.redis.GetDatabase();
            // Autoclean output logs after a specific timeout.
            TimeSpan timeout = TimeSpan.FromMinutes(30);

            if (msg.Stream != null) {
                string key = FormatJobStdout(msg.JobId);
                await db.StringAppendAsync(
                    key,
                    msg.Stream,
                    flags: CommandFlags.FireAndForget);
                await db.KeyExpireAsync(key, timeout);
            }

            if (msg.Error != null) {
                string key = FormatJobError(msg.JobId);
                await db.StringAppendAsync(
                    key,
                    msg.Error,
                    flags: CommandFlags.FireAndForget);
                await db.KeyExpireAsync(key, timeout);
            }

            // if (msg.Stream != null)
            //     values.Add(new NameValueEntry("stream", msg.Stream));
            // if (msg.Error != null)
            //     values.Add(new NameValueEntry("error", msg.Error));

            // await db.StreamAddAsync(
            //     FormatJobStdout(msg.JobId),
            //     values.ToArray(),
            //     maxLength: 2000,
            //     flags: StackExchange.Redis.CommandFlags.FireAndForget);
        }

        public static string FormatJobError(FlowSnake id) => $"job:{id}:error";
        public static string FormatJobStdout(FlowSnake id) => $"job:{id}:stream";

        /// <summary>
        /// Check if the authorization header is valid. 
        /// </summary>
        async ValueTask<JudgerEntry?> Authenticate(string? tokenString) {
            if (tokenString == null) return null;
            using var scope = scopeProvider.CreateScope();
            var judgerService = scope.ServiceProvider.GetService<JudgerService>();
            if (judgerService == null) return null;
            return await judgerService.GetJudgerByToken(tokenString);
        }

        /// <summary>
        /// Dispatch a single job to the given judger. 
        /// 
        /// <p>
        /// Note: this method changes the state of the job, so it needs to be 
        /// saved to database after this method returns.
        /// </p>
        /// </summary>
        protected async ValueTask<bool> DispatchJob(Judger judger, Job job) {
            var redis = await this.redis.GetDatabase();
            await redis.StringSetAsync(FormatJobStdout(job.Id), "", expiry: TimeSpan.FromHours(2), flags: CommandFlags.FireAndForget);
            await redis.StringSetAsync(FormatJobError(job.Id), "", expiry: TimeSpan.FromHours(2), flags: CommandFlags.FireAndForget);

            try {
                await judger.Socket.SendMessage(new MultipleNewJobServerMsg()
                {
                    Jobs = new List<Job> { job },
                    ReplyTo = null
                });
                job.Judger = judger.Id;
                job.Stage = JobStage.Dispatched;
                job.DispatchTime = DateTimeOffset.Now;
                return true;
            } catch { return false; }
        }

        /// <summary>
        /// Dispatch a single job to the given judger. 
        /// 
        /// <p>
        /// Note: this method changes the state of the job, so it needs to be 
        /// saved to database after this method returns.
        /// </p>
        /// </summary>
        protected async ValueTask<bool> DispatchJobs(Judger judger, List<Job> jobs) {
            var redis = await this.redis.GetDatabase();

            try {
                await judger.Socket.SendMessage(new MultipleNewJobServerMsg()
                {
                    Jobs = jobs,
                    ReplyTo = null
                });

                foreach (var job in jobs) {
                    await redis.StringSetAsync(FormatJobStdout(job.Id), "", expiry: TimeSpan.FromHours(2), flags: CommandFlags.FireAndForget);
                    await redis.StringSetAsync(FormatJobError(job.Id), "", expiry: TimeSpan.FromHours(2), flags: CommandFlags.FireAndForget);

                    job.Judger = judger.Id;
                    job.Stage = JobStage.Dispatched;
                    job.DispatchTime = DateTimeOffset.Now;
                }
                return true;
            } catch { return false; }
        }

        static readonly TimeSpan DISPATH_TIMEOUT = TimeSpan.FromMinutes(30);

        protected async Task<Job?> GetLastUndispatchedJobFromDatabase(RurikawaDb db) {
            var res = await QueuedCriteria(db.Jobs)
                .OrderBy(j => j.Id)
                .FirstOrDefaultAsync();
            return res;
        }

        protected async Task<List<Job>> GetUndispatchedJobsFromDatabase(RurikawaDb db, int count) {
            var res = await QueuedCriteria(db.Jobs)
                .OrderBy(j => j.Id)
                .Take(count)
                .ToListAsync();
            return res;
        }

        public static IQueryable<Job> QueuedCriteria(IQueryable<Job> jobs) {
            var timeoutTaskStartsBeforeThis = DateTimeOffset.Now - DISPATH_TIMEOUT;
            return jobs.Where(
                    j =>
                       // queued jobs
                       j.Stage == JobStage.Queued
                    // aborted jobs
                    || j.Stage == JobStage.Aborted
                    // stalled jobs
                    || (
                        j.Stage != JobStage.Cancelled
                        && j.Stage != JobStage.Aborted
                        && j.Stage != JobStage.Skipped
                        && j.Stage != JobStage.Queued
                        && j.DispatchTime != null
                        && j.DispatchTime < timeoutTaskStartsBeforeThis)
                );
        }

        /// <summary>
        /// Dispatch ONE job from database.
        /// </summary>
        /// <param name="judger">The judger to dispatch from</param>
        /// <returns></returns>
        protected async ValueTask<bool> TryDispatchJobFromDatabase(Judger judger) {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);
            using var tx = await db.Database.BeginTransactionAsync(System.Data.IsolationLevel.Serializable);
            var job = await GetLastUndispatchedJobFromDatabase(db);
            if (job == null) return false;

            try {
                var res = await DispatchJob(judger, job);
                await db.SaveChangesAsync();
                await tx.CommitAsync();
                return res;
            } catch {
                await tx.RollbackAsync();
                return false;
            }
        }

        /// <summary>
        /// Dispatch MANY job from database. This method should be favored over
        /// <c>TryDispatchJobFromDatabase(Judger)</c>
        /// </summary>
        /// <param name="judger">The judger to dispatch from.</param>
        /// <param name="count">The maximum count of jobs to dispatch.</param>
        /// <returns></returns>
        protected async ValueTask<int> TryDispatchJobFromDatabase(Judger judger, int count) {
            using var scope = scopeProvider.CreateScope();
            var db = GetDb(scope);
            using var tx = await db.Database.BeginTransactionAsync(System.Data.IsolationLevel.Serializable);
            var job = await GetUndispatchedJobsFromDatabase(db, count);

            try {
                var res = await DispatchJobs(judger, job);
                await db.SaveChangesAsync();
                await tx.CommitAsync();
                return job.Count;
            } catch {
                await tx.RollbackAsync();
                return 0;
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

            using var tx = await db.Database.BeginTransactionAsync(System.Data.IsolationLevel.Serializable);
            var suite = await db.TestSuites.Where(x => x.Id == job.TestSuite).SingleOrDefaultAsync();
            if (suite == null) {
                throw new KeyNotFoundException();
            } else if (suite.EndTime < DateTime.Now) {
                // WARN: We don't check about submitting BEFORE activation; this is intentional.
                throw new OutOfActiveTimeException();
            }
            job.Stage = JobStage.Queued;

            // NOTE: We no longer directly dispatch jobs to judgers. Instead, 
            // we let judgers poll for new jobs using `JudgerStatusUpdateMessage`.

            db.Jobs.Add(job);
            await db.SaveChangesAsync();
            await tx.CommitAsync();
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
