using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Microsoft.Extensions.Logging;
using Minio;
using Minio.DataModel.Tracing;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class SingleBucketFileStorageService {
        public class Params {
            public string Bucket { get; set; }
            public string Endpoint { get; set; }
            public string? PublicEndpoint { get; set; }
            public string AccessKey { get; set; }
            public string SecretKey { get; set; } = "";
            public string BucketPolicy { get; set; } = "";
            public bool Ssl { get; set; } = true;
            public bool PublicSsl { get; set; } = true;
        }

        public SingleBucketFileStorageService(
            Params param,
            ILogger<SingleBucketFileStorageService> logger,
            MinioRequestLogger? minioRequestLogger
        ) : this(
            param.Bucket,
            param.Endpoint,
            param.PublicEndpoint,
            param.AccessKey,
            param.SecretKey,
            param.Ssl,
            param.PublicSsl,
            logger,
            minioRequestLogger
        ) { }

        public SingleBucketFileStorageService(
            string bucket,
            string endpoint,
            string? publicEndpoint,
            string accessKey,
            string secretKey,
            bool hasSsl,
            bool hasPublicSsl,
            ILogger<SingleBucketFileStorageService> logger,
            MinioRequestLogger? minioRequestLogger
        ) {
            client = new Minio.MinioClient(endpoint, accessKey, secretKey);
            client.SetTraceOn(minioRequestLogger);
            if (hasSsl) client = client.WithSSL();
            this.bucket = bucket;
            this.endpoint = endpoint;
            this.publicEndpoint = publicEndpoint;
            var endpointUri = new UriBuilder(publicEndpoint ?? this.endpoint);
            if (endpointUri.Host == null || endpointUri.Host == "" || endpointUri.Scheme != null || endpointUri.Scheme != "") {
            } else {
                endpointUri.Scheme = hasPublicSsl ? "https" : "http";
            }
            this.publicEndpointUri = new Uri(endpointUri.Uri, bucket + "/");
            logger.LogInformation("Set up public endpoint as {0}", publicEndpointUri.ToString());
            this.hasSsl = hasSsl;
            this.logger = logger;
        }

        private ILogger<SingleBucketFileStorageService> logger;

        private Minio.MinioClient client;
        private readonly string bucket;
        private readonly string endpoint;
        private readonly string? publicEndpoint;
        private readonly bool hasSsl;
        private readonly Uri publicEndpointUri;

        public async Task Check() {
            if (!await client.BucketExistsAsync(bucket)) {
                await client.MakeBucketAsync(bucket);
                await client.SetPolicyAsync(bucket, $@"
{{
  ""Id"": ""ReadOnlyForEveryone"",
  ""Version"": ""2012-10-17"",
  ""Statement"": [
    {{
      ""Sid"": ""Stmt1601372292618"",
      ""Action"": [
        ""s3:GetObject"",
        ""s3:GetObjectVersion""
      ],
      ""Effect"": ""Allow"",
      ""Resource"": ""arn:aws:s3:::{bucket}/*"",
      ""Principal"": ""*""
    }}
  ]
}}
            ");
            }
        }

        public async Task UploadFile(
            string fileName,
            Stream file,
            long length,
            bool isPublic = true,
            CancellationToken c = default
        ) {
            logger.LogInformation("Upload started. filename {0}, length {1}", fileName, length);
            var metadata = new Dictionary<string, string>();
            if (isPublic) {
                metadata["x-amz-acl"] = "public-read";
            }
            await client.PutObjectAsync(
                bucket,
                fileName,
                file,
                length,
                metaData: metadata,
                cancellationToken: c);
            logger.LogInformation("Upload end.");
        }

        /// <summary>
        /// Formats and returns the 
        /// </summary>
        /// <param name="filename"></param>
        /// <returns></returns>
        public string GetFileAddress(
            string filename
        ) {
            // filename must be a relative directory
            filename = filename.TrimStart('/');
            var uri = new Uri(publicEndpointUri, filename);
            logger.LogInformation("Mapped endpoint {0} as {1}", filename, uri.ToString());
            return uri.ToString();
        }

        /// <summary>
        /// Adaptor to ASP.NET Core's logger
        /// </summary>
        public class MinioRequestLogger : Minio.IRequestLogger {
            private readonly ILogger<MinioClient> logger;

            public MinioRequestLogger(ILogger<MinioClient> logger) {
                this.logger = logger;
            }

            public void LogRequest(RequestToLog requestToLog, ResponseToLog responseToLog, double durationMs) {
                if (!this.logger.IsEnabled(LogLevel.Trace)) return;

                var msg = new StringBuilder();
                msg.AppendFormat("{0}ms\n", durationMs);
                msg.AppendFormat("--> {0} {1} : {2}\n", requestToLog.method, requestToLog.uri, requestToLog.resource);
                foreach (var param in requestToLog.parameters) {
                    msg.AppendFormat("    {0}: {1}\n", param.name, param.value);
                }

                msg.AppendFormat("<-- {0}\n", responseToLog.statusCode);
                foreach (var header in responseToLog.headers) {
                    msg.AppendFormat("    {0}: {1}\n", header.Name, header.Value);
                }
                msg.AppendLine();
                if (responseToLog.errorMessage != null && responseToLog.errorMessage != "") {
                    msg.AppendFormat("Err: {0}", responseToLog.errorMessage);
                } else {
                    msg.Append(responseToLog.content);
                }


                this.logger.LogTrace(msg.ToString());
            }
        }
    }

}
