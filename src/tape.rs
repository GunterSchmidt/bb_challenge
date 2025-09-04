pub mod tape_128;
// pub mod tape_compact;
// pub mod tape_long_fixed_apex;
pub mod tape_long_shifted;
pub mod tape_utils;

use crate::{
    config::{Config, StepTypeBig},
    tape::tape_utils::TapeLongPositions,
    transition_binary::TransitionBinary,
};

/// This trait provided defined function for a tape. While the trait is not used directly, it
/// allows to switch tapes quickly in the deciders to do tests, e.g. performance or results.
pub trait Tape: std::fmt::Display {
    fn new(config: &Config) -> Self;

    /// Resets the tape for a new machine.
    fn clear(&mut self);

    /// Returns the ones which are set in the tape.
    fn count_ones(&self) -> u32;

    /// Returns the symbol at the head.
    fn get_current_symbol(&self) -> usize;

    /// Returns true if all bits left of the head and the head itself are 0.
    fn is_left_empty(&self) -> bool;

    /// Returns true if all bits right of the head and the head itself are 0.
    fn is_right_empty(&self) -> bool;

    fn left_64_bit(&self) -> u64;
    fn right_64_bit(&self) -> u64;

    // /// Update tape: write symbol at head position into cell
    // // TODO remove, replace with update_tape_single_step call, needs to cater for hold transition
    // #[deprecated]
    fn set_current_symbol(&mut self, transition: TransitionBinary);

    // /// If this tape supports speed up (self-ref) functionality
    // fn supports_speed_up(&self) -> bool;

    /// For HTML output, tape long positions if available.
    fn tape_long_positions(&self) -> Option<TapeLongPositions>;

    /// Returns the approximate tape size, which is actually not known exactly. \
    /// The high/low bound may indicate the actual used tape or may have shifted to the first 1 in that direction.
    fn tape_size_cells(&self) -> u32;

    /// Sets the symbol of the transition and moves the tape according to direction of the transition.
    /// Also prints and writes step to html if feature "enable_html_reports" is set.
    /// # Returns
    /// False if the tape bounds were reached and/or the tape could not be expanded (tape_size_limit). \
    /// In case of an error self.status is set to that error.
    #[must_use]
    fn update_tape_single_step(&mut self, transition: TransitionBinary) -> bool;

    fn update_tape_self_ref_speed_up_unused_or_used(
        &mut self,
        transition: TransitionBinary,
    ) -> bool;

    /// Current pos_middle. This is an optional value only to be used for html or debug output.
    #[cfg(feature = "enable_html_reports")]
    fn pos_middle_print(&self) -> i64;

    /// Tape as 128-Bit with head as bit 63. Displays the actual current bits, not the working tape_shifted.
    #[cfg(feature = "enable_html_reports")]
    fn tape_shifted_clean(&self) -> u128;
}

pub trait TapeSpeedUp: Tape {
    /// Sets the symbol of the transition and moves the tape according to direction of the transition.
    /// This also checks speed-up options (self-ref) and may move the tape many steps at once.
    /// Also prints and writes step to html if feature "enable_html_reports" is set.
    /// # Returns
    /// False if the tape bounds were reached and/or the tape could not be expanded (tape_size_limit). \
    /// In case of an error self.status is set to that error.
    #[must_use]
    fn update_tape_self_ref_speed_up(
        &mut self,
        transition: TransitionBinary,
        tr_field: usize,
    ) -> StepTypeBig;
}
