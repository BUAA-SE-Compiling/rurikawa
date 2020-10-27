using System;
using System.Text.Json;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class GenericCacheService {
        private readonly RedisService redis;
        private readonly JsonSerializerOptions? jsonOptions;

        public GenericCacheService(RedisService redis, JsonSerializerOptions? jsonOptions) {
            this.redis = redis;
            this.jsonOptions = jsonOptions;
        }

        public string FormatRedisKey(string section, string key) => $"{section}:{key}";

        public async Task<string?> CheckCacheString(
            string section,
            string key) {
            var redisKey = FormatRedisKey(section, key);
            var db = await redis.GetDatabase();
            var val = await db.StringGetAsync(redisKey);
            if (val.IsNull) {
                return null;
            } else {
                return val;
            }
        }

        public async Task<T?> CheckCache<T>(
            string section,
            string key) where T : class {
            var redisKey = FormatRedisKey(section, key);
            var db = await redis.GetDatabase();
            var val = await db.StringGetAsync(redisKey);
            if (val.IsNull) {
                return null;
            } else {
                var t = JsonSerializer.Deserialize<T>(val, jsonOptions);
                return t;
            }
        }

        public async Task<T?> CheckCacheStruct<T>(
            string section,
            string key) where T : struct {
            var redisKey = FormatRedisKey(section, key);
            var db = await redis.GetDatabase();
            var val = await db.StringGetAsync(redisKey);
            if (val.IsNull) {
                return null;
            } else {
                var t = JsonSerializer.Deserialize<T>(val, jsonOptions);
                return t;
            }
        }

        public async Task PutCache<T>(
            string section,
            string key,
            T value,
            TimeSpan expireTime) {
            var db = await redis.GetDatabase();
            var stringified = JsonSerializer.Serialize(value, jsonOptions);

            db.StringSet(
               FormatRedisKey(section, key),
               stringified,
               expireTime,
               flags: StackExchange.Redis.CommandFlags.FireAndForget);
        }

        public async Task PurgeCache(string section, string key) {
            var db = await redis.GetDatabase();
            db.KeyDelete(FormatRedisKey(section, key), StackExchange.Redis.CommandFlags.FireAndForget);
        }
    }

    public class RurikawaCacheService {
        private readonly GenericCacheService cacheService;

        public RurikawaCacheService(GenericCacheService cacheService) {
            this.cacheService = cacheService;
        }

        const string JobSection = "cache:jobs";
        const string SuiteSection = "cache:suite";

        readonly TimeSpan CacheExpiry = TimeSpan.FromHours(2);

        public Task<Job?> GetCachedJob(FlowSnake key) {
            return cacheService.CheckCache<Job>(JobSection, key.ToString());
        }

        public Task SetJob(Job job) {
            return cacheService.PutCache(JobSection, job.Id.ToString(), job, CacheExpiry);
        }

        public Task PurgeJob(FlowSnake key) {
            return cacheService.PurgeCache(JobSection, key.ToString());
        }

        public Task<TestSuite?> GetCachedTestSuite(FlowSnake key) {
            return cacheService.CheckCache<TestSuite>(SuiteSection, key.ToString());
        }

        public Task<string?> GetCachedTestSuiteString(FlowSnake key) {
            return cacheService.CheckCacheString(SuiteSection, key.ToString());
        }

        public Task SetTestSuite(TestSuite suite) {
            return cacheService.PutCache(SuiteSection, suite.Id.ToString(), suite, CacheExpiry);
        }

        public Task PurgeSuite(FlowSnake key) {
            return cacheService.PurgeCache(SuiteSection, key.ToString());
        }
    }
}
