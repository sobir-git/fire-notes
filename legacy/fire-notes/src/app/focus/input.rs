//! Focus input routing — Note: Focus no longer implements InputHandler.
//! All keyboard routing goes through AppLogic directly:
//!   - TabRename keys route through AppLogic.rename_input
//!   - NotesPicker keys route through AppLogic.notes_picker
//!   - Editor keys route through AppLogic.tabs[active_tab]
