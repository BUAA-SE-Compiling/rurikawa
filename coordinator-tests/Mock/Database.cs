using Karenia.Rurikawa.Models;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;

namespace Karenia.Rurikawa.Coordinator.Tests {
    public static class Mock {
        /// <summary>
        /// This creates an in-memory database for test uses.
        /// </summary>
        /// <returns></returns>
        public static RurikawaDb CreateMockDatabase() {
            var dbCtxOptions = new DbContextOptionsBuilder<RurikawaDb>();
            dbCtxOptions.UseSqlite("Data Source=:memory:");

            var dbCtx = new RurikawaDb(dbCtxOptions.Options);
            return dbCtx;
        }
    }
}
