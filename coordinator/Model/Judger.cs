using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using System.Threading.Channels;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;

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
        public Job(ulong id, string repo, string? branch, string testName) {
            Id = id;
            Repo = repo;
            Branch = branch;
            TestName = testName;
        }

        /// <summary>
        /// A globally unique identifier of this job.
        /// </summary>
        public ulong Id { get; private set; }

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
        public string TestName { get; set; }
    }
}
