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
        // Channel<string> chan
        ) {
            Id = id;
            Socket = socket;
            // Chan = chan;
        }

        /// <summary>
        /// Run a judger and get results.
        /// </summary>
        public async Task<int> Run() {
            // TODO: Actually run the judger.
            var rand = new Random();
            var dur = rand.Next(2000);
            // Run an expensive job.
            await Task.Delay(dur);
            // Send a signal to the channel when finished,
            // indicating availability.
            await Finish();
            return 0;
        }

        /// <summary>
        /// Tell the channel that the job is done.
        /// </summary>
        public async Task Finish() {
            // await Chan.Writer.WriteAsync(Id);
        }
    }

    /// <summary>
    /// A job to be run, which involves 1 test suite and 1 repo to be tested.
    /// </summary>
    public class Job {
        public Job(
            long id,
            string repo,
            string? branch,
            FlowSnake testSuite,
            List<string> tests,
            JobStage stage = default,
            Dictionary<string, TestResult>? results = null
        ) {
            Id = new FlowSnake(id);
            Repo = repo;
            Branch = branch;
            TestSuite = testSuite;
            Tests = tests;
            Stage = stage;
            Results = results;
        }

        public Job(
            FlowSnake id,
            string repo,
            string? branch,
            FlowSnake testSuite,
            List<string> tests,
            JobStage stage,
            Dictionary<string, TestResult>? results
        ) {
            Id = id;
            Repo = repo;
            Branch = branch;
            TestSuite = testSuite;
            Tests = tests;
            Stage = stage;
            Results = results;
        }

        /// <summary>
        /// A globally unique identifier of this job.
        /// </summary>
        public FlowSnake Id { get; private set; }

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

        [Column(TypeName = "jsonb")]
        public Dictionary<string, TestResult>? Results { get; set; }
    }

    /// <summary>
    /// Represents a single judger added to the system
    /// </summary>
    public class JudgerEntry {
        public JudgerEntry(
            string id,
            string? alternateName,
            List<string>? tags = null,
             bool acceptUntaggedJobs = false
        ) {
            Id = id;
            AlternateName = alternateName;
            Tags = tags;
            AcceptUntaggedJobs = acceptUntaggedJobs;
        }

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

        public bool AcceptUntaggedJobs { get; set; }
    }
}
