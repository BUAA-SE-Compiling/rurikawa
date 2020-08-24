using System;
using System.ComponentModel.DataAnnotations.Schema;

#pragma warning disable CS8618  
namespace Karenia.Rurikawa.Models.Account {
    public enum AccountKind {
        User,
        Admin,
        Root,
    }

    public class Account {
        public string Username { get; set; }

        public byte[] HashedPassword { get; set; }

        public byte[] Salt { get; set; }

        public AccountKind Kind { get; set; }
    }


    /// <summary>
    /// The class used for storing user profiles.
    /// </summary>
    public class UserProfile {
        public string Username { get; set; }
        public string? Email { get; set; }
        public string? StudentId { get; set; }
    }

    /// <summary>
    /// The class used for storing long-lived Access Tokens and 
    /// Refresh Tokens for users
    /// </summary>
    public class TokenEntry {
        public TokenEntry(string username, string accessToken) {
            Username = username;
            AccessToken = accessToken;
        }

        public string Username { get; set; }
        public string AccessToken { get; set; }
        public DateTimeOffset? Expires { get; set; }
    }
}
