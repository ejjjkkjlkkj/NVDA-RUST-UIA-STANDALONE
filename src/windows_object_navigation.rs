use crate::navigation::NavigationDirection;
use windows::Win32::{IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker};
use windows_core::Result;

/// Windows UI Automation adapter for object navigation.
///
/// The portable navigator owns the logical navigation position. This adapter
/// only resolves a requested relation in the UIA Control View and never moves
/// keyboard focus or invokes an element.
pub struct UiaControlWalker {
    walker: IUIAutomationTreeWalker,
}

impl UiaControlWalker {
    pub fn new(automation: &IUIAutomation) -> Result<Self> {
        let walker = unsafe { automation.ControlViewWalker()? };
        Ok(Self { walker })
    }

    pub fn related_element(
        &self,
        element: &IUIAutomationElement,
        direction: NavigationDirection,
    ) -> Result<IUIAutomationElement> {
        unsafe {
            match direction {
                NavigationDirection::Parent => self.walker.GetParentElement(element),
                NavigationDirection::FirstChild => self.walker.GetFirstChildElement(element),
                NavigationDirection::PreviousSibling => {
                    self.walker.GetPreviousSiblingElement(element)
                }
                NavigationDirection::NextSibling => self.walker.GetNextSiblingElement(element),
            }
        }
    }

    /// Convenience form for exploratory navigation where an absent relation is
    /// not exceptional. Provider/COM failures are deliberately collapsed here;
    /// callers that need diagnostics should use `related_element` instead.
    pub fn try_related_element(
        &self,
        element: &IUIAutomationElement,
        direction: NavigationDirection,
    ) -> Option<IUIAutomationElement> {
        self.related_element(element, direction).ok()
    }
}
