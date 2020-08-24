using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using Karenia.Rurikawa.Helpers;

namespace Karenia.Rurikawa.Models.Test {
    public class TestSuite {
        public string Name { get; set; }

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
