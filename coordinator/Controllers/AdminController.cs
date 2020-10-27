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
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using NReco.Csv;
using static Karenia.Rurikawa.Coordinator.Controllers.AccountController;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/admin")]
    [Authorize("admin")]
    public class AdminController : ControllerBase {
        private readonly DbService dbService;
        private readonly AccountService accountService;

        public AdminController(DbService dbService, AccountService accountService) {
            this.dbService = dbService;
            this.accountService = accountService;
        }

        [HttpGet]
        [Route("suite/{suiteId}/jobs")]
        public async Task<IList<Job>> GetJobsFromSuite(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] FlowSnake startId = new FlowSnake(),
            [FromQuery] int take = 20,
            [FromQuery] bool asc = false) {
            var username = AuthHelper.ExtractUsername(HttpContext.User);
            return await dbService.GetJobs(
                startId: startId,
                take: take,
                asc: asc,
                bySuite: suiteId);
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

            var ptr = db.Jobs.FromSqlInterpolated($@"
            select
                distinct on (account)
                *
            from jobs
            where test_suite = {suiteId.Num}
            order by account, id desc
            ").AsAsyncEnumerable();

            Response.StatusCode = 200;
            Response.ContentType = "application/csv";

            const int flushInterval = 100;

            // write to body of response
            var sw = new StreamWriter(Response.Body);
            var csvWriter = new CsvWriter(sw);
            csvWriter.QuoteAllFields = true;

            csvWriter.WriteField("id");
            csvWriter.WriteField("account");
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
            return new EmptyResult();
        }

        private void WriteJobInfo(CsvWriter csv, Job job, IList<string> columns) {
            csv.WriteField(job.Id.ToString());
            csv.WriteField(job.Account);
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

        public class JudgerStat {
            public int Count { get; set; }
            public int Connected { get; set; }
            public int Running { get; set; }
        }

        [HttpGet("judger/stat")]
        public async Task<JudgerStat> GetJudgerStat(
            [FromServices] JudgerCoordinatorService coordinatorService,
            [FromServices] RurikawaDb db) {
            var judgerCount = await db.Judgers.CountAsync();
            var (connectedCount, runningCount) = await coordinatorService.GetConnectedJudgerInfo();
            return new JudgerStat
            {
                Count = judgerCount,
                Connected = connectedCount,
                Running = runningCount
            };
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
