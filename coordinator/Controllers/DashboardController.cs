using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Unicode;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.WebUtilities;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using SharpCompress.Readers;

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
            JsonSerializerOptions? jsonOptions) {
            this.logger = logger;
            this.db = db;
            this.jsonOptions = jsonOptions;
        }

        public class Dashboard {
            public TestSuite Suite { get; set; }
            public Job? Job { get; set; }
        }

        [HttpGet]
        [Authorize("user")]
        public async Task<IList<Dashboard>> GetDashboard(
            [FromQuery] int limit = 10,
            [FromQuery] FlowSnake startId = new FlowSnake()) {
            var username = AuthHelper.ExtractUsername(HttpContext.User);
            if (startId == FlowSnake.MinValue) startId = FlowSnake.MaxValue;

            var suites = await db.TestSuites.AsQueryable()
                .Where(suite => (!suite.StartTime.HasValue || suite.StartTime.Value <= DateTimeOffset.Now) && suite.IsPublic)
                .Where(suite => suite.Id < startId)
                .OrderByDescending(t => t.Id)
                .Take(limit)
                .AsNoTracking()
                .ToListAsync();

            var suiteIds = suites.Select(s => s.Id).ToList();

            var jobs = await db.Jobs.FromSqlInterpolated(
                $@"
                select
                    distinct on (test_suite)
                    * from (
                        select * from jobs
                        order by id desc
                    ) as sub
                where account = {username}
                "
            ).Where(j => suiteIds.Contains(j.TestSuite)).AsNoTracking().ToListAsync();

            var dashboard = suites
                .GroupJoin(
                    jobs,
                    s => s.Id,
                    j => j.TestSuite,
                    (s, j) => new Dashboard { Suite = s, Job = j.FirstOrDefault() })
                .ToList();

            return dashboard;
        }
    }
}
