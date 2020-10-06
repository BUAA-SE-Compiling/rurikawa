using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Claims;
using System.Text;
using System.Text.Unicode;
using System.Threading;
using System.Threading.Tasks;
using CliWrap;
using CliWrap.Buffered;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using NSwag.Annotations;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [Route("api/v1/job/")]
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
        [HttpGet("{id}")]
        public async Task<ActionResult<Job>> GetJob([FromRoute] FlowSnake id) {
            var res = await dbsvc.GetJob(id);
            if (res == null) {
                return NotFound();
            } else {
                return res;
            }
        }

        [HttpGet("")]
        public async Task<IList<Job>> GetJobs(
            [FromQuery] FlowSnake startId = new FlowSnake(),
            [FromQuery] int take = 20,
            [FromQuery] bool asc = false) {
            return await dbsvc.GetJobs(startId, take, asc);
        }

#pragma warning disable 
        public class NewJobMessage {
            public string Repo { get; set; }
            public string? Ref { get; set; }
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
                Branch = m.Ref,
                TestSuite = m.TestSuite,
                Tests = m.Tests,
                Stage = JobStage.Queued,
            };
            try {
                var result = await GetRevision(job);
                if (result.isSuccess) {
                    job.Revision = result.rev!;
                } else {
                    return BadRequest(new ErrorResponse("no_such_revision", result.message));
                }
                logger.LogInformation("Scheduleing job {0}", job.Id);
                await coordinatorService.ScheduleJob(job);
            } catch (KeyNotFoundException) {
                logger.LogInformation("No such test suite {1} for job {0}", job.Id, job.TestSuite);
                return BadRequest(new ErrorResponse("no_such_suite"));
            } catch (TaskCanceledException) {
                logger.LogInformation("Fetching for job {0} timed out", job.Id);
                return BadRequest(new ErrorResponse("revision_fetch_timeout"));
            } catch (OutOfActiveTimeException) {
                logger.LogInformation("Fetching for job {0} timed out", job.Id);
                return BadRequest(new ErrorResponse("not_in_active_timespan"));
            }
            return Ok(id.ToString());
        }

        /// <summary>
        /// Get the revision commit that is 
        /// </summary>
        /// <param name="job"></param>
        /// <returns></returns>
        [OpenApiIgnore]
        public async Task<GetRevisionResult> GetRevision(Job job) {
            var cancel = new CancellationTokenSource();
            cancel.CancelAfter(30_000);

            logger.LogInformation("Fetching revision for job {0}", job.Id);

            var res = await Cli.Wrap("git").WithArguments(new List<string>(){
                "ls-remote",
                job.Repo,
                job.Branch??"HEAD",
                "-q",
                "--exit-code"
            })
                .WithValidation(CommandResultValidation.None)
                .ExecuteBufferedAsync(cancel.Token);

            var stdout = res.StandardOutput;
            var exitCode = res.ExitCode;

            if (exitCode == 0) {
                var rev = stdout.Split('\t')[0];
                return new GetRevisionResult
                {
                    isSuccess = true,
                    rev = rev
                };
            } else {
                return new GetRevisionResult
                {
                    isSuccess = false,
                    message = stdout
                };
            }
        }

        public struct GetRevisionResult {
            public bool isSuccess;
            public string? message;
            public string? rev;
        }
    }
}
