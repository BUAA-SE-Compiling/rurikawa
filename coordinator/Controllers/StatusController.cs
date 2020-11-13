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
        /// <summary>
        /// Always return 204.
        /// </summary>
        [HttpGet("ping")]
        public ActionResult Pong() { return NoContent(); }

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
            return Assembly.GetEntryAssembly()?.GetName().FullName;
        }

        public class JudgerStat {
            public int Count { get; set; }
            public int Connected { get; set; }
            public int Running { get; set; }
        }

        [HttpGet("judger")]
        public async Task<ActionResult<JudgerStat>> GetJudgerStat(
            [FromServices] JudgerCoordinatorService coordinatorService,
            [FromServices] RurikawaDb db,
            [FromServices] RedisService redis,
            [FromServices] JsonSerializerOptions jsonSerializerOptions) {
            var red = await redis.GetDatabase();
            var judgerStat = await red.StringGetAsync("judger");
            if (!judgerStat.IsNullOrEmpty) {
                return new ContentResult()
                {
                    Content = (string)judgerStat,
                    StatusCode = 200,
                    ContentType = "application/json"
                };
            }

            var judgerCount = await db.Judgers.CountAsync();
            var (connectedCount, runningCount) = await coordinatorService.GetConnectedJudgerInfo();
            var stat = new JudgerStat
            {
                Count = judgerCount,
                Connected = connectedCount,
                Running = runningCount
            };

            await red.StringSetAsync(
                "judger",
                JsonSerializer.Serialize(stat, jsonSerializerOptions),
                expiry: TimeSpan.FromMinutes(2));

            return stat;
        }
    }
}
