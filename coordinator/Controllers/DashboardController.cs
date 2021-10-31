using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/dashboard/")]
    public class DashboardController : ControllerBase {
        private readonly ILogger<JudgerApiController> logger;
        private readonly RurikawaDb db;
        private readonly JsonSerializerOptions? jsonOptions;

        public DashboardController(
            ILogger<JudgerApiController> logger,
            RurikawaDb db,
            SingleBucketFileStorageService fs,
            JsonSerializerOptions? jsonOptions
        ) {
            this.logger = logger;
            this.db = db;
            this.jsonOptions = jsonOptions;
        }

        // Disable "Consider declaring the property as nullable" warning.
#pragma warning disable CS8618
        public class Dashboard {
            public PartialTestSuite Suite { get; set; }
            public Job? Job { get; set; }
        }

        public class PartialTestSuite {
            public FlowSnake Id { get; set; }
            public string Title { get; set; }
        }
#pragma warning restore CS8618

        [HttpGet]
        [Authorize("user")]
        public async Task<IList<Dashboard>> GetDashboard(
            [FromQuery] int limit = 10,
            [FromQuery] FlowSnake startId = new FlowSnake()
        ) {
            var username = AuthHelper.ExtractUsername(HttpContext.User);
            if (startId == FlowSnake.MinValue) {
                startId = FlowSnake.MaxValue;
            }
            var now = DateTimeOffset.Now;
            var suites = await db.TestSuites.AsQueryable()
                .Where(suite => (suite.StartTime == null || suite.StartTime <= now) && suite.IsPublic)
                .Where(suite => suite.Id < startId)
                .OrderByDescending(t => t.Id)
                .Take(limit)
                .AsNoTracking()
                .Select(x => new PartialTestSuite { Id = x.Id, Title = x.Title })
                .ToListAsync();

            var suiteIds = suites.Select(s => s.Id).ToList();

            var jobs = await db.Jobs.FromSqlInterpolated(
                $@"
                select distinct on (test_suite)
                    * 
                from 
                    jobs
                where 
                    account = {username}
                order by
                    test_suite, id desc
                "
            )
                .Where(j => suiteIds.Contains(j.TestSuite))
                .AsNoTracking()
                .ToListAsync();

            var dashboard = suites
                .GroupJoin(
                    jobs,
                    s => s.Id,
                    j => j.TestSuite,
                    (s, j) => new Dashboard { Suite = s, Job = j.FirstOrDefault() }
                )
                .ToList();

            return dashboard;
        }
    }
}
