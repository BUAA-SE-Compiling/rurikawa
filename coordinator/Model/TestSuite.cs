using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using System.Text.Json;
using System.Text.Json.Serialization;
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

        public ScoringMode ScoringMode { get; set; }

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
    [JsonConverter(typeof(SerDe.TestCaseDefinitionConverter))]
    public class TestCaseDefinition {
        public string Name { get; set; }
        public double BaseScore { get; set; } = 1.0;
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
        ShouldFail = 6,
        NotRunned = -1,
        Waiting = -2,
        Running = -3,
        OtherError = -100,
    }

    public enum JobStage {
        Queued = 0,
        Dispatched,
        Fetching,
        Compiling,
        Running,
        Finished,
        Cancelled,
        Skipped,
        Aborted,
    }

    public enum ScoringMode {
        /// <summary>
        /// The basic scoring mode, display `{passedCases}/{totalCases}`
        /// </summary>
        Basic = 0,

        /// <summary>
        /// Floating scoring mode. Displays `{currentScore}/{totalScore}`
        /// </summary>
        Floating = 1,
    }

    public class TestResult {
        public TestResultKind Kind { get; set; }
        public string? ResultFileId { get; set; }
        public double? Score { get; set; }
    }

    namespace SerDe {
        public class TestCaseDefinitionConverter : JsonConverter<TestCaseDefinition> {
            public override TestCaseDefinition Read(
                ref Utf8JsonReader reader,
                Type typeToConvert,
                JsonSerializerOptions options) {
                if (reader.TokenType == JsonTokenType.String) {
                    string name = reader.GetString()!;
                    return new TestCaseDefinition()
                    {
                        Name = name,
                        HasOut = true,
                        ShouldFail = false
                    };
                } else if (reader.TokenType == JsonTokenType.StartObject) {
                    return DeserializeFromMap(ref reader, options);
                } else {
                    throw new JsonException("Expected string or object");
                }
            }

            private TestCaseDefinition DeserializeFromMap(
                ref Utf8JsonReader reader,
                JsonSerializerOptions options) {
                string propName_name = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.Name)) ?? nameof(TestCaseDefinition.Name);
                string propName_hasOut = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.HasOut)) ?? nameof(TestCaseDefinition.HasOut);
                string propName_shouldFail = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.ShouldFail)) ?? nameof(TestCaseDefinition.ShouldFail);
                string propName_baseScore = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.BaseScore)) ?? nameof(TestCaseDefinition.BaseScore);

                string? name = null;
                bool? hasOut = null;
                bool? shouldFail = null;
                double? baseScore = null;

                while (reader.Read()) {
                    if (reader.TokenType == JsonTokenType.EndObject) break;
                    if (reader.TokenType != JsonTokenType.PropertyName) {
                        throw new JsonException("Expected Property Name");
                    }
                    var key = reader.GetString();
                    if (!reader.Read()) throw new JsonException("Expected value");
                    if (key == propName_name) {
                        if (name != null)
                            throw new JsonException("Duplicate property 'name'");
                        name = reader.GetString();
                    } else if (key == propName_hasOut) {
                        if (hasOut != null)
                            throw new JsonException("Duplicate property 'hasOut'");
                        hasOut = reader.GetBoolean();
                    } else if (key == propName_shouldFail) {
                        if (shouldFail != null)
                            throw new JsonException("Duplicate property 'shouldFail'");
                        shouldFail = reader.GetBoolean();
                    } else if (key == propName_baseScore) {
                        if (baseScore != null)
                            throw new JsonException("Duplicate property 'baseScore'");
                        baseScore = reader.GetDouble();
                    } else {
                        throw new JsonException($"Unknown property '{key}'");
                    }
                }

                if (name == null) throw new JsonException("Expected property 'name'");
                return new TestCaseDefinition
                {
                    Name = name,
                    HasOut = hasOut ?? true,
                    ShouldFail = shouldFail ?? false,
                    BaseScore = baseScore ?? 1.0
                };
            }

            public override void Write(
                Utf8JsonWriter writer,
                TestCaseDefinition value,
                JsonSerializerOptions options) {
                string propName_name = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.Name)) ?? nameof(TestCaseDefinition.Name);
                string propName_hasOut = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.HasOut)) ?? nameof(TestCaseDefinition.HasOut);
                string propName_shouldFail = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.ShouldFail)) ?? nameof(TestCaseDefinition.ShouldFail);
                string propName_baseScore = options.PropertyNamingPolicy?.ConvertName(nameof(TestCaseDefinition.BaseScore)) ?? nameof(TestCaseDefinition.BaseScore);

                writer.WriteStartObject();
                writer.WriteString(propName_name, value.Name);
                writer.WriteBoolean(propName_hasOut, value.HasOut);
                writer.WriteBoolean(propName_shouldFail, value.ShouldFail);
                writer.WriteNumber(propName_baseScore, value.BaseScore);
                writer.WriteEndObject();
            }
        }
    }
}
