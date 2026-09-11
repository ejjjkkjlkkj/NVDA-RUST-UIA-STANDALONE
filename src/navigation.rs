use crate::semantic::AccessibleNode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectId(String);

impl ObjectId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ObjectId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ObjectId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Parent,
    FirstChild,
    PreviousSibling,
    NextSibling,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectRelations {
    pub parent: Option<ObjectId>,
    pub first_child: Option<ObjectId>,
    pub previous_sibling: Option<ObjectId>,
    pub next_sibling: Option<ObjectId>,
}

impl ObjectRelations {
    pub fn target(&self, direction: NavigationDirection) -> Option<&ObjectId> {
        match direction {
            NavigationDirection::Parent => self.parent.as_ref(),
            NavigationDirection::FirstChild => self.first_child.as_ref(),
            NavigationDirection::PreviousSibling => self.previous_sibling.as_ref(),
            NavigationDirection::NextSibling => self.next_sibling.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigableObject {
    pub id: ObjectId,
    pub node: AccessibleNode,
    pub relations: ObjectRelations,
}

impl NavigableObject {
    pub fn new(id: impl Into<ObjectId>, node: AccessibleNode) -> Self {
        Self {
            id: id.into(),
            node,
            relations: ObjectRelations::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectNavigator {
    current: Option<ObjectId>,
    follows_focus: bool,
}

impl Default for ObjectNavigator {
    fn default() -> Self {
        Self {
            current: None,
            follows_focus: true,
        }
    }
}

impl ObjectNavigator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<&ObjectId> {
        self.current.as_ref()
    }

    pub const fn follows_focus(&self) -> bool {
        self.follows_focus
    }

    pub fn set_follows_focus(&mut self, enabled: bool) {
        self.follows_focus = enabled;
    }

    pub fn set_current(&mut self, id: impl Into<ObjectId>) {
        self.current = Some(id.into());
    }

    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Synchronize the navigator with the system focus when focus tracking is enabled.
    ///
    /// The first observed focus also initializes an empty navigator even if tracking
    /// has already been disabled. This gives callers a deterministic starting object
    /// without forcing them to maintain a second initialization path.
    pub fn sync_focus(&mut self, focus_id: impl Into<ObjectId>) -> bool {
        if !self.follows_focus && self.current.is_some() {
            return false;
        }

        let focus_id = focus_id.into();
        if self.current.as_ref() == Some(&focus_id) {
            return false;
        }

        self.current = Some(focus_id);
        true
    }

    /// Resolve a navigation command from the currently inspected object's relations.
    ///
    /// The platform adapter owns tree traversal and object lookup. The portable core
    /// only decides which relation is requested and updates the independent navigator
    /// position after the adapter supplied that relation snapshot.
    pub fn move_from(
        &mut self,
        current_object: &NavigableObject,
        direction: NavigationDirection,
    ) -> Option<ObjectId> {
        if self.current.as_ref() != Some(&current_object.id) {
            return None;
        }

        let target = current_object.relations.target(direction)?.clone();
        self.current = Some(target.clone());
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{AccessibleNode, Role};

    fn object(id: &str) -> NavigableObject {
        NavigableObject::new(
            id,
            AccessibleNode {
                role: Role::Button,
                name: id.to_string(),
                ..AccessibleNode::default()
            },
        )
    }

    #[test]
    fn navigator_follows_focus_by_default() {
        let mut navigator = ObjectNavigator::new();
        assert!(navigator.follows_focus());
        assert!(navigator.sync_focus("editor"));
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("editor"));
        assert!(navigator.sync_focus("apply"));
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("apply"));
    }

    #[test]
    fn navigator_can_remain_independent_from_focus() {
        let mut navigator = ObjectNavigator::new();
        navigator.sync_focus("editor");
        navigator.set_follows_focus(false);

        assert!(!navigator.sync_focus("apply"));
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("editor"));
    }

    #[test]
    fn disabled_focus_tracking_still_initializes_empty_navigator() {
        let mut navigator = ObjectNavigator::new();
        navigator.set_follows_focus(false);

        assert!(navigator.sync_focus("initial-focus"));
        assert_eq!(
            navigator.current().map(ObjectId::as_str),
            Some("initial-focus")
        );
    }

    #[test]
    fn object_navigation_moves_without_changing_system_focus() {
        let mut root = object("root");
        root.relations.first_child = Some(ObjectId::from("first"));

        let mut navigator = ObjectNavigator::new();
        navigator.sync_focus("root");
        navigator.set_follows_focus(false);

        let target = navigator.move_from(&root, NavigationDirection::FirstChild);
        assert_eq!(target.as_ref().map(ObjectId::as_str), Some("first"));
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("first"));

        // A later system focus event is intentionally ignored.
        assert!(!navigator.sync_focus("unrelated-focus"));
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("first"));
    }

    #[test]
    fn parent_and_sibling_relations_are_directional() {
        let relations = ObjectRelations {
            parent: Some(ObjectId::from("parent")),
            first_child: Some(ObjectId::from("child")),
            previous_sibling: Some(ObjectId::from("previous")),
            next_sibling: Some(ObjectId::from("next")),
        };

        assert_eq!(
            relations
                .target(NavigationDirection::Parent)
                .map(ObjectId::as_str),
            Some("parent")
        );
        assert_eq!(
            relations
                .target(NavigationDirection::FirstChild)
                .map(ObjectId::as_str),
            Some("child")
        );
        assert_eq!(
            relations
                .target(NavigationDirection::PreviousSibling)
                .map(ObjectId::as_str),
            Some("previous")
        );
        assert_eq!(
            relations
                .target(NavigationDirection::NextSibling)
                .map(ObjectId::as_str),
            Some("next")
        );
    }

    #[test]
    fn unavailable_relation_does_not_move_navigator() {
        let current = object("only");
        let mut navigator = ObjectNavigator::new();
        navigator.sync_focus("only");

        assert_eq!(
            navigator.move_from(&current, NavigationDirection::NextSibling),
            None
        );
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("only"));
    }

    #[test]
    fn stale_relation_snapshot_cannot_move_current_navigator() {
        let mut stale = object("old");
        stale.relations.next_sibling = Some(ObjectId::from("wrong"));

        let mut navigator = ObjectNavigator::new();
        navigator.sync_focus("current");

        assert_eq!(
            navigator.move_from(&stale, NavigationDirection::NextSibling),
            None
        );
        assert_eq!(navigator.current().map(ObjectId::as_str), Some("current"));
    }

    #[test]
    fn clear_resets_navigation_position() {
        let mut navigator = ObjectNavigator::new();
        navigator.sync_focus("editor");
        navigator.clear();
        assert_eq!(navigator.current(), None);
    }
}
