using System;
using System.ComponentModel.DataAnnotations.Schema;

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
    /// The class used for storing long-lived Access Tokens and 
    /// Refresh Tokens for users
    /// </summary>
    public class TokenEntry {
        public TokenEntry(string username, string accessToken, string? tokenName, DateTimeOffset? expires) {
            Username = username;
            AccessToken = accessToken;
            TokenName = tokenName;
            Expires = expires;
        }

        public string Username { get; set; }
        public string AccessToken { get; set; }
        public string? TokenName { get; set; }
        public DateTimeOffset? Expires { get; set; }
    }
}
