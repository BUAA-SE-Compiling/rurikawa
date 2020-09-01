pub type PopenResult<T> = Result<T, std::io::Error>;
mod flowsnake;
pub use flowsnake::*;

