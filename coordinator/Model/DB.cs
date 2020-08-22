using Microsoft.EntityFrameworkCore;

namespace Karenia.Rurikawa.Models {
    public class DB : DbContext {
        

        protected override void OnConfiguring(DbContextOptionsBuilder opt) {

        }
    }
}
