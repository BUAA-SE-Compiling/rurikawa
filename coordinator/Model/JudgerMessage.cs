using System.Text.Json.Serialization;
using Dahomey.Json.Attributes;

namespace Karenia.Rurikawa.Models.Judger {
    /// <summary>
    /// Base class of all messages that are sent from the server (coordinator).
    /// </summary>
    public class ServerMsg { }

    /// <summary>
    /// Message that provides a new job to judger with given id and specification.
    /// <br/>
    /// The client MUST accept this job after it's being sent.
    /// </summary>
    [JsonDiscriminator("new_job")]
    public class NewJobServerMsg : ServerMsg {
        public string Id { get; set; }

        [JsonPropertyName("pkg_uri")]
        public string PackageUri { get; set; }
    }

    /// <summary>
    /// Base class of all messages that are sent from a client (judger).
    /// </summary>
    public class ClientMsg { }

    /// <summary>
    /// Message that reports the result of a single job in judger.
    /// <br/>
    /// The state of this job should be changed to "completed" after this 
    /// message is received.
    /// </summary>
    [JsonDiscriminator("job_result")]
    public class JobResultMsg : ClientMsg { }

    public enum JobStage {
        Created,
        Cloning,
        Compiling,
        Testing,
        Finished
    }

    /// <summary>
    /// Message that reports the progress of a single job in judger.
    /// </summary>
    [JsonDiscriminator("job_progress")]
    public class JobProgressMsg : ClientMsg {
        public string Id { get; set; }

        /// <summary>
        /// Current stage of the job.
        /// </summary>
        public JobStage Stage { get; set; }

        /// <summary>
        /// Total progress points of this stage.
        /// </summary>
        public ulong? TotalPoints { get; set; }

        /// <summary>
        /// Finished progress points of this stage.
        /// </summary>
        public ulong? FinishedPoints { get; set; }
    }

    /// <summary>
    /// Message that reports the current status of the judger.
    /// <br/>
    /// The status of the client should be updated correspondingly after this 
    /// message is received.
    /// </summary>
    [JsonDiscriminator("client_status")]
    public class ClientStatusMsg : ClientMsg {
        public int ActiveTaskCount { get; set; }
        public bool CanAcceptNewTask { get; set; }
    }
}
