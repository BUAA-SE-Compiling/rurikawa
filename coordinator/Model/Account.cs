using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations.Schema;
using System.Diagnostics;

#pragma warning disable CS8618
namespace Karenia.Rurikawa.Models.Account {
    public enum AccountKind {
        User,
        Admin,
        Root,
    }

    public class UserAccount {
        public string Username { get; set; }

        public string HashedPassword { get; set; }

        public AccountKind Kind { get; set; }
    }


    /// <summary>
    /// The class used for storing user profiles.
    /// </summary>
    public class Profile {
        public string Username { get; set; }
        public string? Email { get; set; }
        public string? StudentId { get; set; }
    }

    /// <summary>
    /// A combination of <c>UserAccount</c> and <c>Profile</c>, in order to provide
    /// some basic account information.
    /// </summary>
    public class AccountAndProfile {
        public string Username { get; set; }
        public string? Email { get; set; }
        public string? StudentId { get; set; }
        public AccountKind Kind { get; set; }
    }

    /// <summary>
    /// A base class for token-related stuff
    /// </summary>
    public class TokenBase {
        public string Token { get; set; }
        public DateTimeOffset IssuedTime { get; set; }
        public DateTimeOffset? Expires { get; set; }
        public bool IsSingleUse { get; set; } = false;
        public DateTimeOffset? LastUseTime { get; set; }

        public bool IsExpired() {
            DateTimeOffset now = DateTimeOffset.Now;
            if (Expires <= now) return true;
            if (IsSingleUse
                && LastUseTime.HasValue
                && LastUseTime.Value.Add(SingleUseTokenGracePeriod) <= now) return true;
            return false;
        }

        public static readonly TimeSpan SingleUseTokenGracePeriod
            = TimeSpan.FromMinutes(5);
    }

    /// <summary>
    /// The class used for storing long-lived Access Tokens and 
    /// Refresh Tokens for users
    /// </summary>
    public class TokenEntry : TokenBase {
        public string Username { get; set; }
        public string? TokenName { get; set; }
        public List<string> Scope { get; set; } = new List<string>();
        public string? RelatedToken { get; set; }
    }

    public class AccessTokenEntry : TokenEntry { }
    public class RefreshTokenEntry : TokenEntry { }

    public class JudgerTokenEntry : TokenBase {
        public List<string> Tags { get; set; } = new List<string>();
    }
}

