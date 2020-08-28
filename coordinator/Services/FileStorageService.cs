using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class SingleBucketFileStorageService {
        public class Params {
            public string Bucket { get; set; }
            public string Endpoint { get; set; }
            public string AccessKey { get; set; }
            public string SecretKey { get; set; } = "";
            public bool Ssl { get; set; } = true;
        }

        public SingleBucketFileStorageService(
            Params param,
            ILogger<SingleBucketFileStorageService> logger
        ) : this(
            param.Bucket,
            param.Endpoint,
            param.AccessKey,
            param.SecretKey,
            param.Ssl,
            logger
        ) { }

        public SingleBucketFileStorageService(
            string bucket,
            string endpoint,
            string accessKey,
            string secretKey,
            bool hasSsl,
            ILogger<SingleBucketFileStorageService> logger
        ) {
            client = new Minio.MinioClient(endpoint, accessKey, secretKey);
            if (hasSsl) client = client.WithSSL();
            this.bucket = bucket;
            this.endpoint = endpoint;
            this.hasSsl = hasSsl;
            this.logger = logger;
        }

        private ILogger<SingleBucketFileStorageService> logger;

        private Minio.MinioClient client;
        private readonly string bucket;
        private readonly string endpoint;
        private readonly bool hasSsl;

        public async Task Check() {
            if (!await client.BucketExistsAsync(bucket)) {
                await client.MakeBucketAsync(bucket);
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
            var uri = new UriBuilder
            {
                Scheme = hasSsl ? "https" : "http",
                Host = $"{bucket}.{endpoint}",
                Path = filename
            };
            return uri.ToString();
        }
    }
}
