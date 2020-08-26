using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using System.Text.RegularExpressions;
using Karenia.Rurikawa.Helpers;

#pragma warning disable CS8618  
namespace Karenia.Rurikawa.Models.Test {
    public class TestSuite {
        /// <summary>
        /// The unique identifier of this test suite
        /// </summary>
        public FlowSnake Id { get; set; }

        /// <summary>
        /// The name of this test suite
        /// </summary>
        public string Name { get; set; }

        /// <summary>
        /// The description of this test suite, written in Markdown
        /// </summary>
        public string Description { get; set; }

        /// <summary>
        /// Tags of this test suite, e.g. which kind of judger should it run in.
        /// </summary>
        public List<string>? Tags { get; set; }

        public string PackageFileId { get; set; }

        public int? TimeLimit { get; set; }

        public int? MemoryLimit { get; set; }

        /// <summary>
        /// All tests inside test suite, grouped by user-defined keys.
        /// <br/>
        /// Tests that do not belong to any group should be put in a
        /// "default" group.
        /// </summary>
        [Column(TypeName = "jsonb")]
        public Dictionary<string, List<string>> TestGroups { get; set; }

        /// <summary>
        /// Name of the default group in test groups
        /// </summary>
        public static readonly string DEFAULT_GROUP_NAME = "default";

        static readonly Regex extRegex =
            new Regex(@"^(?:(?<filename>.+?)\.)?(?<ext>(?:tar.)?[^.]+)$");

        public TestSuite(FlowSnake id, string name, string description, List<string>? tags, string packageFileId, int? timeLimit, int? memoryLimit, Dictionary<string, List<string>> testGroups) {
            Id = id;
            Name = name;
            Description = description;
            Tags = tags;
            PackageFileId = packageFileId;
            TimeLimit = timeLimit;
            MemoryLimit = memoryLimit;
            TestGroups = testGroups;
        }

        // Constructor for suites with no id specified yet
        public TestSuite(string name, string description, List<string>? tags, string packageFileId, int? timeLimit, int? memoryLimit, Dictionary<string, List<string>> testGroups) {
            Id = new FlowSnake(0);
            Name = name;
            Description = description;
            Tags = tags;
            PackageFileId = packageFileId;
            TimeLimit = timeLimit;
            MemoryLimit = memoryLimit;
            TestGroups = testGroups;
        }

        public static string FormatFileName(string orig, FlowSnake id) {
            var match = extRegex.Match(orig);
            if (match.Success) {
                var filename = match.Groups["filename"].Value;
                var extension = match.Groups["ext"].Value;
                return $"{filename}.{id}.{extension}";
            } else {
                return $"{id}.{orig}";
            }
        }
    }

    public class TestJob {

    }

    public enum TestResultKind {
        Accepted = 0,
        WrongAnswer = 1,
        RuntimeError = 2,
        PipelineFailed = 3,
        TimeLimitExceeded = 4,
        MemoryLimitExceeded = 5,
        NotRunned = -1,
        Waiting = -2,
        Running = -3,
        OtherError = -100,
    }

    public enum JobStage {
        Queued,
        Dispatched,
        Fetching,
        Compiling,
        Running,
        Finished,
        Cancelled,
        Skipped,
    }

    public class TestResult {
        public TestResultKind Kind { get; set; }
        public string? ResultFileId { get; set; }
    }
}
