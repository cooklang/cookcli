//! Who is looking at a page, as far as the templates need to know.
//!
//! `cook server` signs people in only when a users file is configured (see
//! `server::auth`). Templates never see that machinery: they get a
//! [`Viewer`] and ask it whether to show the controls that change things.
//! Kept outside the `server` feature because the recipe templates, and so this
//! type, also compile for `cook build`, which renders with the default.

/// The person a page is rendered for.
///
/// The default is an open server: sign-in is off and everyone may edit, which
/// is also what the static site renderer uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Viewer {
    auth_enabled: bool,
    user: Option<String>,
}

impl Viewer {
    /// Sign-in is off: everyone may edit.
    #[cfg(feature = "server")]
    pub fn open() -> Self {
        Self::default()
    }

    /// Sign-in is on and this visitor has not signed in.
    #[cfg(feature = "server")]
    pub fn guest() -> Self {
        Self {
            auth_enabled: true,
            user: None,
        }
    }

    /// Sign-in is on and this visitor is signed in as `user`.
    #[cfg(feature = "server")]
    pub fn signed_in(user: impl Into<String>) -> Self {
        Self {
            auth_enabled: true,
            user: Some(user.into()),
        }
    }

    /// Whether the page should offer controls that change recipes, the
    /// pantry or the shopping list.
    pub fn can_edit(&self) -> bool {
        !self.auth_enabled || self.user.is_some()
    }

    /// Whether the server asks people to sign in at all, which decides if the
    /// navigation shows a sign-in link.
    pub fn auth_enabled(&self) -> bool {
        self.auth_enabled
    }

    /// Whether someone is signed in.
    pub fn is_signed_in(&self) -> bool {
        self.user.is_some()
    }

    /// The signed-in user's name, or an empty string for a guest.
    pub fn username(&self) -> &str {
        self.user.as_deref().unwrap_or_default()
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn open_server_lets_everyone_edit() {
        let viewer = Viewer::default();
        assert_eq!(viewer, Viewer::open());
        assert!(viewer.can_edit());
        assert!(!viewer.auth_enabled());
        assert!(!viewer.is_signed_in());
    }

    #[test]
    fn guest_cannot_edit() {
        let viewer = Viewer::guest();
        assert!(!viewer.can_edit());
        assert!(viewer.auth_enabled());
        assert_eq!(viewer.username(), "");
    }

    #[test]
    fn signed_in_user_can_edit() {
        let viewer = Viewer::signed_in("alice");
        assert!(viewer.can_edit());
        assert!(viewer.is_signed_in());
        assert_eq!(viewer.username(), "alice");
    }
}
