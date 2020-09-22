using System.Collections.Generic;
using Dahomey.Json.Attributes;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;

namespace Karenia.Rurikawa.Models.WebsocketApi {
    public class WsApiServerMsg { }

    [JsonDiscriminator("new_job_s")]
    public class NewJobUpdateMsg : WsApiServerMsg {
        public Job Job { get; set; }
    }

    [JsonDiscriminator("job_status_s")]
    public class JobStatusUpdateMsg : WsApiServerMsg {
        public FlowSnake JobId { get; set; }
        public string? BuildStream { get; set; }
        public JobStage? Stage { get; set; }
        public JobResultKind? JobResult { get; set; }
        public Dictionary<string, TestResult>? TestResult { get; set; }
    }

    [JsonDiscriminator("judger_status_s")]
    public class JudgerStatusUpdateMsg : WsApiServerMsg { }

    [JsonDiscriminator("test_output_s")]
    public class TestOutputUpdateMsg : WsApiServerMsg { }

    public class WsApiClientMsg { }

    [JsonDiscriminator("sub_c")]
    public class SubscribeMsg : WsApiClientMsg {
        /// <summary>
        /// Whether to subscribe or unsubscribe
        /// </summary>
        public bool Sub { get; set; }
        /// <summary>
        /// Jobs to subscribe/unsubscribe
        /// </summary>
        public List<FlowSnake>? Jobs { get; set; }
        /// <summary>
        /// Suites to subscribe/unsubscribe
        /// </summary>
        public List<FlowSnake>? Suites { get; set; }
    }
}
