﻿using System;
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
using Karenia.Rurikawa.Models.Test;
using MicroKnights.IO.Streams;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.WebUtilities;
using Microsoft.Extensions.Logging;
using SharpCompress.Readers;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/tests/")]
    public class TestApiController : ControllerBase {
        private readonly ILogger<JudgerApiController> logger;
        private readonly RurikawaDb db;
        private readonly SingleBucketFileStorageService fs;
        private readonly JsonSerializerOptions? jsonOptions;
        public static readonly string TestSuiteBaseDir = "test/";

        public TestApiController(
            ILogger<JudgerApiController> logger,
            RurikawaDb db,
            SingleBucketFileStorageService fs,
            JsonSerializerOptions? jsonOptions) {
            this.logger = logger;
            this.db = db;
            this.fs = fs;
            this.jsonOptions = jsonOptions;
        }

        /// <summary>
        /// Accepts an uploaded file as a test suite archive, parse the test 
        /// spec inside this archive, saves this test suite spec into database,
        /// and returns it as a base for client to edit.
        /// </summary>
        /// <param name="contentLength"></param>
        /// <returns>Test suite spec</returns>
        [HttpPost]
        public async Task<IActionResult> PostNewTestSuite(
            [FromHeader] string filename
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
            }
            logger.LogInformation("End uploading");
            await db.TestSuites.AddAsync(testSuite);
            logger.LogInformation("DB updated");
            return Ok(testSuite);
            // throw new NotImplementedException();
        }

        /// <summary>
        /// For an uploaded test suite, parse its contents and upload it to s3 
        /// bucket at the same time. Closes stream after all things are done.
        /// </summary>
        /// <param name="fileStream"></param>
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
                logger.LogInformation("Parse failed");
                throw new EndOfStreamException("No test suite were found after reading the whole package!");
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