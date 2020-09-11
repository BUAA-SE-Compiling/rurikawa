using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Claims;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/job")]
    [Authorize("user")]
    public class JobController : ControllerBase {
        public JobController(ILogger<JobController> logger, DbService dbsvc, JudgerCoordinatorService coordinatorService) {
            this.logger = logger;
            this.dbsvc = dbsvc;
            this.coordinatorService = coordinatorService;
        }

        private readonly ILogger<JobController> logger;
        private readonly DbService dbsvc;
        private readonly JudgerCoordinatorService coordinatorService;

        /// <summary>
        /// GETs a job by its identifier (stringified version)
        /// </summary>
        /// <param name="id"></param>
        /// <returns></returns>
        [HttpGet]
        [Route("{id}")]
        public async Task<ActionResult<Job>> GetJob(FlowSnake id) {
            var res = await dbsvc.GetJob(id);
            if (res == null) {
                return NotFound();
            } else {
                return res;
            }
        }

#pragma warning disable 
        public class NewJobMessage {
            public string Repo { get; set; }
            public string? Branch { get; set; }
            public FlowSnake TestSuite { get; set; }
            public List<string> Tests { get; set; }
        }
#pragma warning restore

        /// <summary>
        /// PUTs a new job
        /// </summary>
        [HttpPost("")]
        public async Task<IActionResult> NewJob([FromBody] NewJobMessage m) {
            var account = HttpContext.User.FindFirst(ClaimTypes.NameIdentifier).Value;
            FlowSnake id = FlowSnake.Generate();
            var job = new Job
            {
                Id = id,
                Account = account,
                Repo = m.Repo,
                Branch = m.Branch,
                TestSuite = m.TestSuite,
                Tests = m.Tests,
                Stage = JobStage.Queued,
            };
            try {
                await coordinatorService.ScheduleJob(job);
            } catch (KeyNotFoundException) {
                return BadRequest("No such test suite");
            }
            return Ok(id.ToString());
        }
    }
}
