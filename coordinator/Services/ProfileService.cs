using System;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Microsoft.EntityFrameworkCore;

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
    }
}
