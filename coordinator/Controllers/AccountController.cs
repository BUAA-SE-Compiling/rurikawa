using System;
using System.Buffers.Text;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices.ComTypes;
using System.Security.Claims;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Auth;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
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
        public class AccountInfo {
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
        [HttpPost("register")]
        public async Task<IActionResult> RegisterAccount(
            [FromBody] AccountInfo msg,
            [FromServices] ProfileService profileService
        ) {
            try {
                await accountService.CreateAccount(msg.Username, msg.Password);
                await profileService.InitializeProfileIfNotExists(msg.Username);
                return NoContent();
            } catch (AccountService.UsernameNotUniqueException e) {
                return BadRequest(new ErrorResponse(
                    ErrorCodes.USERNAME_NOT_UNIQUE,
                    $"Username '{e.Username}' is not unique inside database"));
            } catch (AccountService.InvalidUsernameException e) {
                return BadRequest(new ErrorResponse(
                    ErrorCodes.INVALID_USERNAME,
                    $"Username '{e.Username}' must be 1 to 64 characters long, and must only contain letter, number, dash '-' and underscore '_'."
                ));
            }
        }

        private List<string> ParseScope(string scope) {
            return scope.Split(",").Select(x => x.Trim()).ToList();
        }

        internal class InvalidLoginInformationException : System.Exception {
            public InvalidLoginInformationException(string message) : base(message) { }
        }
        internal class NotEnoughInformationException : System.Exception {
            public NotEnoughInformationException(string message) : base(message) { }
        }


        /// <summary>
        /// Login with specified username/password or refresh token. <br/>
        /// </summary>
        /// <param name="msg"></param>
        /// <returns></returns>
        [HttpPost("login")]
        [ProducesResponseType(typeof(OAuth2Response), 200)]
        [ProducesResponseType(typeof(ErrorResponse), 400)]
        public async Task<IActionResult> LoginUser([FromBody] OAuth2Request msg) {
            try {
                switch (msg.GrantType) {
                    case "password":
                        return Ok(await LoginUsingPassword(msg));
                    case "refresh_token":
                        return Ok(await LoginUsingRefreshToken(msg));
                    default:
                        return BadRequest(new ErrorResponse(ErrorCodes.INVALID_GRANT_TYPE));
                }
            } catch (InvalidLoginInformationException e) {
                return BadRequest(new ErrorResponse(ErrorCodes.INVALID_LOGIN_INFO, e.Message));
            } catch (NotEnoughInformationException e) {
                return BadRequest(new ErrorResponse(ErrorCodes.NOT_ENOUGH_LOGIN_INFO, e.Message));
            }
        }

        static readonly TimeSpan JwtAccessTokenLifespan = TimeSpan.FromHours(1);
        static readonly TimeSpan RefreshTokenLifespan = TimeSpan.FromDays(30);

        private async Task<OAuth2Response> LoginUsingPassword(OAuth2Request msg) {
            var username = ((JsonElement?)msg.ExtraInfo.GetValueOrDefault("username"))?.GetString();
            var password = ((JsonElement?)msg.ExtraInfo.GetValueOrDefault("password"))?.GetString();
            if (username == null || password == null)
                throw new NotEnoughInformationException("Please provide both username and password!");
            var account = await accountService.GetAccount(username);
            if (account == null)
                throw new InvalidLoginInformationException("Username or password is wrong");
            var result = accountService.VerifyPassword(password, account.HashedPassword);
            if (!result)
                throw new InvalidLoginInformationException("Username or password is wrong");

            return await GenerateOAuth2Response(msg.Scope, account);
        }

        private async Task<OAuth2Response> GenerateOAuth2Response(string strScope, Models.Account.UserAccount account) {
            var scope = ParseScope(strScope);
            DateTimeOffset accessTokenExpireTime = DateTimeOffset.Now.Add(JwtAccessTokenLifespan);
            var accessToken = accountService.CreateNewJwtAccessToken(
                account,
                scope,
                accessTokenExpireTime);
            var refreshToken = await accountService.CreateNewRefreshToken(
                account.Username,
                accessToken,
                scope,
                DateTimeOffset.Now.Add(RefreshTokenLifespan),
                true);

            return new OAuth2Response {
                AccessToken = accessToken,
                RefreshToken = refreshToken,
                ExpiresIn = accessTokenExpireTime.ToUnixTimeSeconds(),
                Scope = strScope,
                Role = account.Kind.ToString()
            };
        }

        private async Task<OAuth2Response> LoginUsingRefreshToken(OAuth2Request msg) {
            string? refreshToken;
            try {
                refreshToken = ((JsonElement?)msg.ExtraInfo.GetValueOrDefault("refreshToken"))?.GetString();
            } catch (InvalidOperationException) {
                throw new NotEnoughInformationException("Please provide refreshToken!");
            }
            if (refreshToken == null)
                throw new NotEnoughInformationException("Please provide refreshToken!");
            var tokenEntry = await accountService.GetRefreshToken(refreshToken);
            if (tokenEntry == null)
                throw new InvalidLoginInformationException("Invalid refresh token");
            var account = await accountService.GetAccount(tokenEntry.Username)!;
            return await GenerateOAuth2Response(msg.Scope, account);
        }

        public class EditPasswordMessage {
            public string Original { get; set; }
            public string New { get; set; }
        }

        [HttpPost("edit/password")]
        [HttpPut("edit/password")]
        [Authorize()]
        public async Task<IActionResult> EditPassword(
            [FromBody] EditPasswordMessage msg) {
            var username = AuthHelper.ExtractUsername(User)!;
            switch (await accountService.EditPassword(username, msg.Original, msg.New)) {
                case AccountService.EditPasswordResult.Success:
                    return NoContent();
                case AccountService.EditPasswordResult.Failure:
                    return BadRequest();
                case AccountService.EditPasswordResult.AccountNotFound:
                    return NotFound();
                default: throw new System.Exception("Unreachable!");
            }
        }

        [HttpGet("ws-token")]
        [Authorize()]
        public ActionResult<string> GetWebsocketToken() {
            var username = AuthHelper.ExtractUsername(HttpContext.User)!;
            return accountService.CreateNewShortLivingToken(username, TimeSpan.FromMinutes(15));
        }

        [HttpPost("ws-token")]
        public ActionResult<string?> VerifyWebsocketToken([FromQuery] string token) {
            var res = accountService.VerifyShortLivingToken(token);
            if (res != null) return res;
            else return BadRequest();
        }

        [HttpPost("test")]
        public async Task Test() {
            foreach (var claim in User.Claims) {
                Console.WriteLine($"{claim.Type},{claim.Value}");
            }
            Console.WriteLine(User.Identity.Name);

        }
    }
}
