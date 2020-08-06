using Dahomey.Json.Attributes;
using System.Text.Json.Serialization;

namespace Karenia.Rurikawa.Models.Judger
{
    public class ServerMsg { }


    [JsonDiscriminator("new_job")]
    public class NewJobServerMsg : ServerMsg
    {
        public string Id { get; set; }

        [JsonPropertyName("pkg_uri")]
        public string PackageUri { get; set; }
    }

    public class ClientMsg { }

    [JsonDiscriminator("job_result")]
    public class JobResultMsg : ClientMsg { }

    [JsonDiscriminator("job_process")]
    public class JobProcessMsg : ClientMsg { }

    [JsonDiscriminator("client_status")]
    public class ClientStatusMsg : ClientMsg
    {
        public int ActiveTaskCount { get; set; }
        public bool CanAcceptNewTask { get; set; }
    }
}
