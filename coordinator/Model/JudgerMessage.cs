using System.Collections.Generic;
using System.Text.Json.Serialization;
using Dahomey.Json.Attributes;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Test;

#pragma warning disable CS8618  
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
        public Job Job { get; set; }
    }

    /// <summary>
    /// Message that provides a new job to judger with given id and specification.
    /// <br/>
    /// The client MUST accept this job after it's being sent.
    /// </summary>
    [JsonDiscriminator("abort_job")]
    public class AbortJobServerMsg : ServerMsg {
        public FlowSnake JobId { get; set; }
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
    public class JobResultMsg : ClientMsg {
        public FlowSnake JobId { get; set; }

        /// <summary>
        /// Indicates whether this job has finished normally or is aborted due 
        /// to client issues (e.g. client is killed).
        /// <br/>
        /// If this value is Aborted, the job should be rescheduled immediately 
        /// to other runners.
        /// </summary>
        public JobResultKind JobResult { get; set; }

        public string? Message { get; set; }

        public Dictionary<string, TestResult>? Results { get; set; }
    }

    /// <summary>
    /// Message that reports the progress of a single job in judger.
    /// </summary>
    [JsonDiscriminator("job_progress")]
    public class JobProgressMsg : ClientMsg {
        public FlowSnake JobId { get; set; }

        /// <summary>
        /// Current stage of the job.
        /// </summary>
        public JobStage Stage { get; set; }

    }

    /// <summary>
    /// Message that reports the progress of a single job in judger.
    /// </summary>
    [JsonDiscriminator("partial_result")]
    public class PartialResultMsg : ClientMsg {
        public FlowSnake JobId { get; set; }

        public string TestId { get; set; }

        public TestResult TestResult { get; set; }
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
        public bool RequestForNewTask { get; set; }
    }
}

