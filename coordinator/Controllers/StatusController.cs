using System;
using System.Reflection;
using System.Text.Json;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/status")]
    public class StatusController : ControllerBase {
        private const string JUDGER_STAT_CACHE_KEY = "stat-cache:judger";
        private const string QUEUE_STAT_CACHE_KEY = "stat-cache:job-queue";

        /// <summary>
        /// Always return 204.
        /// </summary>
        [HttpGet("ping")]
        public ActionResult Pong() => NoContent();

        /// <summary>
        /// Get the name and version of the running assembly.
        /// </summary>
        /// <returns>
        ///     A string formatted in the following fashion:
        /// <code>
        ///     {AssemblyName}, Version={Version}, {AdditionalData}
        /// </code>
        /// </returns>
        [HttpGet("assembly")]
        public string? GetAssembly() {
            AssemblyName? assemblyName = Assembly.GetEntryAssembly()?.GetName();
            return assemblyName == null ? null : $"{assemblyName.Name} v{assemblyName.Version}";
        }

        public class JudgerStat {
            public int Count { get; set; }
            public int Connected { get; set; }
            public int Running { get; set; }
        }

        /// <summary>
        /// Reports the stat of judger queue.
        /// </summary>
        /// <returns></returns>
        [HttpGet("judger")]
        public async Task<ActionResult<JudgerStat>> GetJudgerStat(
            [FromServices] JudgerCoordinatorService coordinatorService,
            [FromServices] RurikawaDb db,
            [FromServices] RedisService redis,
            [FromServices] JsonSerializerOptions jsonSerializerOptions
        ) {
            var red = await redis.GetDatabase();
            var judgerStat = await red.StringGetAsync(JUDGER_STAT_CACHE_KEY);
            if (!judgerStat.IsNullOrEmpty) {
                return new ContentResult() {
                    Content = judgerStat,
                    StatusCode = 200,
                    ContentType = "application/json"
                };
            }

            var judgerCount = await db.Judgers.CountAsync();
            var (connectedCount, runningCount) = await coordinatorService.GetConnectedJudgerInfo();
            var stat = new JudgerStat {
                Count = judgerCount,
                Connected = connectedCount,
                Running = runningCount
            };

            _ = await red.StringSetAsync(
                JUDGER_STAT_CACHE_KEY,
                JsonSerializer.Serialize(stat, jsonSerializerOptions),
                expiry: TimeSpan.FromSeconds(10)
            );

            return stat;
        }

        public class QueueStat {
            public int QueuedJobs { get; set; }
        }

        /// <summary>
        /// Reports the status of the job queue.
        /// </summary>
        /// <returns></returns>
        [HttpGet("job-queue")]
        public async Task<ActionResult<QueueStat>> GetJobQueueStat(
            [FromServices] RurikawaDb db,
            [FromServices] RedisService redis,
            [FromServices] JsonSerializerOptions jsonSerializerOptions
        ) {
            var red = await redis.GetDatabase();
            var judgerStat = await red.StringGetAsync(QUEUE_STAT_CACHE_KEY);
            if (!judgerStat.IsNullOrEmpty) {
                return new ContentResult() {
                    Content = judgerStat,
                    StatusCode = 200,
                    ContentType = "application/json"
                };
            }

            // TODO: Use redis to track jobs count?
            var jobCount = await JudgerCoordinatorService.QueuedCriteria(db.Jobs).CountAsync();
            var stat = new QueueStat { QueuedJobs = jobCount };

            _ = await red.StringSetAsync(
                QUEUE_STAT_CACHE_KEY,
                JsonSerializer.Serialize(stat, jsonSerializerOptions),
                expiry: TimeSpan.FromSeconds(20)
            );

            return stat;
        }
    }
}
