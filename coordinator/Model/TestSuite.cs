using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
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
        public Dictionary<string, List<string>> TestGroups { get; set; }

        /// <summary>
        /// Name of the default group in test groups
        /// </summary>
        public static readonly string DEFAULT_GROUP_NAME = "default";
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
