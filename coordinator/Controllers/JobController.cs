using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/job/")]
    [Authorize()]
    public class JobController : ControllerBase {
        public JobController(ILogger<JobController> logger, RurikawaDb db) {
            this.logger = logger;
            this.db = db;
        }

        private readonly ILogger<JobController> logger;
        private readonly RurikawaDb db;


        /// <summary>
        /// GETs a job by its identifier
        /// </summary>
        /// <param name="id"></param>
        /// <returns></returns>
        [HttpGet]
        [Route("{id}")]
        public Job GetJob(ulong id) {
            throw new NotImplementedException();
        }

        /// <summary>
        /// GETs a job by its identifier (stringified version)
        /// </summary>
        /// <param name="id"></param>
        /// <returns></returns>
        [HttpGet]
        [Route("{id}")]
        public Job GetJob(string id) {
            throw new NotImplementedException();
        }

        /// <summary>
        /// PUTs a new job
        /// </summary>
        /// <param name="job"></param>
        /// <returns></returns>
        [HttpPost]
        [Authorize(Roles = "admin")]
        public string NewJob(Job job) {
            throw new NotImplementedException();
        }
    }
}
