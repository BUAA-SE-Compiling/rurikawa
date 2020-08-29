using System;
using System.Collections.Generic;
using System.Linq;
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
    [Route("api/v1/job/")]
    [Authorize()]
    public class JobController : ControllerBase {
        public JobController(ILogger<JobController> logger, JudgerCoordinatorService coordinatorService) {
            this.logger = logger;
            this.coordinatorService = coordinatorService;
        }

        private readonly ILogger<JobController> logger;
        private readonly JudgerCoordinatorService coordinatorService;

        /// <summary>
        /// GETs a job by its identifier (stringified version)
        /// </summary>
        /// <param name="id"></param>
        /// <returns></returns>
        [HttpGet]
        [Route("{id}")]
        public Job GetJob(string id) {
            throw new NotImplementedException();
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
        /// <param name="job"></param>
        /// <returns></returns>
        [HttpPost]
        [Authorize("user")]
        public async Task<string> NewJob(NewJobMessage msg) {
            FlowSnake id = FlowSnake.Generate();
            var job = new Job(
                id,
                msg.Repo,
                msg.Branch,
                msg.TestSuite,
                msg.Tests);
            await coordinatorService.ScheduleJob(job);
            return id.ToString();
        }
    }
}
