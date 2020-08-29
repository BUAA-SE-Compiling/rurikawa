pub type PopenResult<T> = Result<T, std::io::Error>;
pub mod flowsnake;
pub use flowsnake::*;
