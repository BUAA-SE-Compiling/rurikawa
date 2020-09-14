using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.EntityFrameworkCore;

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
}
