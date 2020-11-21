using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using NReco.Csv;
using static Karenia.Rurikawa.Coordinator.Controllers.AccountController;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/admin")]
    [Authorize("admin", AuthenticationSchemes = JwtBearerDefaults.AuthenticationScheme + "," + "token")]
    public class AdminController : ControllerBase {
        private readonly DbService dbService;
        private readonly AccountService accountService;

        public AdminController(DbService dbService, AccountService accountService) {
            this.dbService = dbService;
            this.accountService = accountService;
        }

        [HttpGet]
        [Route("profile/dump")]
        public async Task<ActionResult> DumpProfiles([FromServices] RurikawaDb db) {
            var ptr = db.Profiles.AsAsyncEnumerable();

            Response.ContentType = "application/csv";
            Response.StatusCode = 200;
            await Response.StartAsync();

            int flushInterval = 50;

            await Task.Run(async () => {
                // write to body of response
                using var sw = new StreamWriter(new StreamAsyncAdaptor(Response.Body));
                await using var swGuard = sw.ConfigureAwait(false);
                var csvWriter = new CsvWriter(sw);
                csvWriter.QuoteAllFields = true;

                csvWriter.WriteField("username");
                csvWriter.WriteField("studentId");
                csvWriter.WriteField("email");

                csvWriter.NextRecord();

                int counter = 0;
                await foreach (var val in ptr) {
                    if (counter % flushInterval == 0) {
                        await sw.FlushAsync();
                    }

                    csvWriter.WriteField(val.Username);
                    csvWriter.WriteField(val.StudentId);
                    csvWriter.WriteField(val.Email);
                    csvWriter.NextRecord();

                    counter++;
                }
                await sw.FlushAsync();
            });

            return new EmptyResult();
        }

        [HttpPost]
        [Route("code")]
        public async Task<string> GetCode([FromServices] TemporaryTokenAuthService tokenAuthService) {
            var token = AccountService.GenerateToken();
            await tokenAuthService.AddToken(
                token,
                new System.Security.Claims.ClaimsIdentity(HttpContext.User.Claims),
                TimeSpan.FromMinutes(5));
            return token;
        }

        [HttpGet]
        [Route("suite/{suiteId}/jobs")]
        public async Task<IList<Job>> GetJobsFromSuite(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] FlowSnake startId = new FlowSnake(),
            [FromQuery] int take = 20,
            [FromQuery] bool asc = false) {
            return await dbService.GetJobs(
                startId: startId,
                take: take,
                asc: asc,
                bySuite: suiteId);
        }

        [HttpGet]
        [Route("suite/{suiteId}/est_dump_jobs")]
        public async Task<ActionResult> EstimateDumpSuiteJobDumpCount(
            [FromRoute] FlowSnake suiteId,
            [FromServices] RurikawaDb db) {
            var res = await db.Jobs.FromSqlInterpolated($@"
                select
                    distinct on (account)
                    *
                from jobs
                order by account, id desc
                ").Where((job) => job.TestSuite == suiteId)
                .CountAsync();

            return Ok(res);
        }

        [HttpGet]
        [Route("suite/{suiteId}/dump_jobs")]
        public async Task<ActionResult> DumpSuiteJobs(
            [FromRoute] FlowSnake suiteId,
            [FromServices] RurikawaDb db) {
            var suite = await dbService.GetTestSuite(suiteId);
            if (suite == null) return NotFound();

            var columns = suite.TestGroups
                    .SelectMany(group => group.Value.Select(value => value.Name))
                    .ToList();

            var suiteIdNum = suiteId.Num;
            var ptr = db.Jobs.FromSqlInterpolated($@"
            select
                distinct on (account)
                *
            from jobs
            where test_suite = {suiteIdNum}
            order by account, id desc
            ")
            .Join(
                db.Profiles,
                (job) => job.Account,
                (profile) => profile.Username,
                (job, profile) =>
                    new JobDumpEntry { Job = job, StudentId = profile.StudentId })
            .AsAsyncEnumerable();

            const int flushInterval = 50;

            Response.ContentType = "application/csv";
            Response.StatusCode = 200;
            Response.Headers.Add("Content-Disposition", $"inline; filename=\"{suiteId}.jobs.csv\"");

            await Response.StartAsync();
            await WriteJobResults(columns, ptr, flushInterval);
            return new EmptyResult();
        }

        [HttpGet]
        [Route("suite/{suiteId}/dump_all_jobs")]
        public async Task<ActionResult> DumpSuiteAllJobs(
            [FromRoute] FlowSnake suiteId,
            [FromServices] RurikawaDb db) {
            var suite = await dbService.GetTestSuite(suiteId);
            if (suite == null) return NotFound();

            var columns = suite.TestGroups
                    .SelectMany(group => group.Value.Select(value => value.Name))
                    .ToList();

            var ptr = db.Jobs
                .Where((job) => job.TestSuite == suiteId)
                .Join(
                    db.Profiles,
                    (job) => job.Account,
                    (profile) => profile.Username,
                    (job, profile) =>
                        new JobDumpEntry { Job = job, StudentId = profile.StudentId })
                .AsAsyncEnumerable();

            const int flushInterval = 50;

            Response.ContentType = "application/csv";
            Response.StatusCode = 200;
            Response.Headers.Add("Content-Disposition", $"inline; filename=\"{suiteId}.all-jobs.csv\"");

            await Response.StartAsync();
            await WriteJobResults(columns, ptr, flushInterval);
            return new EmptyResult();
        }

        private async Task WriteJobResults(
            List<string> columns,
            IAsyncEnumerable<JobDumpEntry> ptr,
            int flushInterval) {
            await Task.Run(async () => {
                // write to body of response
                using var sw = new StreamWriter(new StreamAsyncAdaptor(Response.Body));
                await using var swGuard = sw.ConfigureAwait(false);
                var csvWriter = new CsvWriter(sw);
                csvWriter.QuoteAllFields = true;

                csvWriter.WriteField("id");
                csvWriter.WriteField("account");
                csvWriter.WriteField("student_id");
                csvWriter.WriteField("repo");
                csvWriter.WriteField("revision");
                csvWriter.WriteField("stage");
                csvWriter.WriteField("result_kind");
                foreach (var col in columns) {
                    csvWriter.WriteField(col);
                }
                csvWriter.NextRecord();

                int counter = 0;
                await foreach (var val in ptr) {
                    if (counter % flushInterval == 0) {
                        await sw.FlushAsync();
                    }
                    WriteJobInfo(csvWriter, val, columns);
                    counter++;
                }
                await sw.FlushAsync();
            });
        }

        private class JobDumpEntry {
            public string? StudentId { get; set; }
            public Job Job { get; set; }
        }

        private void WriteJobInfo(CsvWriter csv, JobDumpEntry jobEntry, IList<string> columns) {
            var job = jobEntry.Job;
            csv.WriteField(job.Id.ToString());
            csv.WriteField(job.Account);
            csv.WriteField(jobEntry.StudentId ?? "");
            csv.WriteField(job.Repo);
            csv.WriteField(job.Revision);
            csv.WriteField(job.Stage.ToString());
            csv.WriteField(job.ResultKind.ToString());

            foreach (var column in columns) {
                if (job.Results.TryGetValue(column, out var colResult)) {
                    if (colResult.Kind == Models.Test.TestResultKind.Accepted) {
                        csv.WriteField("1");
                    } else {
                        csv.WriteField("0");
                    }
                } else {
                    csv.WriteField("0");
                }
            }
            csv.NextRecord();
        }

        public class CreateJudgerTokenRequest {
            public DateTimeOffset ExpireAt { get; set; }
            public bool IsSingleUse { get; set; }
            public List<string> Tags { get; set; }
        }

        [HttpPost("judger/register-token")]
        public async Task<string> GetJudgerRegisterToken(
            [FromServices] AccountService accountService,
            [FromBody] CreateJudgerTokenRequest req
            ) {
            return await accountService.CreateNewJudgerToken(req.ExpireAt, req.IsSingleUse, req.Tags);
        }

        [HttpGet("init")]
        [AllowAnonymous]
        public ValueTask<bool> IsInitalized() {
            return accountService.IsInitialzed();
        }

        [HttpPost("init")]
        [AllowAnonymous]
        public async Task<IActionResult> InitializeRoot(AccountInfo msg) {
            try {
                await accountService.InitializeRootAccount(msg.Username, msg.Password);
                return NoContent();
            } catch (AccountService.AlreadyInitializedException) {
                return BadRequest(
                    new ErrorResponse(
                        "already_initialzed",
                        "Root account is already initialized!"));
            } catch (AccountService.InvalidUsernameException e) {
                return BadRequest(new ErrorResponse(
                    "invalid_username",
                    $"Username '{e.Username}' must be 1 to 64 characters long, and must only contain letter, number, dash '-' and underscore '_'."
                ));
            }
        }

#pragma warning disable CS8618
        public class RootCreateAccountInfo {
            public string Username { get; set; }
            public string Password { get; set; }
            public AccountKind Kind { get; set; }
        }
#pragma warning restore CS8618

        /// <summary>
        /// Create a new account with given username, nickname and password.
        /// A result of <i>204 No Content</i> means the account is created successfully
        /// and the end-user may log in using the provided pair of username
        /// and password.
        /// </summary>
        [HttpPost("register")]
        public async Task<IActionResult> CreateAccount(
            [FromServices] SingleBucketFileStorageService fs,
            [FromBody] RootCreateAccountInfo msg
        ) {
            try {
                await accountService.CreateAccount(msg.Username, msg.Password, msg.Kind);
                await fs.Check();
            } catch (AccountService.UsernameNotUniqueException e) {
                return BadRequest(new ErrorResponse(
                    "username_not_unique",
                    $"Username {e.Username} is not unique inside database"));
            }
            return NoContent();
        }
    }
}
