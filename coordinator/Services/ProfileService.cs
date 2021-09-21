using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Microsoft.EntityFrameworkCore;
using Pipelines.Sockets.Unofficial.Arenas;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class ProfileService {
        private readonly RurikawaDb db;

        public ProfileService(RurikawaDb db) {
            this.db = db;
        }

        public async Task InitializeProfileIfNotExists(string username) {
            if (await db.Profiles.AnyAsync(p => p.Username == username)) {
                return;
            }
            db.Profiles.Add(new Profile() { Username = username });
            await db.SaveChangesAsync();
        }

        public async Task UpsertProfile(string username, Profile profile) {
            if (profile.Username != username) {
                throw new ArgumentException("Username not matching profile", nameof(username));
            }
            var dbProfile = await db.Profiles.Where(p => p.Username == username)
                .SingleOrDefaultAsync();
            if (dbProfile == null) {
                db.Add(profile);
            } else {
                dbProfile.Email = profile.Email;
                dbProfile.StudentId = profile.StudentId;
            }
            await db.SaveChangesAsync();
        }

        public async Task<Profile?> GetProfile(string username) {
            return await db.Profiles.Where(p => p.Username == username)
                .SingleOrDefaultAsync();
        }

        public async Task<AccountAndProfile?> GetAccountAndProfile(string username) {
            return await db.AccountAndProfileView.Where(p => p.Username == username).SingleOrDefaultAsync();
        }

        public async Task<List<AccountAndProfile>> SearchAccountAndProfile(
            string? usernameLike,
            AccountKind? kind,
            string? studentId,
            string? startUsername,
            bool descending,
            bool searchNameUsingRegex = false,
            int take = 50
        ) {
            var query = db.AccountAndProfileView.AsQueryable();
            if (usernameLike != null && searchNameUsingRegex) {
                query = query.Where(p => Regex.IsMatch(p.Username, usernameLike));
            } else if (usernameLike != null && !searchNameUsingRegex) {
                query = query.Where(p => EF.Functions.Like(p.Username, usernameLike));
            }
            if (kind != null) {
                AccountKind kind_ = kind.Value;
                query = query.Where(p => p.Kind == kind_);
            }
            if (studentId != null) {
                query = query.Where(p => p.StudentId == studentId);
            }
            if (descending) {
                query = query.Where(p => p.Username.CompareTo(startUsername) < 0)
                    .OrderByDescending(p => p.Username);
            } else {
                startUsername ??= "";
                query = query.Where(p => p.Username.CompareTo(startUsername) > 0)
                    .OrderBy(p => p.Username);
            }
            query = query.Take(take);

            return await query.ToListAsync();
        }
    }
}
