//! Who is looking at a page, as far as the templates need to know.
//!
//! `cook server` signs people in only when a users file is configured (see
//! `server::auth`). Templates never see that machinery: they get a
//! [`Viewer`] and ask it whether to show the controls that change things.
//! Kept outside the `server` feature because the recipe templates, and so this
//! type, also compile for `cook build`, which renders with the default.

use std::fmt;
use std::str::FromStr;

/// What a signed-in user may do, set per user in the users file.
///
/// Each role can do everything the ones before it can.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Reads, like a guest, but signed in.
    Reader,
    /// Also changes the shopping list and the pantry.
    Shopper,
    /// Also creates, edits and deletes recipes and menus.
    Editor,
    /// Also links cook.md sync and uses the editor's language server. What a
    /// user without a role in the users file is.
    Admin,
}

impl Role {
    /// Every role, least to most capable.
    pub const ALL: [Role; 4] = [Role::Reader, Role::Shopper, Role::Editor, Role::Admin];

    /// The name the users file and `cook server user` use.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Reader => "reader",
            Role::Shopper => "shopper",
            Role::Editor => "editor",
            Role::Admin => "admin",
        }
    }

    /// Whether this role may do `capability`.
    pub fn allows(self, capability: Capability) -> bool {
        self >= capability.minimum_role()
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Role::ALL
            .into_iter()
            .find(|role| role.as_str() == name)
            .ok_or_else(|| {
                let names: Vec<_> = Role::ALL.iter().map(|role| role.as_str()).collect();
                format!("unknown role {name:?}: use {}", names.join(", "))
            })
    }
}

/// Something a request may need the viewer to be allowed to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    /// Change the shopping list or the pantry.
    EditLists,
    /// Create, edit or delete recipes, menus and title pictures, and open the
    /// editor.
    EditRecipes,
    /// Link or unlink cook.md sync, and run the editor's language server.
    Administer,
}

impl Capability {
    /// The least capable role that may do this.
    pub fn minimum_role(self) -> Role {
        match self {
            Capability::EditLists => Role::Shopper,
            Capability::EditRecipes => Role::Editor,
            Capability::Administer => Role::Admin,
        }
    }
}

/// The person a page is rendered for.
///
/// The default is an open server: sign-in is off and everyone may do
/// everything, which is also what the static site renderer uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Viewer {
    auth_enabled: bool,
    user: Option<(String, Role)>,
}

impl Viewer {
    /// Sign-in is off: everyone may do everything.
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

    /// Sign-in is on and this visitor is signed in as `user`, who has `role`.
    #[cfg(feature = "server")]
    pub fn signed_in(user: impl Into<String>, role: Role) -> Self {
        Self {
            auth_enabled: true,
            user: Some((user.into(), role)),
        }
    }

    /// Whether this viewer may do `capability`.
    pub fn can(&self, capability: Capability) -> bool {
        match &self.user {
            Some((_, role)) => role.allows(capability),
            None => !self.auth_enabled,
        }
    }

    /// Whether the page should offer controls that change the shopping list
    /// or the pantry.
    pub fn can_edit_lists(&self) -> bool {
        self.can(Capability::EditLists)
    }

    /// Whether the page should offer controls that create, edit or delete
    /// recipes and menus.
    pub fn can_edit_recipes(&self) -> bool {
        self.can(Capability::EditRecipes)
    }

    /// Whether the page should offer the cook.md sync controls and connect
    /// the editor to the language server.
    pub fn can_admin(&self) -> bool {
        self.can(Capability::Administer)
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
        self.user.as_ref().map_or("", |(name, _)| name.as_str())
    }

    /// The signed-in user's role, or `None` for a guest or an open server.
    pub fn role(&self) -> Option<Role> {
        self.user.as_ref().map(|(_, role)| *role)
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    const EVERYTHING: [Capability; 3] = [
        Capability::EditLists,
        Capability::EditRecipes,
        Capability::Administer,
    ];

    #[test]
    fn open_server_lets_everyone_do_everything() {
        let viewer = Viewer::default();
        assert_eq!(viewer, Viewer::open());
        for capability in EVERYTHING {
            assert!(viewer.can(capability), "{capability:?}");
        }
        assert!(!viewer.auth_enabled());
        assert!(!viewer.is_signed_in());
        assert_eq!(viewer.role(), None);
    }

    #[test]
    fn guest_can_do_nothing() {
        let viewer = Viewer::guest();
        for capability in EVERYTHING {
            assert!(!viewer.can(capability), "{capability:?}");
        }
        assert!(viewer.auth_enabled());
        assert_eq!(viewer.username(), "");
    }

    #[test]
    fn each_role_adds_to_the_one_before() {
        let can = |role| {
            let viewer = Viewer::signed_in("alice", role);
            [
                viewer.can_edit_lists(),
                viewer.can_edit_recipes(),
                viewer.can_admin(),
            ]
        };
        assert_eq!(can(Role::Reader), [false, false, false]);
        assert_eq!(can(Role::Shopper), [true, false, false]);
        assert_eq!(can(Role::Editor), [true, true, false]);
        assert_eq!(can(Role::Admin), [true, true, true]);
    }

    #[test]
    fn signed_in_user_is_named() {
        let viewer = Viewer::signed_in("alice", Role::Editor);
        assert!(viewer.is_signed_in());
        assert_eq!(viewer.username(), "alice");
        assert_eq!(viewer.role(), Some(Role::Editor));
    }

    #[test]
    fn role_names_round_trip() {
        for role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>(), Ok(role));
        }
        let err = "Editor".parse::<Role>().unwrap_err();
        assert!(err.contains("reader, shopper, editor, admin"), "{err}");
    }
}
