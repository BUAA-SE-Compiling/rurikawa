using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/judger/")]
    public class JudgerApiController : ControllerBase {
        private readonly ILogger<JudgerApiController> _logger;

        public JudgerApiController(ILogger<JudgerApiController> logger) {
            _logger = logger;
        }


    }
}
