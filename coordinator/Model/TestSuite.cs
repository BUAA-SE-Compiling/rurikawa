using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using Karenia.Rurikawa.Helpers;

#pragma warning disable CS8618  
namespace Karenia.Rurikawa.Models.Test {
    public class TestSuite {
        /// <summary>
        /// The name of this test suite
        /// </summary>
        public string Name { get; set; }

        /// <summary>
        /// The description of this test suite, written in Markdown
        /// </summary>
        public string Description { get; set; }

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
