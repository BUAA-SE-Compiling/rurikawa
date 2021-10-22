pub fn is_recoverable_error(e: &bollard::errors::Error) -> bool {
    matches!(
        &e,
        bollard::errors::Error::JsonDataError { .. }
            | bollard::errors::Error::JsonSerdeError { .. }
            | bollard::errors::Error::StrParseError { .. }
            | bollard::errors::Error::StrFmtError { .. }
            | bollard::errors::Error::URLEncodedError { .. }
    )
}
