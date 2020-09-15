using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/file/")]
    public class GetFileController : ControllerBase {
        private readonly ILogger<JudgerApiController> _logger;
        private readonly SingleBucketFileStorageService fs;

        public GetFileController(
            ILogger<JudgerApiController> logger,
            SingleBucketFileStorageService fs) {
            _logger = logger;
            this.fs = fs;
        }

        [Route("{**name}")]
        public IActionResult GetFile(string name) {
            string url = fs.GetFileAddress(name);
            return RedirectPreserveMethod(url);
        }
    }
}
