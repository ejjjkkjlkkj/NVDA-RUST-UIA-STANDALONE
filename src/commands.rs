use crate::navigation::NavigationDirection;

/// User intents understood by the screen-reader core.
///
/// Keyboard, braille, touch and future remote-control adapters translate their
/// native gestures into these commands. Platform accessibility backends never
/// own gesture semantics directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenReaderCommand {
    StopSpeech,
    ReportFocus,
    ReportNavigator,
    MoveNavigatorToFocus,
    NavigatorParent,
    NavigatorFirstChild,
    NavigatorPreviousSibling,
    NavigatorNextSibling,
    NavigatorPreviousInFlow,
    NavigatorNextInFlow,
    ActivateNavigator,
}

impl ScreenReaderCommand {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::StopSpeech => "stop_speech",
            Self::ReportFocus => "report_focus",
            Self::ReportNavigator => "report_navigator",
            Self::MoveNavigatorToFocus => "navigator_to_focus",
            Self::NavigatorParent => "navigator_parent",
            Self::NavigatorFirstChild => "navigator_first_child",
            Self::NavigatorPreviousSibling => "navigator_previous_sibling",
            Self::NavigatorNextSibling => "navigator_next_sibling",
            Self::NavigatorPreviousInFlow => "navigator_previous_in_flow",
            Self::NavigatorNextInFlow => "navigator_next_in_flow",
            Self::ActivateNavigator => "activate_navigator",
        }
    }

    pub const fn hierarchy_direction(self) -> Option<NavigationDirection> {
        match self {
            Self::NavigatorParent => Some(NavigationDirection::Parent),
            Self::NavigatorFirstChild => Some(NavigationDirection::FirstChild),
            Self::NavigatorPreviousSibling => Some(NavigationDirection::PreviousSibling),
            Self::NavigatorNextSibling => Some(NavigationDirection::NextSibling),
            _ => None,
        }
    }

    pub const fn changes_navigator(self) -> bool {
        matches!(
            self,
            Self::MoveNavigatorToFocus
                | Self::NavigatorParent
                | Self::NavigatorFirstChild
                | Self::NavigatorPreviousSibling
                | Self::NavigatorNextSibling
                | Self::NavigatorPreviousInFlow
                | Self::NavigatorNextInFlow
        )
    }

    pub const fn is_read_only(self) -> bool {
        !matches!(self, Self::ActivateNavigator)
    }
}

/// Stable gesture identifiers used by configuration and platform input
/// adapters. They intentionally describe physical intent rather than UIA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GestureId {
    Escape,
    ScreenReaderBackspace,
    ScreenReaderShiftLeft,
    ScreenReaderShiftRight,
    ScreenReaderShiftUp,
    ScreenReaderShiftDown,
    ScreenReaderShiftLeftBracket,
    ScreenReaderShiftRightBracket,
    ScreenReaderEnter,
}

pub const fn default_laptop_command(gesture: GestureId) -> Option<ScreenReaderCommand> {
    match gesture {
        GestureId::Escape => Some(ScreenReaderCommand::StopSpeech),
        GestureId::ScreenReaderBackspace => Some(ScreenReaderCommand::MoveNavigatorToFocus),
        GestureId::ScreenReaderShiftLeft => Some(ScreenReaderCommand::NavigatorPreviousSibling),
        GestureId::ScreenReaderShiftRight => Some(ScreenReaderCommand::NavigatorNextSibling),
        GestureId::ScreenReaderShiftUp => Some(ScreenReaderCommand::NavigatorParent),
        GestureId::ScreenReaderShiftDown => Some(ScreenReaderCommand::NavigatorFirstChild),
        GestureId::ScreenReaderShiftLeftBracket => Some(ScreenReaderCommand::NavigatorPreviousInFlow),
        GestureId::ScreenReaderShiftRightBracket => Some(ScreenReaderCommand::NavigatorNextInFlow),
        GestureId::ScreenReaderEnter => Some(ScreenReaderCommand::ActivateNavigator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchical_commands_map_to_navigation_directions() {
        assert_eq!(
            ScreenReaderCommand::NavigatorParent.hierarchy_direction(),
            Some(NavigationDirection::Parent)
        );
        assert_eq!(
            ScreenReaderCommand::NavigatorFirstChild.hierarchy_direction(),
            Some(NavigationDirection::FirstChild)
        );
        assert_eq!(
            ScreenReaderCommand::NavigatorPreviousSibling.hierarchy_direction(),
            Some(NavigationDirection::PreviousSibling)
        );
        assert_eq!(
            ScreenReaderCommand::NavigatorNextSibling.hierarchy_direction(),
            Some(NavigationDirection::NextSibling)
        );
    }

    #[test]
    fn flattened_navigation_is_not_misrepresented_as_direct_tree_relation() {
        assert_eq!(
            ScreenReaderCommand::NavigatorPreviousInFlow.hierarchy_direction(),
            None
        );
        assert_eq!(
            ScreenReaderCommand::NavigatorNextInFlow.hierarchy_direction(),
            None
        );
    }

    #[test]
    fn default_laptop_object_navigation_matches_reference_intent() {
        assert_eq!(
            default_laptop_command(GestureId::ScreenReaderShiftLeft),
            Some(ScreenReaderCommand::NavigatorPreviousSibling)
        );
        assert_eq!(
            default_laptop_command(GestureId::ScreenReaderShiftRight),
            Some(ScreenReaderCommand::NavigatorNextSibling)
        );
        assert_eq!(
            default_laptop_command(GestureId::ScreenReaderShiftUp),
            Some(ScreenReaderCommand::NavigatorParent)
        );
        assert_eq!(
            default_laptop_command(GestureId::ScreenReaderShiftDown),
            Some(ScreenReaderCommand::NavigatorFirstChild)
        );
    }

    #[test]
    fn focus_sync_and_activation_are_distinct_commands() {
        assert!(ScreenReaderCommand::MoveNavigatorToFocus.changes_navigator());
        assert!(ScreenReaderCommand::MoveNavigatorToFocus.is_read_only());
        assert!(!ScreenReaderCommand::ActivateNavigator.changes_navigator());
        assert!(!ScreenReaderCommand::ActivateNavigator.is_read_only());
    }

    #[test]
    fn command_names_are_stable_and_platform_neutral() {
        assert_eq!(ScreenReaderCommand::StopSpeech.canonical_name(), "stop_speech");
        assert_eq!(
            ScreenReaderCommand::NavigatorNextInFlow.canonical_name(),
            "navigator_next_in_flow"
        );
        assert_eq!(
            ScreenReaderCommand::ActivateNavigator.canonical_name(),
            "activate_navigator"
        );
    }
}
