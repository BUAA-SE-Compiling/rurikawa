using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Mime;
using System.Text.Json;
using System.Text.Unicode;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using MicroKnights.IO.Streams;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.WebUtilities;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using SharpCompress.Readers;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/tests/")]
    public class TestSuiteController : ControllerBase {
        private readonly ILogger<JudgerApiController> logger;
        private readonly RurikawaDb db;
        private readonly DbService dbService;
        private readonly SingleBucketFileStorageService fs;
        private readonly JsonSerializerOptions? jsonOptions;
        private readonly RurikawaCacheService cacheService;
        public static readonly string TestSuiteBaseDir = "test/";

        public TestSuiteController(
            ILogger<JudgerApiController> logger,
            RurikawaDb db,
            DbService dbService,
            SingleBucketFileStorageService fs,
            JsonSerializerOptions? jsonOptions,
            RurikawaCacheService cacheService) {
            this.logger = logger;
            this.db = db;
            this.dbService = dbService;
            this.fs = fs;
            this.jsonOptions = jsonOptions;
            this.cacheService = cacheService;
        }

        /// <summary>
        /// Gets a list of test suites
        /// </summary>
        /// <returns></returns>
        [HttpGet()]
        public async Task<ActionResult<List<TestSuite>>> QueryTestSuites(
            [FromQuery] FlowSnake startId = new FlowSnake(),
            [FromQuery] int take = 20,
            [FromQuery] bool asc = false
        ) {
            FlowSnake? startId_ = startId;
            if (startId == FlowSnake.MinValue) startId_ = null;
            return await dbService.GetTestSuites(startId_, take, asc);
        }

        /// <summary>
        /// Gets a test suite by its id
        /// </summary>
        /// <param name="id"></param>
        /// <returns>the test suite</returns>
        [HttpGet("{id}")]
        [ProducesErrorResponseType(typeof(void))]
        public async Task<ActionResult<TestSuite>> GetTestSuite(
            [FromRoute] FlowSnake id) {
            var cached = await cacheService.GetCachedTestSuiteString(id);
            if (cached != null) {
                return new ContentResult()
                {
                    StatusCode = 200,
                    Content = cached,
                    ContentType = "application/json"
                };
            }
            var res = await db.TestSuites.Where(t => t.Id == id).SingleOrDefaultAsync();
            if (res == null) return NotFound();
            else {
                await cacheService.SetTestSuite(res);
                return res;
            }
        }

        [HttpGet]
        [Route("{suiteId}/jobs")]
        [Authorize("user")]
        public async Task<IList<Job>> GetJobsFromSuite(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] FlowSnake startId = new FlowSnake(),
            [FromQuery] int take = 20,
            [FromQuery] bool asc = false) {
            FlowSnake? startId_ = startId;
            if (startId_ == FlowSnake.MinValue) startId_ = null;
            var username = AuthHelper.ExtractUsername(HttpContext.User);
            return await dbService.GetJobs(
                startId: startId_,
                take: take,
                asc: asc,
                bySuite: suiteId,
                byUsername: username);
        }

        [HttpPost("{suiteId}/visibility")]
        public async Task<ActionResult> SetTestSuiteVisibility(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] bool visible
        ) {
            var original = await db.TestSuites.Where(t => t.Id == suiteId).SingleOrDefaultAsync();
            if (original == null) { return NotFound(new ErrorResponse("no_such_suite")); }

            original.IsPublic = visible;
            await db.SaveChangesAsync();

            await cacheService.PurgeSuite(suiteId);

            return NoContent();
        }

        [HttpPut("{suiteId}")]
        public async Task<ActionResult> PatchTestSuite(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] TestSuite.TestSuitePatch patch
        ) {
            var original = await db.TestSuites.Where(t => t.Id == suiteId).SingleOrDefaultAsync();
            if (original == null) { return NotFound(new ErrorResponse("no_such_suite")); }

            original.Patch(patch);
            await db.SaveChangesAsync();

            await cacheService.PurgeSuite(suiteId);

            return NoContent();
        }

        /// <summary>
        /// Accepts an uploaded file as a test suite archive, parse the test 
        /// spec inside this archive, saves this test suite spec into database,
        /// and returns it as a base for client to edit.
        /// </summary>
        /// <param name="filename">the name of this file, required</param>
        /// <param name="suiteId">the id of the test suite, required</param>
        /// <param name="replaceDescription">
        /// whether to replace description, defaults to true</param>
        /// <returns>Test suite spec</returns>
        [HttpPut("{suiteId}/file")]
        [Authorize("admin")]
        public async Task<IActionResult> ReplaceTestSuiteFile(
            [FromRoute] FlowSnake suiteId,
            [FromQuery] string filename,
            [FromQuery] bool replaceDescription = true
        ) {
            var original = await db.TestSuites.Where(t => t.Id == suiteId).SingleOrDefaultAsync();
            if (original == null) { return NotFound(new ErrorResponse("no_such_suite")); }

            if (!Request.ContentLength.HasValue) {
                return BadRequest("Content length must be present in header");
            } else if (Request.ContentLength == 0) {
                return BadRequest("The request has an empty body!");
            }
            long contentLength = Request.ContentLength.Value;
            var id = FlowSnake.Generate();
            var newFilename = TestSuite.FormatFileName(filename, id);

            TestSuite newSuite;
            logger.LogInformation("Begin uploading");
            try {
                newSuite = await UploadAndParseTestSuite(
                    Request.Body,
                    contentLength,
                    id,
                    newFilename);
                newSuite.Id = id;
            } catch (EndOfStreamException e) {
                return BadRequest(e.Message);
            }
            logger.LogInformation("End uploading");

            original.Patch(newSuite, replaceDescription);
            await db.SaveChangesAsync();
            logger.LogInformation("DB updated");

            await cacheService.PurgeSuite(suiteId);

            return Ok(original);
        }

        [HttpDelete("{suiteId}")]
        [Authorize("admin")]
        public async Task<ActionResult> DeleteTestSuite([FromRoute] FlowSnake suiteId) {
            if (!await db.TestSuites.Where(suite => suite.Id == suiteId).AnyAsync()) {
                return NotFound();
            }
            await dbService.RemoveTestSuiteCascade(suiteId);
            return NoContent();
        }

        /// <summary>
        /// Accepts an uploaded file as a test suite archive, parse the test 
        /// spec inside this archive, saves this test suite spec into database,
        /// and returns it as a base for client to edit.
        /// </summary>
        /// <param name="filename">the name of this file, required</param>
        /// <returns>Test suite spec</returns>
        [HttpPost]
        [Authorize("admin")]
        public async Task<IActionResult> PostNewTestSuite(
            [FromQuery] string filename
            ) {
            if (!Request.ContentLength.HasValue) {
                return BadRequest("Content length must be present in header");
            } else if (Request.ContentLength == 0) {
                return BadRequest("The request has an empty body!");
            }
            long contentLength = Request.ContentLength.Value;
            var id = FlowSnake.Generate();
            var newFilename = TestSuite.FormatFileName(filename, id);
            TestSuite testSuite;
            logger.LogInformation("Begin uploading");
            try {
                testSuite = await UploadAndParseTestSuite(
                    Request.Body,
                    contentLength,
                    id,
                    newFilename);
                testSuite.Id = id;
            } catch (EndOfStreamException e) {
                return BadRequest(e.Message);
            } catch (JsonException e) {
                return BadRequest(e.ToString());
            }
            logger.LogInformation("End uploading");
            await db.TestSuites.AddAsync(testSuite);
            await db.SaveChangesAsync();
            logger.LogInformation("DB updated");
            return Ok(testSuite);
        }

        /// <summary>
        /// For an uploaded test suite, parse its contents and upload it to s3 
        /// bucket at the same time. Closes stream after all things are done.
        /// </summary>
        /// <returns></returns>
        async Task<TestSuite> UploadAndParseTestSuite(
            Stream fileStream,
            long len,
            FlowSnake id, string filename) {
            var baseStream = new ReadableSplitStream(fileStream);
            using (var split1 = baseStream.GetForwardReadOnlyStream())
            using (var split2 = baseStream.GetForwardReadOnlyStream()) {
                logger.LogInformation("Splitting streams");
                await baseStream.StartReadAhead();

                var res = await Task.WhenAll(
                    UploadTestSuiteWrapped(split1, filename, len),
                    Task.Run(() => ParseTestSuiteWrapped(split2, id))
                );

                logger.LogInformation("Finished");
                var addr = (string)res[0];
                var suite = (TestSuite)res[1];

                suite.PackageFileId = addr;
                return suite;
            }
        }

        async Task<object> UploadTestSuiteWrapped(
            Stream fileStream,
            string filename,
            long len)
            => await UploadTestSuite(fileStream, filename, len);

        async Task<string> UploadTestSuite(Stream fileStream, string filename, long len) {
            await fs.UploadFile(TestSuiteBaseDir + filename, fileStream, len);
            return TestSuiteBaseDir + filename;
        }

        async Task<object> ParseTestSuiteWrapped(Stream fileStream, FlowSnake id)
            => await ParseTestSuite(fileStream, id);

        async Task<TestSuite> ParseTestSuite(Stream fileStream, FlowSnake id) {
            logger.LogInformation("Parse started");
            var opt = new ReaderOptions
            {
                LeaveStreamOpen = true,
            };
            string desc = "";
            TestSuite? suite = null;
            using (var reader = ReaderFactory.Open(fileStream, opt)) {
                logger.LogInformation("Stream started");
                while (reader.MoveToNextEntry()) {
                    var entry = reader.Entry;
                    logger.LogInformation("Entry: {0}", entry.Key);
                    // if (entry.IsDirectory) continue;
                    using var file = reader.OpenEntryStream();
                    switch (entry.Key.ToLower()) {
                        case "testconf.json":
                            logger.LogInformation("testconf!");
                            suite = await ParseTestSuiteJson(file);
                            break;
                        case "readme.md":
                            logger.LogInformation("readme!");
                            desc = await ParseTestSuiteDesc(file);
                            break;
                        default:
                            break;
                    }
                }
            }
            if (suite == null) {
                logger.LogInformation("Cannot find test configuration");
                throw new EndOfStreamException("Cannot find test configuration");
            }
            suite.Description = desc;
            logger.LogInformation("Parse succeeded");
            return suite;
        }

        async Task<TestSuite> ParseTestSuiteJson(Stream file) {
            return await JsonSerializer.DeserializeAsync<TestSuite>(file, jsonOptions);
        }

        async Task<string> ParseTestSuiteDesc(Stream file) {
            var reader = new StreamReader(file);
            return await reader.ReadToEndAsync();
        }
    }
}
