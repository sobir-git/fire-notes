#![allow(dead_code)]
//! Universal event return type for widget event handlers.

/// Returned by every widget event handler.
///
/// `Msg` is the widget-specific business message — only cross-boundary events,
/// never internal navigation (scroll, selection, cursor movement).
///
/// `Pass` is the only non-consuming variant.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResponse<Msg = ()> {
    /// Not this widget's concern — propagate to the next handler.
    Pass,
    /// Consumed internally, no redraw needed.
    Handled,
    /// Consumed internally, redraw needed.
    Redraw,
    /// Produced a business message for the parent (implies redraw).
    Emit(Msg),
}

impl<Msg> EventResponse<Msg> {
    /// True if any variant other than `Pass` — the widget consumed the event.
    pub fn is_consumed(&self) -> bool {
        !matches!(self, Self::Pass)
    }

    /// True if the frame should be redrawn after this response.
    pub fn needs_redraw(&self) -> bool {
        matches!(self, Self::Redraw | Self::Emit(_))
    }

    /// Map the message type, preserving all other variants unchanged.
    pub fn map<M2>(self, f: impl FnOnce(Msg) -> M2) -> EventResponse<M2> {
        match self {
            Self::Pass        => EventResponse::Pass,
            Self::Handled     => EventResponse::Handled,
            Self::Redraw      => EventResponse::Redraw,
            Self::Emit(msg)   => EventResponse::Emit(f(msg)),
        }
    }

    /// Cast the message type via `Into`.
    pub fn cast<M2: From<Msg>>(self) -> EventResponse<M2> {
        self.map(Into::into)
    }

    /// Drop the message, keeping the consumed/redraw signal.
    /// Useful when a child emits a message the parent handles but
    /// needs to return an untyped result to its own parent.
    pub fn ignore_msg(self) -> EventResponse<()> {
        match self {
            Self::Pass      => EventResponse::Pass,
            Self::Handled   => EventResponse::Handled,
            Self::Redraw    => EventResponse::Redraw,
            Self::Emit(_)   => EventResponse::Redraw,
        }
    }
}
