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

namespace Karenia.Rurikawa.Coordinator.Controllers.Admin {
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

            Response.ContentType = "application/csv";
            Response.StatusCode = 200;
            Response.Headers.Add("Content-Disposition", $"inline; filename=\"{suiteId}.jobs.csv\"");

            await Response.StartAsync();
            // write to body of response
            using var sw = new StreamWriter(new StreamAsyncAdaptor(Response.Body));
            await using var swGuard = sw.ConfigureAwait(false);
            var csvWriter = new CsvWriter(sw);

            var startId = FlowSnake.MaxValue;
            const int batchSize = 100;
            while (true) {

                var batch = db.Jobs.FromSqlInterpolated($@"
                    select
                        distinct on (account)
                        *
                    from jobs
                    where test_suite = {suiteIdNum}
                    order by account, id desc
                    ")
                    .Where(job => job.Id < startId)
                    .Take(batchSize)
                    .Join(
                        db.Profiles,
                        (job) => job.Account,
                        (profile) => profile.Username,
                        (job, profile) =>
                            new JobDumpEntry { Job = job, StudentId = profile.StudentId })
                    .ToList();

                if (batch.Count == 0) break;
                startId = batch.Last().Job.Id;

                await Task.Run(() => {
                    foreach (var val in batch) {
                        WriteJobInfo(csvWriter, val, columns);
                    }
                });
            }
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

            Response.ContentType = "application/csv";
            Response.StatusCode = 200;
            Response.Headers.Add("Content-Disposition", $"inline; filename=\"{suiteId}.all-jobs.csv\"");

            await Response.StartAsync();

            // write to body of response
            using var sw = new StreamWriter(new StreamAsyncAdaptor(Response.Body));
            await using var swGuard = sw.ConfigureAwait(false);
            var csvWriter = new CsvWriter(sw);


            const int batchSize = 100;
            var startId = FlowSnake.MaxValue;

            await WriteJobHeaders(csvWriter, columns);
            while (true) {
                var batch = db.Jobs
                    .OrderByDescending(job => job.Id)
                    .Where((job) => job.TestSuite == suiteId && job.Id < startId)
                    .Take(batchSize)
                    .Join(
                        db.Profiles,
                        (job) => job.Account,
                        (profile) => profile.Username,
                        (job, profile) =>
                            new JobDumpEntry { Job = job, StudentId = profile.StudentId })
                    .ToList();

                if (batch.Count == 0) break;
                startId = batch.Last().Job.Id;

                await Task.Run(() => {
                    foreach (var val in batch) {
                        WriteJobInfo(csvWriter, val, columns);
                    }
                });

                await sw.FlushAsync();
            }


            return new EmptyResult();
        }

        /// <summary>
        /// Write the header of a JobResult to the given response's body.
        /// </summary>
        /// <param name="csvWriter"></param>
        /// <param name="columns"></param>
        /// <returns></returns>
        private async Task WriteJobHeaders(
            CsvWriter csvWriter,
            List<string> columns
        ) {
            await Task.Run(() => {
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
                        if (colResult.Score.HasValue) {
                            csv.WriteField(colResult.Score.Value.ToString());
                        } else {
                            csv.WriteField("1");
                        }
                    } else {
                        csv.WriteField("0");
                    }
                } else {
                    csv.WriteField("0");
                }
            }
            csv.NextRecord();
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
                        ErrorCodes.ALREADY_INITIALIZED,
                        "Root account is already initialized!"));
            } catch (AccountService.InvalidUsernameException e) {
                return BadRequest(new ErrorResponse(
                    ErrorCodes.INVALID_USERNAME,
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
            [FromServices] ProfileService profile,
            [FromBody] RootCreateAccountInfo msg
        ) {
            try {
                await accountService.CreateAccount(msg.Username, msg.Password, msg.Kind);
                await profile.InitializeProfileIfNotExists(msg.Username);
            } catch (AccountService.UsernameNotUniqueException e) {
                return BadRequest(new ErrorResponse(
                    ErrorCodes.USERNAME_NOT_UNIQUE,
                    $"Username {e.Username} is not unique inside database"));
            }
            return NoContent();
        }

        public class AdminEditPasswordMessage {
            public string Username { get; set; }
            public string Password { get; set; }
        }

        /// <summary>
        /// Force edit a user's password using admin rights.
        /// </summary>
        /// <param name="msg">The password editing message</param>
        /// <returns></returns>
        /// <exception cref="System.Exception"></exception>
        [HttpPost("edit-password")]
        public async Task<ActionResult> ChangePassword([FromBody] AdminEditPasswordMessage msg) {
            switch (await accountService.ForceEditPassword(msg.Username, msg.Password)) {
                case AccountService.EditPasswordResult.Success:
                    return NoContent();
                case AccountService.EditPasswordResult.AccountNotFound:
                    return NotFound();
                default: throw new System.Exception("Unreachable!");
            }
        }

        /// <summary>
        /// Get the basic information of the specific user.
        /// </summary>
        /// <param name="profileService">the service we will use</param>
        /// <param name="username">the username of this user</param>
        [HttpGet("user-info/{username}")]
        public async Task<ActionResult<AccountAndProfile>> GetUserInfo(
            [FromServices] ProfileService profileService,
            [FromRoute] string username) {
            var result = await profileService.GetAccountAndProfile(username);
            if (result == null) return NotFound();
            else return result;
        }

        /// <summary>
        /// Search for the specific user in database.
        /// </summary>
        [HttpGet("user-info")]
        public async Task<ActionResult<List<AccountAndProfile>>> GetUserInfoLists(
            [FromServices] ProfileService profileService,
            [FromQuery] string? usernameLike,
            [FromQuery] AccountKind? kind,
            [FromQuery] string? studentId,
            [FromQuery] string? startUsername,
            [FromQuery] bool descending,
            [FromQuery] bool searchNameUsingRegex = false,
            [FromQuery] int take = 50
        ) {
            var result = await profileService.SearchAccountAndProfile(
                usernameLike,
                kind,
                studentId,
                startUsername,
                descending,
                searchNameUsingRegex,
                take);
            return result;
        }
    }
}
