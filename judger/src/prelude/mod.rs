pub type PopenResult<T> = Result<T, std::io::Error>;

mod cancel_token;
mod flowsnake;

pub use cancel_token::*;
pub use flowsnake::*;
