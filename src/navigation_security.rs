/// Security context evaluated before the navigator is allowed to enter a
/// target object. Platform adapters are responsible for determining whether a
/// session is locked and whether a target belongs to content hidden behind the
/// lock surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NavigationSecurityContext {
    pub session_locked: bool,
    pub target_hidden_behind_lock_surface: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationSecurityDecision {
    Allow,
    DenyLockedSessionEscape,
}

pub const fn evaluate_navigation_target(
    context: NavigationSecurityContext,
) -> NavigationSecurityDecision {
    if context.session_locked && context.target_hidden_behind_lock_surface {
        NavigationSecurityDecision::DenyLockedSessionEscape
    } else {
        NavigationSecurityDecision::Allow
    }
}

pub const fn navigation_target_allowed(context: NavigationSecurityContext) -> bool {
    matches!(
        evaluate_navigation_target(context),
        NavigationSecurityDecision::Allow
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlocked_session_allows_normal_navigation() {
        assert!(navigation_target_allowed(NavigationSecurityContext {
            session_locked: false,
            target_hidden_behind_lock_surface: true,
        }));
    }

    #[test]
    fn locked_session_allows_objects_on_lock_surface() {
        assert!(navigation_target_allowed(NavigationSecurityContext {
            session_locked: true,
            target_hidden_behind_lock_surface: false,
        }));
    }

    #[test]
    fn locked_session_blocks_escape_to_hidden_desktop_content() {
        assert_eq!(
            evaluate_navigation_target(NavigationSecurityContext {
                session_locked: true,
                target_hidden_behind_lock_surface: true,
            }),
            NavigationSecurityDecision::DenyLockedSessionEscape
        );
    }
}
