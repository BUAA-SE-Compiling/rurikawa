using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Judger;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class JudgerService {
        private readonly ILogger<JudgerService> logger;
        private readonly AccountService accountService;
        private readonly RurikawaDb db;

        public JudgerService(ILogger<JudgerService> logger, AccountService accountService, RurikawaDb db) {
            this.logger = logger;
            this.accountService = accountService;
            this.db = db;
        }

        public async Task<JudgerEntry> RegisterJudger(
            string registerToken,
            string? alternateName,
            List<string>? tags) {
            var judgerToken = await accountService.GetJudgerRegisterToken(registerToken);
            if (judgerToken == null)
                throw new KeyNotFoundException("No such token was found");

            var judgerAccessToken = AccountService.GenerateToken();
            var judger = new JudgerEntry {
                Id = judgerAccessToken,
                AlternateName = alternateName,
                Tags = tags,
                AcceptUntaggedJobs = true
            };
            db.Judgers.Add(judger);
            await db.SaveChangesAsync();
            return judger;
        }

        public async ValueTask<JudgerEntry?> GetJudgerByToken(string token) {
            var judger = await db.Judgers.Where(j => j.Id == token)
                .AsNoTracking()
                .SingleOrDefaultAsync();
            return judger;
        }

        public async Task<List<JudgerEntry>> QueryJudger(
            List<string> tag, string fromId = "", int count = 50) {
            var judger = await db.Judgers
                .Where(j => j.Tags != null && j.Tags.All(t => tag.Contains(t)))
                .Where(j => j.Id.CompareTo(fromId) > 0)
                .Take(count)
                .AsNoTracking()
                .ToListAsync();
            return judger;
        }

        public async Task<List<JudgerTokenEntry>> QueryJudgerRegisterToken(
            List<string> tags,
            bool? expired,
            string start,
            int take) {
            var query = db.JudgerRegisterTokens.AsQueryable();
            if (tags.Count > 0) {
                query = query.Where(token => tags.All(tag => token.Tags.Contains(tag)));
            }
            if (expired != null) {
                var now = DateTimeOffset.Now;
                if (expired.Value) {
                    query = query.Where(token => token.Expires < now);
                } else {
                    query = query.Where(token => token.Expires >= now);
                }
            }
            return await query.Where(token => token.Token.CompareTo(start) > 0)
                .Take(take)
                .ToListAsync();
        }

        public async Task<int> DeleteJudger(string id) {
            return await db.Judgers.Where(j => j.Id == id).DeleteFromQueryAsync();
        }

        public async Task<int> DeleteToken(string id) {
            return await db.JudgerRegisterTokens.Where(j => j.Token == id).DeleteFromQueryAsync();
        }
    }
}
