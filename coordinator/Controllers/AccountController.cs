using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Claims;
using System.Text.Json;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
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
        [Route("register")]
        public async Task<IActionResult> RegisterAccount(
            [FromBody] AccountInfo msg
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

        private List<string> ParseScope(string scope) {
            return scope.Split(",").Select(x => x.Trim()).ToList();
        }

        internal class InvalidLoginInformationException : System.Exception {
            public InvalidLoginInformationException(string message) : base(message) { }
        }
        internal class NotEnoughInformationException : System.Exception {
            public NotEnoughInformationException(string message) : base(message) { }
        }

        [Route("login")]
        public async Task<IActionResult> LoginUser([FromBody] OAuth2Request msg) {
            try {
                switch (msg.GrantType) {
                    case "password":
                        return Ok(await LoginUsingPassword(msg));
                    case "refresh_token":
                        return Ok(await LoginUsingRefreshToken(msg));
                    default:
                        return BadRequest(new ErrorResponse("invalid_grant_type"));
                }
            } catch (InvalidLoginInformationException e) {
                return BadRequest(new ErrorResponse("invalid_login_info", e.Message));
            } catch (NotEnoughInformationException e) {
                return BadRequest(new ErrorResponse("not_enough_login_info", e.Message));
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

            return new OAuth2Response
            {
                AccessToken = accessToken,
                RefreshToken = refreshToken,
                ExpiresIn = accessTokenExpireTime.ToUnixTimeSeconds(),
                Scope = strScope
            };
        }

        private async Task<OAuth2Response> LoginUsingRefreshToken(OAuth2Request msg) {
            var refreshToken = ((JsonElement?)msg.ExtraInfo.GetValueOrDefault("refresh_token"))?.GetString(); ;
            if (refreshToken == null)
                throw new NotEnoughInformationException("Please provide refresh_token!");
            var tokenEntry = await accountService.GetRefreshToken(refreshToken);
            if (tokenEntry == null)
                throw new InvalidLoginInformationException("Invalid refresh token");
            var account = await accountService.GetAccount(tokenEntry.Username)!;
            return await GenerateOAuth2Response(msg.Scope, account);
        }

        [Route("edit/password")]
        [Authorize()]
        public async Task<IActionResult> EditPassword(
            [FromQuery] string originalPassword,
            [FromQuery] string newPassword) {
            var username = AuthHelper.ExtractUsername(User)!;
            switch (await accountService.EditPassword(username, originalPassword, newPassword)) {
                case AccountService.EditPasswordResult.Success:
                    return NoContent();
                case AccountService.EditPasswordResult.Failure:
                    return BadRequest();
                case AccountService.EditPasswordResult.AccountNotFound:
                    return NotFound();
                default: throw new System.Exception("Unreachable!");
            }
        }


        [Route("test")]
        [Authorize("admin")]
        public async Task Test() {
            foreach (var claim in User.Claims) {
                Console.WriteLine($"{claim.Type},{claim.Value}");
            }
            Console.WriteLine(User.Identity.Name);

        }
    }
}
