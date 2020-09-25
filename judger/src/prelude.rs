pub type PopenResult<T> = Result<T, std::io::Error>;

pub mod cancel_token;
mod flowsnake;

pub use flowsnake::*;
