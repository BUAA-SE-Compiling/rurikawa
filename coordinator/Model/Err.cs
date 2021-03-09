namespace Karenia.Rurikawa.Models {
    public class ErrorResponse {
        public ErrorResponse(string err, string? message = null) {
            Err = err;
            Message = message;
        }

        public string Err { get; set; }
        public string? Message { get; set; }
    }

    public static class ErrorCodes {
        public const string GIT_NO_SUCH_REVISION = "git_no_such_revision";
        public const string REVISION_FETCH_TIMEOUT = "revision_fetch_timeout";
        public const string NO_SUCH_SUITE = "no_such_suite";
        public const string NOT_IN_ACTIVE_TIMESPAN = "not_in_active_timespan";

        public const string NOT_OWNER = "not_owner";

        public const string INVALID_GRANT_TYPE = "invalid_grant_type";
        public const string INVALID_LOGIN_INFO = "invalid_login_info";
        public const string NOT_ENOUGH_LOGIN_INFO = "not_enough_login_info";

        public const string USERNAME_NOT_UNIQUE = "username_not_unique";
        public const string INVALID_USERNAME = "invalid_username";
        public const string ALREADY_INITIALIZED = "already_initialized";

        public const string JUDGER_NO_SUCH_REGISTER_TOKEN = "judger_no_such_register_token";
        public const string UNSPECIFIED_CONTENT_LENGTH = "unspecified_content_length";
        public const string INVALID_MESSAGE_TYPE = "invalid_message_type";
    }
}
