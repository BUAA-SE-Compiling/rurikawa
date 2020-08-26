using System;
using System.IO;
using System.Threading.Tasks;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class SingleBucketFileStorageService {
        public SingleBucketFileStorageService(
            string bucket,
            string endpoint,
            string accessKey,
            string secretKey,
            bool hasSsl
            ) {
            client = new Minio.MinioClient(endpoint, accessKey, secretKey);
            if (hasSsl) client = client.WithSSL();
            this.bucket = bucket;
            this.endpoint = endpoint;
            this.hasSsl = hasSsl;
        }

        private Minio.MinioClient client;
        private readonly string bucket;
        private readonly string endpoint;
        private readonly bool hasSsl;

        public async Task Check() {
            if (!await client.BucketExistsAsync(bucket)) {
                await client.MakeBucketAsync(bucket);
            }
        }

        public async Task<bool> UploadFile(
            string fileName,
            Stream file,
            long length
        ) {
            try {
                await client.PutObjectAsync(bucket, fileName, file, length);
                return true;
            } catch {
                return false;
            }
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
