using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using StackExchange.Redis;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class DbService {
        private readonly RurikawaDb db;

        public DbService(RurikawaDb db) {
            this.db = db;
        }

        public async Task<Job> GetJob(FlowSnake id) {
            return await db.Jobs.Where(j => j.Id == id).SingleOrDefaultAsync();
        }

        public async Task<TestSuite> GetTestSuite(FlowSnake id) {
            return await db.TestSuites.Where(j => j.Id == id).SingleOrDefaultAsync();
        }

        public async Task<IList<Job>> GetJobs(
            FlowSnake? startId = null,
            int take = 20,
            bool asc = false,
            FlowSnake? bySuite = null,
            string? byUsername = null
            ) {
            var query = db.Jobs.AsQueryable();

            if (bySuite != null)
                query = query.Where(j => j.TestSuite == bySuite.Value);

            if (byUsername != null)
                query = query.Where(j => j.Account == byUsername);

            if (asc) {
                if (startId == null) startId = FlowSnake.MinValue;
                query = query.Where(j => j.Id > startId).OrderBy(j => j.Id);
            } else {
                if (startId == null) startId = FlowSnake.MaxValue;
                query = query.Where(j => j.Id < startId).OrderByDescending(j => j.Id);
            }

            query = query.Take(take);
            var result = await query.ToListAsync();

            return result;
        }

        public async Task<List<TestSuite>> GetTestSuites(
            FlowSnake? startId = null,
            int take = 20,
            bool asc = false) {
            var query = db.TestSuites.AsQueryable();
            if (asc) {
                if (startId == null) startId = FlowSnake.MinValue;
                query = query.Where(j => j.Id > startId).OrderBy(j => j.Id);
            } else {
                if (startId == null) startId = FlowSnake.MaxValue;
                query = query.Where(j => j.Id < startId).OrderByDescending(j => j.Id);
            }
            query = query.Take(take);
            return await query.ToListAsync();
        }
    }

    public class DbVacuumingService {
        private static readonly TimeSpan vacuumInterval = TimeSpan.FromMinutes(30);
        private readonly ILogger<DbVacuumingService> logger;
        private readonly IServiceScopeFactory scopeFactory;

        public DbVacuumingService(
            ILogger<DbVacuumingService> logger,
            IServiceScopeFactory scopeFactory) {
            this.logger = logger;
            this.scopeFactory = scopeFactory;
        }

        public async void StartVacuuming() {
            while (true) {
                await VacuumDb();
                await Task.Delay(vacuumInterval);
            }
        }

        private async Task VacuumDb() {
            using var scope = scopeFactory.CreateScope();
            var db = scope.ServiceProvider.GetService<RurikawaDb>();
            await VacuumTokens(db.AccessTokens);
            await VacuumTokens(db.RefreshTokens);
            await VacuumTokens(db.JudgerRegisterTokens);
        }

        private async Task VacuumTokens<T>(DbSet<T> tokenSet) where T : TokenBase {
            var now = DateTimeOffset.Now;
            var nowMinusGracePeriod = DateTimeOffset.Now - TokenBase.SingleUseTokenGracePeriod;
            await tokenSet
                .Where(x => (
                    x.Expires < now
                    || (x.IsSingleUse
                        && x.LastUseTime.HasValue
                        && x.LastUseTime < nowMinusGracePeriod)))
                .DeleteFromQueryAsync();
        }
    }

    public class RedisService {
        public RedisService(string redisConnectionString) {
            this.redisConnectionString = redisConnectionString;
        }

        private readonly string redisConnectionString;
        private ConnectionMultiplexer? redisConnection;

        public async ValueTask<IDatabase> GetDatabase() {
            return (await GetRedisConnection()).GetDatabase();
        }

        public async ValueTask<ConnectionMultiplexer> GetRedisConnection() {
            if (this.redisConnection == null)
                this.redisConnection = await ConnectionMultiplexer.ConnectAsync(this.redisConnectionString);
            return this.redisConnection;
        }
    }
}
