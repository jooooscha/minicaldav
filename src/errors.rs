/// Errors that may occur during CalDAV operations.
#[derive(Debug)]
pub enum MiniCaldavError {
    /// Could not find data `String` in PROPFIND response
    PathNotExists(String),
    CouldNotJoinUrl(String),
    RequestFailed(String),
    CouldNotParseXml(String),
    CouldNotParseTodo(String, String),
    CouldNotParseEvent(String, String),
}

impl From<url::ParseError> for MiniCaldavError {
    fn from(e: url::ParseError) -> Self {
        Self::CouldNotJoinUrl(e.to_string())
    }
}

impl From<reqwest::Error> for MiniCaldavError {
    fn from(e: reqwest::Error) -> Self {
        Self::RequestFailed(e.to_string())
    }
}

impl From<xmltree::ParseError> for MiniCaldavError {
    fn from(e: xmltree::ParseError) -> Self {
        Self::CouldNotParseXml(e.to_string())
    }
}
