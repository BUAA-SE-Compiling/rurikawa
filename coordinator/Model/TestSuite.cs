using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using System.Text.RegularExpressions;
using Dahomey.Json.Attributes;
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
        /// The displayed title of this test suite
        /// </summary>
        public string Title { get; set; }

        /// <summary>
        /// The description of this test suite, written in Markdown
        /// </summary>
        public string Description { get; set; }

        /// <summary>
        /// Tags of this test suite, e.g. which kind of judger should it run in.
        /// </summary>
        public List<string>? Tags { get; set; }

        public string PackageFileId { get; set; }

        public bool IsPublic { get; set; }

        public DateTimeOffset? StartTime { get; set; }

        public DateTimeOffset? EndTime { get; set; }

        public int? TimeLimit { get; set; }

        public int? MemoryLimit { get; set; }

        /// <summary>
        /// All tests inside test suite, grouped by user-defined keys.
        /// <br/>
        /// Tests that do not belong to any group should be put in a
        /// "default" group.
        /// </summary>
        [Column(TypeName = "jsonb")]
        public Dictionary<string, List<TestCaseDefinition>> TestGroups { get; set; }

        /// <summary>
        /// Name of the default group in test groups
        /// </summary>
        public static readonly string DEFAULT_GROUP_NAME = "default";

        static readonly Regex extRegex =
            new Regex(@"^(?:(?<filename>.+?)\.)?(?<ext>(?:tar.)?[^.]+)$");

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

        public void Patch(TestSuite other, bool patchDescription = true) {
            this.Name = other.Name;
            this.MemoryLimit = other.MemoryLimit;
            this.TimeLimit = other.TimeLimit;
            this.Title = other.Title;
            this.TestGroups = other.TestGroups;
            this.Tags = other.Tags;
            this.StartTime = other.StartTime;
            this.EndTime = other.EndTime;
            this.PackageFileId = other.PackageFileId;
            if (patchDescription) this.Description = other.Description;
        }

        public void Patch(TestSuitePatch patch) {
            this.Name = patch.Name;
            this.Title = patch.Title;
            this.Description = patch.Description;
            this.Tags = patch.Tags;
            this.IsPublic = patch.IsPublic;
            this.StartTime = patch.StartTime;
            this.EndTime = patch.EndTime;
            this.MemoryLimit = patch.MemoryLimit;
            this.TimeLimit = patch.TimeLimit;
        }

        /// <summary>
        /// A patch class to change various data of a test suite
        /// </summary>
        public class TestSuitePatch {
            public string Name { get; set; }

            public string Title { get; set; }

            public string Description { get; set; }

            public List<string>? Tags { get; set; }

            public bool IsPublic { get; set; }

            public DateTimeOffset? StartTime { get; set; }

            public DateTimeOffset? EndTime { get; set; }

            public int? TimeLimit { get; set; }

            public int? MemoryLimit { get; set; }
        }
    }

    /*
    /// The definition of a test case
    #[derive(Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct TestCaseDefinition {
        pub name: String,
        pub should_fail: bool,
        pub has_out: bool,
    }
    */
    /// <summary>
    /// The definition of a test case
    /// </summary>
    public class TestCaseDefinition {
        public string Name { get; set; }
        public bool HasOut { get; set; }
        public bool ShouldFail { get; set; }
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
