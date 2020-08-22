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

        // public int ActiveTaskCount { get; set; } = 0;
        // public bool CanAcceptNewTask { get; set; } = true;
        public JsonWebsocketWrapper<ClientMsg, ServerMsg> Socket { get; }

        /// <summary>
        /// A channel to communicate with `JudgerState`,
        /// and to indicate if this judger is finished.
        /// </summary>
        public Channel<string> Chan { get; }

        public string Id { get; }

        public Judger(
            string id,
            JsonWebsocketWrapper<ClientMsg, ServerMsg> socket,
            Channel<string> chan
        ) {
            Id = id;
            Socket = socket;
            Chan = chan;
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
            await Chan.Writer.WriteAsync(Id);
        }
    }

    /// <summary>
    /// A job to be run, which involves 1 test suite and 1 repo to be tested.
    /// </summary>
    public class Job {
        public readonly string imageName;

        /// <summary>
        /// Git remote address for the repo being tested,
        /// to be cloned and unzipped by the backend.
        /// </summary>
        public readonly string repo;

        [Column(TypeName = "jsonb")]
        public readonly JobConfig config;
    }

    public class JobConfig {
        /// <summary>
        /// Directory containing the source files and stdin files of the test suite.
        /// </summary>
        public string sourceDir;

        /// <summary>
        /// Directory containing the stdout files of the test suite.
        /// </summary>
        public string outDir;

        /// <summary>
        /// Names of the test files, without extensions.
        /// </summary>
        public List<string> tests;

        public uint? timeLimit;
        public uint? memLimit;
        public bool buildImage;

        public readonly List<VolumeBind> binds;
    }

    public class VolumeBind {
        /// <summary>
        /// Absolute/Relative `from` path (in the host machine).
        /// </summary>
        public string from;

        /// <summary>
        /// Absolute to path (in the container).
        /// </summary>
        public string to;

        /// <summary>
        /// Extra options for this bind. Leave a new `String` for empty.
        /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
        /// </summary>
        public string options;

        public VolumeBind(string from, string to, string options) {
            this.from = from;
            this.to = to;
            this.options = options;
        }
    }
}
