using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    // [Authorize("judger")]
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

        [AllowAnonymous]
        [Route("register")]
        public async Task<IActionResult> RegisterJudgerSelf([FromBody] JudgerRegisterMessage msg) {
            try {
                var result = await judgerService.RegisterJudger(msg.Token, msg.AlternateName, msg.Tags);
                return Ok(result.Id);
            } catch (KeyNotFoundException) {
                return BadRequest("No such token was found");
            }
        }

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

        [Route("download-suite/{suite_}")]
        public async Task<IActionResult> DownloadSuite(
             string suite_,
            [FromServices] RurikawaDb db) {
            FlowSnake suite;
            try { suite = new FlowSnake(suite_); } catch {
                return BadRequest("Invalid suite id");
            }

            var test_suite = await db.TestSuites.SingleOrDefaultAsync(s => s.Id == suite);

            return Redirect($"/api/v1/file/{test_suite.PackageFileId}");
        }
    }
}
