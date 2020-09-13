namespace Karenia.Rurikawa.Models {
    public class ErrorResponse {
        public ErrorResponse(string err, string? message = null) {
            Err = err;
            Message = message;
        }

        public string Err { get; set; }
        public string? Message { get; set; }
    }
}
