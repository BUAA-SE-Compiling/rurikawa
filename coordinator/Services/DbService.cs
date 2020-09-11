using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Judger;
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
    }
}
