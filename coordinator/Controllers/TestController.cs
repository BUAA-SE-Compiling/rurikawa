using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Unicode;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Test;
using MicroKnights.IO.Streams;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using SharpCompress.Readers;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/tests/")]
    public class TestApiController : ControllerBase {
        private readonly ILogger<JudgerApiController> _logger;
        private readonly RurikawaDb db;
        private readonly SingleBucketFileStorageService fs;
        private readonly JsonSerializerOptions? jsonOptions;
        public static readonly string TestSuiteBaseDir = "test/";

        public TestApiController(
            ILogger<JudgerApiController> logger,
            RurikawaDb db,
            SingleBucketFileStorageService fs,
            JsonSerializerOptions? jsonOptions) {
            _logger = logger;
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
            [FromHeader] string filename,
            [FromHeader] long contentLength
            ) {
            var id = FlowSnake.Generate();
            var newFilename = TestSuite.FormatFileName(filename, id);
            TestSuite testSuite;
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
            await db.TestSuites.AddAsync(testSuite);
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
            using var baseStream = new ReadableSplitStream(fileStream);
            using var split1 = baseStream.GetForwardReadOnlyStream();
            using var split2 = baseStream.GetForwardReadOnlyStream();

            var res = await Task.WhenAll(
                UploadTestSuiteWrapped(split1, filename, len),
                Task.Run(() => ParseTestSuiteWrapped(split2, id))
            );
            var addr = (string)res[0];
            var suite = (TestSuite)res[1];

            suite.PackageFileId = addr;
            return suite;
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
            var opt = new ReaderOptions
            {
                LeaveStreamOpen = true
            };
            var reader = ReaderFactory.Open(fileStream);

            string desc = "";
            TestSuite? suite = null;

            while (reader.MoveToNextEntry()) {
                var entry = reader.Entry;
                if (entry.IsDirectory) continue;
                switch (entry.Key.ToLower()) {
                    case "test.json":
                        suite = await ParseTestSuiteJson(reader.OpenEntryStream());
                        break;
                    case "readme.md":
                        desc = await ParseTestSuiteDesc(reader.OpenEntryStream());
                        break;
                    default:
                        break;
                }
            }
            fileStream.Close();
            if (suite == null) {
                throw new EndOfStreamException("No test suite were found after reading the whole package!");
            }
            suite.Description = desc;
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
