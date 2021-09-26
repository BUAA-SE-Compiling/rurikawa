using System;
using System.Collections.Generic;
using System.Security.Policy;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Karenia.Rurikawa.Coordinator.Controllers.Admin {
    [ApiController]
    [Route("api/v1/admin/tests/")]
    [Authorize("admin", AuthenticationSchemes = JwtBearerDefaults.AuthenticationScheme + "," + "token")]
    public class AdminTestController : ControllerBase {
        [HttpGet]
        [Route("{suiteId}/jobs")]
        public async Task<IList<Job>> GetJobsFromSuite(
            [FromServices] DbService dbService,
            [FromRoute] FlowSnake suiteId,
            [FromQuery] FlowSnake? startId = null,
            [FromQuery] int take = 20,
            [FromQuery] string? user = null,
            [FromQuery] bool asc = false) {
            FlowSnake? startId_ = startId;
            if (startId_ == FlowSnake.MinValue) startId_ = null;
            return await dbService.GetJobs(
                startId: startId_,
                take: take,
                asc: asc,
                bySuite: suiteId,
                byUsername: user);
        }
    }
}
