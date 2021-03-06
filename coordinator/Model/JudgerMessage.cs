using System;
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

    [JsonDiscriminator("server_hello")]
    public class ServerHelloMsg : ServerMsg { }

    /// <summary>
    /// Message that provides a new job to judger with given id and specification.
    /// <br/>
    /// The client MUST accept this job after it's being sent.
    /// </summary>
    [Obsolete("Please use MultipleNewJobServerMsg")]
    [JsonDiscriminator("new_job")]
    public class NewJobServerMsg : ServerMsg {
        public Job Job { get; set; }
    }

    /// <summary>
    /// Message that provides a new job to judger with given id and specification.
    /// This message replaces <c>NewJobServeMsg</c> as we switch from pusing job
    /// to polling job.
    /// <br/>
    /// The client MUST accept this job after it's being sent.
    /// </summary>
    [JsonDiscriminator("new_job_multi")]
    public class MultipleNewJobServerMsg : ServerMsg {
        /// <summary>
        /// The message this message replies to.
        /// </summary>
        public FlowSnake? ReplyTo { get; set; }

        /// <summary>
        /// The list of jobs for this judger to run. The count of jobs might be 
        /// less than the requested count.
        /// </summary>
        public List<Job> Jobs { get; set; }
    }

    /// <summary>
    /// Message that requests the given client to abort this job.
    /// <p>
    /// The client MUST abort this job according to the given params. The client
    /// MUST either send a single <c>JobProgressMsg</c> with the given params,
    /// or send no message after abort.
    /// </p>
    /// <p>
    /// If <c>AsCancel</c>
    /// is true, this job MUST be marked as <c>Cancelled</c> and will not be retried. 
    /// Otherwise, it MUST be marked as <c>Aborted</c> and be rescheduled to a future
    /// run.
    /// </p>
    /// </summary>
    [JsonDiscriminator("abort_job")]
    public class AbortJobServerMsg : ServerMsg {
        public FlowSnake JobId { get; set; }
        public bool AsCancel { get; set; }
    }

    /// <summary>
    /// Base class of all messages that are sent from a client (judger).
    /// </summary>
    public class ClientMsg { }

    /// <summary>
    /// Message that reports the result of a single job in judger.
    /// <br/>
    /// The state of this job SHOULD be changed to "completed" after this 
    /// message is received.
    /// </summary>
    [JsonDiscriminator("job_result")]
    public class JobResultMsg : ClientMsg {
        public FlowSnake JobId { get; set; }

        /// <summary>
        /// Indicates whether this job has finished normally or is aborted due 
        /// to client issues (e.g. client is killed).
        /// <br/>
        /// If this value is Aborted, the job SHOULD be rescheduled to other runners,
        /// with job results resetted to Null.
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
    /// Message that sends the output of a job in client
    /// </summary>
    [JsonDiscriminator("job_output")]
    public class JobOutputMsg : ClientMsg {
        public FlowSnake JobId { get; set; }

        /// <summary>
        /// output stream chunk
        /// </summary>
        public string? Stream { get; set; }

        /// <summary>
        /// output stream error
        /// </summary>
        /// <value></value>
        public string? Error { get; set; }
    }

    /// <summary>
    /// Message that reports the progress of a single job in judger.
    /// <para>
    ///     The server MUST set this test result onto the corresponding job.
    /// </para>
    /// </summary>
    [JsonDiscriminator("partial_result")]
    public class PartialResultMsg : ClientMsg {
        public FlowSnake JobId { get; set; }

        public string TestId { get; set; }

        public TestResult TestResult { get; set; }
    }

    /// <summary>
    /// Message that reports the current status of the judger.
    /// <para>
    ///     The status of the client should be updated correspondingly after this 
    ///     message is received.
    /// </para>
    /// <para>
    ///     This class is only for compatibility. Use <c>JobRequestMsg</c> 
    ///     in new code instead.
    /// </para>
    /// </summary>
    [JsonDiscriminator("client_status")]
    public class ClientStatusMsg : ClientMsg {
        /// <summary>
        /// The number of jobs currently running in this judger
        /// </summary>
        public int ActiveTaskCount { get; set; }

        /// <summary>
        /// Whether this judger still accepts new jobs.
        /// </summary>
        public bool CanAcceptNewTask { get; set; }
    }

    [JsonDiscriminator("job_request")]
    public class JobRequestMsg : ClientMsg {
        /// <summary>
        /// The number of jobs currently running in this judger
        /// </summary>
        public int ActiveTaskCount { get; set; }

        /// <summary>
        /// The number of jobs to request for
        /// </summary>
        public int RequestForNewTask { get; set; }

        /// <summary>
        /// An ID for tracking job requests.
        /// </summary>
        public FlowSnake? MessageId { get; set; }
    }
}

