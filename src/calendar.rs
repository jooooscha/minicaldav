use serde::{Deserialize, Serialize};
use url::Url;

/// A remote CalDAV calendar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    base_url: Url,
    inner: CalendarRef,
}

impl Calendar {
    pub fn new(base_url: Url, inner: CalendarRef) -> Self {
        Calendar { base_url, inner }
    }

    pub fn url(&self) -> &Url {
        &self.inner.url
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn name(&self) -> &String {
        &self.inner.name
    }
    pub fn color(&self) -> Option<&String> {
        self.inner.color.as_ref()
    }
    pub fn writable(&self) -> bool {
        self.inner.privileges.iter().any(|p| p == "write")
    }
    pub fn is_subscription(&self) -> bool {
        self.inner.is_subscription
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CalendarRef {
    pub url: Url,
    pub name: String,
    pub color: Option<String>,
    pub privileges: Vec<String>,
    pub is_subscription: bool,
}

impl std::fmt::Debug for CalendarRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalendarRef")
            .field("url", &self.url.to_string())
            .field("name", &self.name)
            .finish()
    }
}
