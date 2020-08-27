using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [Route("api/v1/account")]
    public class AccountController : ControllerBase {
        private readonly ILogger<JudgerApiController> logger;
        private readonly RurikawaDb db;
        private readonly AccountService accountService;

        public AccountController(
            ILogger<JudgerApiController> logger,
            RurikawaDb db,
            AccountService accountService
        ) {
            this.logger = logger;
            this.db = db;
            this.accountService = accountService;
        }
#pragma warning disable CS8618  
        public class RegisterAccountMessage {
            public string Username { get; set; }
            public string Password { get; set; }
        }
#pragma warning restore CS8618

        /// <summary>
        /// Create a new account with given username, nickname and password. 
        /// A result of <i>204 No Content</i> means the account is created successfully
        /// and the end-user may log in using the provided pair of username
        /// and password.
        /// </summary>
        [Route("register")]
        public async Task<IActionResult> RegisterAccount(
            [FromBody] RegisterAccountMessage msg
        ) {
            try {
                await accountService.CreateAccount(msg.Username, msg.Password);
            } catch (AccountService.UsernameNotUniqueException e) {
                return BadRequest(new ErrorResponse(
                    "username_not_unique",
                    $"Username {e.Username} is not unique inside database"));
            }
            return NoContent();
        }

        [Route("edit/password")]
        [Authorize("user")]
        public async Task<IActionResult> EditPassword(
            [FromQuery] string originalPassword,
            [FromQuery] string newPassword) {

            switch (await accountService.EditPassword("", originalPassword, newPassword)) {
                case AccountService.EditPasswordResult.Success:
                    return NoContent();
                case AccountService.EditPasswordResult.Failure:
                    return BadRequest();
                case AccountService.EditPasswordResult.AccountNotFound:
                    return NotFound();
                default: throw new System.Exception("Unreachable!");
            }
        }

    }
}
