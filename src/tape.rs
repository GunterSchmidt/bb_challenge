use crate::{
    config::{Config, StepTypeBig},
    tape_utils::TapeLongPositions,
    transition_symbol2::TransitionSymbol2,
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

    /// Current pos_middle. This is an optional value only to be used for html or debug output.
    fn pos_middle(&self) -> u32;

    /// Update tape: write symbol at head position into cell
    fn set_current_symbol(&mut self, transition: TransitionSymbol2);

    /// For HTML output, tape long positions if available.
    fn tape_long_positions(&self) -> Option<TapeLongPositions>;

    /// Tape as 128-Bit with head as bit 63. Displays the actual current bits, not the working tape_shifted.
    fn tape_shifted_clean(&self) -> u128;

    /// Returns the approximate tape size, which is actually not known exactly. \
    /// The high/low bound may indicate the actual used tape or may have shifted to the first 1 in that direction.
    fn tape_size_cells(&self) -> u32;

    /// Sets the symbol of the transition and moves the tape according to direction of the transition.
    /// Also prints and writes step to html if feature "bb_enable_html_reports" is set.
    /// # Returns
    /// False if the tape bounds were reached and/or the tape could not be expanded (tape_size_limit). \
    /// In case of an error self.status is set to that error.
    #[must_use]
    fn update_tape_single_step(&mut self, transition: TransitionSymbol2) -> bool;

    /// Sets the symbol of the transition and moves the tape according to direction of the transition.
    /// This also checks speed-up options (self-ref) and may move the tape many steps at once.
    /// Also prints and writes step to html if feature "bb_enable_html_reports" is set.
    /// # Returns
    /// False if the tape bounds were reached and/or the tape could not be expanded (tape_size_limit). \
    /// In case of an error self.status is set to that error.
    #[must_use]
    fn update_tape_self_ref_speed_up(
        &mut self,
        transition: TransitionSymbol2,
        tr_field: usize,
    ) -> StepTypeBig;
}
