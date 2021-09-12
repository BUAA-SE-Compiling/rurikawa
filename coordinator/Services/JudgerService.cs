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

        /// <summary>
        /// Search <see cref="JudgerEntry"/>s by tags.
        /// 
        /// <para>
        ///     If a future version adds more properties to judgers, they should also be 
        ///     able to be queried by this method.
        /// </para>
        /// </summary>
        /// <param name="tags">Judger tags. A judger should contain all tags specified to apperar in the result.</param>
        /// <param name="fromId">The starting judger id to be queried</param>
        /// <param name="count">The number of judgers to return</param>
        /// <returns></returns>
        public async Task<List<JudgerEntry>> QueryJudger(
            List<string> tags, string fromId = "", int count = 50) {
            var judger = await db.Judgers
                .Where(j => j.Tags != null && j.Tags.All(t => tags.Contains(t)))
                .Where(j => j.Id.CompareTo(fromId) > 0)
                .Take(count)
                .AsNoTracking()
                .ToListAsync();
            return judger;
        }

        /// <summary>
        /// Search <see cref="JudgerTokenEntry"/>s by their properties.
        /// </summary>
        /// <param name="tags">Token tags. A token must contain all tags to appear in the results.</param>
        /// <param name="expired">Whether the token is already expired</param>
        /// <param name="start">The starting token to be queried</param>
        /// <param name="take">The number of tokens to return</param>
        /// <returns></returns>
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

        /// <summary>
        /// Remove the specified judger from database. The judger will <b>not</b> be 
        /// disconnected immediately.
        /// </summary>
        /// <param name="id"></param>
        /// <returns></returns>
        public async Task<int> DeleteJudger(string id) {
            return await db.Judgers.Where(j => j.Id == id).DeleteFromQueryAsync();
        }

        /// <summary>
        /// Remove the specified Judger Register Token from database.
        /// </summary>
        /// <param name="token"></param>
        /// <returns></returns>
        public async Task<int> DeleteJudgerToken(string token) {
            return await db.JudgerRegisterTokens.Where(j => j.Token == token).DeleteFromQueryAsync();
        }
    }
}
