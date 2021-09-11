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
            return await accountService.GenerateAndSaveNewJudgerToken(req.ExpireAt, req.IsSingleUse, req.Tags);
        }

        /// <summary>
        /// Query existing judger register tokens
        /// </summary>
        /// <param name="judgerService"></param>
        /// <param name="tags">judger tags; leave empty for none</param>
        /// <param name="expired">whether the token has expired</param>
        /// <param name="start"></param>
        /// <param name="take"></param>
        /// <returns></returns>
        [HttpGet("register-token")]
        public async Task<ActionResult<List<JudgerTokenEntry>>> GetJudgerTokenList(
            [FromServices] JudgerService judgerService,
            [FromQuery] List<string> tags,
            [FromQuery] bool? expired,
            [FromQuery] string start = "",
            [FromQuery] int take = 50
        ) {
            return await judgerService.QueryJudgerRegisterToken(tags, expired, start, take);
        }

        /// <summary>
        /// Query existing judgers
        /// </summary>
        /// <param name="judgerService"></param>
        /// <param name="tags"></param>
        /// <param name="start"></param>
        /// <param name="take"></param>
        /// <returns></returns>
        [HttpGet("")]
        public async Task<ActionResult<List<JudgerEntry>>> GetJudgerList(
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
            var result = await judgerService.DeleteJudger(id);
            // TODO: Disconnect from that judger
            if (result == 0) {
                return NotFound();
            } else {
                return NoContent();
            }
        }

        [HttpDelete("register-token/{id}")]
        public async Task<ActionResult> DeleteJudgerToken(
            [FromServices] JudgerService judgerService,
            [FromRoute] string id) {
            var result = await judgerService.DeleteToken(id);
            if (result == 0) {
                return NotFound();
            } else {
                return NoContent();
            }
        }
    }
}
