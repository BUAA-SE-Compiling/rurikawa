using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Karenia.Rurikawa.Models.Judger;
using Karenia.Rurikawa.Models.Test;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.ChangeTracking;
using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Metadata;

#nullable disable
namespace Karenia.Rurikawa.Models {
    public class RurikawaDb : DbContext {
        /// <summary>
        /// All jobs added to this judging system
        /// </summary>
        public DbSet<Job> Jobs { get; set; }

        /// <summary>
        /// All judgers added to this system
        /// </summary>
        public DbSet<JudgerEntry> Judgers { get; set; }

        public DbSet<TestSuite> TestSuites { get; set; }

        public DbSet<TestSuite> Accounts { get; set; }


        protected override void OnConfiguring(DbContextOptionsBuilder opt) {

        }

        protected override void OnModelCreating(ModelBuilder modelBuilder) {
            base.OnModelCreating(modelBuilder);
            modelBuilder.Entity<Job>().HasKey(x => x.Id);
        }
    }
}
