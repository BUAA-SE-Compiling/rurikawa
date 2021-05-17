using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Models;
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
    }
}
