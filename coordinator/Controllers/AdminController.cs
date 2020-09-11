using System.Collections.Generic;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/admin")]
    public class AdminController : ControllerBase {
        private readonly DbService dbService;

        public AdminController(DbService dbService) {
            this.dbService = dbService;
        }


        [HttpGet]
        [Route("suite/{id}/jobs")]
        [Authorize("user")]
        public async Task<IList<Job>> GetJobsFromSuite(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] FlowSnake startId = new FlowSnake(),
            [FromQuery] int take = 20,
            [FromQuery] bool asc = false) {
            var username = AuthHelper.ExtractUsername(HttpContext.User);
            return await dbService.GetJobs(
                startId: startId,
                take: take,
                asc: asc,
                bySuite: suiteId);
        }
    }
}
