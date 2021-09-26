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
                {
                    // authorize
                    var role = HttpContext.User.FindFirst(ClaimTypes.Role)?.Value;
                    if (role != "Admin" && role != "Root") {
                        var account = AuthHelper.ExtractUsername(HttpContext.User);
                        if (res.Account != account) return NotFound();
                    }
                }
                return res;
            }
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
        public async Task<ActionResult<string>> NewJob([FromBody] NewJobMessage m) {
            var account = HttpContext.User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
            if (account == null) return BadRequest();

            FlowSnake id = FlowSnake.Generate();
            var job = new Job {
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
                    return BadRequest(new ErrorResponse(ErrorCodes.GIT_NO_SUCH_REVISION, result.message));
                }
                logger.LogInformation("Scheduleing job {0}", job.Id);
                await coordinatorService.ScheduleJob(job);
            } catch (KeyNotFoundException) {
                logger.LogInformation("No such test suite {1} for job {0}", job.Id, job.TestSuite);
                return BadRequest(new ErrorResponse(ErrorCodes.NO_SUCH_SUITE));
            } catch (TaskCanceledException) {
                logger.LogInformation("Fetching for job {0} timed out", job.Id);
                return BadRequest(new ErrorResponse(ErrorCodes.REVISION_FETCH_TIMEOUT));
            } catch (OutOfActiveTimeException) {
                logger.LogInformation("Fetching for job {0} timed out", job.Id);
                return BadRequest(new ErrorResponse(ErrorCodes.NOT_IN_ACTIVE_TIMESPAN));
            }
            return Ok(id.ToString());
        }

        /// <summary>
        /// Submit a job with identical parameters as the given job.
        /// </summary>
        /// <param name="id"></param>
        /// <returns></returns>
        [HttpPost("respawn/{id}")]
        public async Task<ActionResult<String>> RespawnJob([FromRoute] FlowSnake id) {
            // the returned job is non-tracking, so we can safely modify its data
            var job = await dbsvc.GetJob(id);
            if (job == null) return NotFound();

            // clear job stats, reset id
            job.ClearStats();
            job.Id = FlowSnake.Generate();

            await coordinatorService.ScheduleJob(job);
            return Ok(job.Id.ToString());
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
                return new GetRevisionResult {
                    isSuccess = true,
                    rev = rev
                };
            } else {
                return new GetRevisionResult {
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
