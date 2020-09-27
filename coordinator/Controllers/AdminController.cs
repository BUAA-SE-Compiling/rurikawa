using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
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
        [Route("suite/{id}/jobs")]
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
