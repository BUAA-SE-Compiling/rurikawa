using System;
using System.Collections.Generic;
using System.Security.Policy;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Karenia.Rurikawa.Coordinator.Controllers.Admin {
    [ApiController]
    [Route("api/v1/admin/judger")]
    [Authorize("admin", AuthenticationSchemes = JwtBearerDefaults.AuthenticationScheme + "," + "token")]
    public class AdminJudgerController : ControllerBase {
#pragma warning disable CS8618
        public class CreateJudgerTokenRequest {
            public DateTimeOffset? ExpireAt { get; set; }
            public bool IsSingleUse { get; set; }
            public List<string> Tags { get; set; }
        }
#pragma warning restore CS8618

        /// <summary>
        /// Generate a new register token for judger
        /// </summary>
        /// <param name="accountService"></param>
        /// <param name="req">the generation request</param>
        /// <returns></returns>
        [HttpPost("register-token")]
        public async Task<string> GenerateJudgerRegisterToken(
            [FromServices] AccountService accountService,
            [FromBody] CreateJudgerTokenRequest req
            ) {
            return await accountService.GenerateAndSaveJudgerToken(req.ExpireAt, req.IsSingleUse, req.Tags);
        }

        /// <summary>
        /// Search <see cref="JudgerTokenEntry"/>s by their properties.
        /// </summary>
        /// <param name="tags">Token tags. A token must contain all tags to appear in the results.</param>
        /// <param name="judgerService"/>
        /// <param name="expired">Whether the token is already expired</param>
        /// <param name="start">The starting token to be queried</param>
        /// <param name="take">The number of tokens to return</param>
        /// <returns></returns>
        [HttpGet("register-token")]
        public async Task<ActionResult<List<JudgerTokenEntry>>> QueryJudgerRegisterToken(
            [FromServices] JudgerService judgerService,
            [FromQuery] List<string> tags,
            [FromQuery] bool? expired,
            [FromQuery] string start = "",
            [FromQuery] int take = 50
        ) {
            return await judgerService.QueryJudgerRegisterToken(tags, expired, start, take);
        }


        /// <summary>
        /// Search <see cref="JudgerEntry"/>s by tags.
        /// 
        /// <para>
        ///     If a future version adds more properties to judgers, they should also be 
        ///     able to be queried by this method.
        /// </para>
        /// </summary>
        /// <param name="judgerService"/>
        /// <param name="tags">Judger tags. A judger should contain all tags specified to apperar in the result.</param>
        /// <param name="start">The starting judger id to be queried</param>
        /// <param name="take">The number of judgers to return</param>
        /// <returns></returns>
        [HttpGet("")]
        public async Task<ActionResult<List<JudgerEntry>>> QueryJudger(
            [FromServices] JudgerService judgerService,
            [FromQuery] List<string> tags,
            [FromQuery] string start = "",
            [FromQuery] int take = 50
        ) {
            return await judgerService.QueryJudger(tags, start, take);
        }

        [HttpDelete("{id}")]
        public async Task<ActionResult> DeleteJudger(
            [FromServices] JudgerService judgerService,
            [FromRoute] string id) {
            var deletedRows = await judgerService.DeleteJudger(id);
            // TODO: Disconnect from that judger
            if (deletedRows == 0) {
                return NotFound();
            } else {
                return NoContent();
            }
        }

        [HttpDelete("register-token/{id}")]
        public async Task<ActionResult> DeleteJudgerToken(
            [FromServices] JudgerService judgerService,
            [FromRoute] string id) {
            var deletedRows = await judgerService.DeleteJudgerToken(id);
            if (deletedRows == 0) {
                return NotFound();
            } else {
                return NoContent();
            }
        }
    }
}
