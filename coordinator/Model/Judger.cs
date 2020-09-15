using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using System.Threading.Channels;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Test;

#pragma warning disable CS8618  
namespace Karenia.Rurikawa.Models.Judger {
    /// <summary>
    /// A runner of a specific testing task.
    /// </summary>
    public class Judger {
        public JsonWebsocketWrapper<ClientMsg, ServerMsg> Socket { get; }
        public JudgerEntry DbJudgerEntry { get; }
        public string Id { get; }

        /// <summary>
        /// Number of tasks (jobs) currently running on this judger.
        /// <br/>
        /// This value currently has no real usage.
        /// </summary>
        public int ActiveTaskCount { get; set; } = 0;

        /// <summary>
        /// Whether this judger can accept new tasks.
        /// </summary>
        public bool CanAcceptNewTask { get; set; } = false;

        public Judger(
            string id,
            JsonWebsocketWrapper<ClientMsg, ServerMsg> socket
        ) {
            Id = id;
            Socket = socket;
        }
    }

    public enum JobResultKind {
        Accepted,
        CompileError,
        PipelineError,
        JudgerError,
        Aborted,
        OtherError,
    }


    /// <summary>
    /// A job to be run, which involves 1 test suite and 1 repo to be tested.
    /// </summary>
    public class Job {
        /// <summary>
        /// A globally unique identifier of this job.
        /// </summary>
        public FlowSnake Id { get; set; }

        /// <summary>
        /// The account that created this job
        /// </summary>
        public string Account { get; set; }

        /// <summary>
        /// Git remote address for the repo being tested,
        /// to be cloned and unzipped by the backend.
        /// </summary>
        public string Repo { get; set; }

        /// <summary>
        /// The branch of that repo to be tested. Omit to use the default branch.
        /// </summary>
        public string? Branch { get; set; }

        /// <summary>
        /// The revision of that repo to be tested. This is the actual data sent
        /// to judgers.
        /// </summary>
        public string Revision { get; set; }

        /// <summary>
        /// The job suite to test.
        /// </summary>
        public FlowSnake TestSuite { get; set; }

        /// <summary>
        /// The test cases selected for this job
        /// </summary>
        public List<string> Tests { get; set; }

        /// <summary>
        /// The current (last seen) stage of this test
        /// </summary>
        public JobStage Stage { get; set; }

        /// <summary>
        /// The result of this job, if applicable
        /// </summary>
        public JobResultKind? ResultKind { get; set; }

        /// <summary>
        /// Attached message for the result of this job, if applicable
        /// </summary>
        public string? ResultMessage { get; set; }

        [Column(TypeName = "jsonb")]
        public Dictionary<string, TestResult> Results { get; set; } = new Dictionary<string, TestResult>();
    }

    /// <summary>
    /// Represents a single judger added to the system
    /// </summary>
    public class JudgerEntry {
        /// <summary>
        /// The ID (and token) of this Judger
        /// </summary>
        public string Id { get; set; }

        /// <summary>
        /// The alternative name of this Judger
        /// </summary>
        public string? AlternateName { get; set; }

        /// <summary>
        /// The tags added to this Judger
        /// </summary>
        public List<string>? Tags { get; set; }

        public bool AcceptUntaggedJobs { get; set; } = true;
    }
}
