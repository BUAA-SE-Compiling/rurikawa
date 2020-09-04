using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/judger/")]
    public class JudgerApiController : ControllerBase {
        private readonly ILogger<JudgerApiController> _logger;
        private readonly JudgerService judgerService;
        private readonly SingleBucketFileStorageService fs;

        public JudgerApiController(
            ILogger<JudgerApiController> logger,
            JudgerService judgerService,
            SingleBucketFileStorageService fs) {
            _logger = logger;
            this.judgerService = judgerService;
            this.fs = fs;
        }

#pragma warning disable CS8618
        public class JudgerRegisterMessage {
            public string Token { get; set; }
            public string? AlternateName { get; set; }
            public List<string>? Tags { get; set; }
        }
#pragma warning restore

        [Route("register")]
        public async Task<IActionResult> RegisterJudgerSelf([FromBody] JudgerRegisterMessage msg) {
            try {
                var result = await judgerService.RegisterJudger(msg.Token, msg.AlternateName, msg.Tags);
                return Ok(result.Id);
            } catch (KeyNotFoundException) {
                return BadRequest("No such token was found");
            }
        }

        [Authorize("judger")]
        [Route("upload")]
        public async Task<IActionResult> UploadJudgerResult(
            [FromQuery] FlowSnake jobId,
            [FromQuery] string testId) {
            if (Request.ContentLength == null)
                return BadRequest("ContentLength must be specified!");

            var filename = $"results/{jobId}/{testId}.json";
            await fs.UploadFile(filename, Request.Body, Request.ContentLength.Value, true);
            return Ok(filename);
        }
    }
}
