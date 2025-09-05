use std::fmt::Display;

use num_format::ToFormattedString;

use crate::config::{user_locale, StepTypeBig, StepTypeSmall};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PreDeciderReason {
    /// No Reason to eliminate machine found.
    None,
    NotAllStatesUsed,
    NotExactlyOneHaltCondition,
    NotStartStateBRight,
    OnlyOneDirection,
    SimpleStartCycle,
    StartRecursive,
    WritesOnlyZero,
}

/// Some defined reasons why the machine will never end.
// TODO Display
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NonHaltReason {
    /// Cycler (steps run, number of steps in the cycle)
    Cycler(StepTypeSmall, StepTypeSmall),
    /// Bouncer (steps run)
    Bouncer(StepTypeSmall),
    ExpandingBouncer(ExpandingBouncerReason),
    ExpandingCycler,

    // These have been moved to PreDeciderReason
    OnlyOneDirection,
    NoHaltTransition,
    SimpleStartCycle,
    /// Always comes back to start with left or right tape all 0, only extending to one side endlessly
    /// e.g. BB3: 84080
    StartRecursive,
    WritesOnlyZero,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UndecidedReason {
    DeciderNoResult,
    TapeLimitLeftBoundReached,
    TapeLimitRightBoundReached,
    NoSinusRhythmIdentified,
    StepLimit,
    TapeSizeLimit,
    Undefined,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ExpandingBouncerReason {
    DeciderNoResult,
    StepDeltaIdentical,
    StepDelta2ndRepeating,
    StepDelta2ndDoubles,
    StepDelta2ndCompressedRepeating,
    HeadMiddleExpanding,
    TapeValueDeltaAlternating,
    TapeValueDeltaIdentical,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum MachineStatus {
    #[default]
    NoDecision,
    DecidedNonHalt(NonHaltReason),
    /// Halt for fast evaluation
    DecidedHalts(StepTypeBig),
    /// Halts after steps, tape size, ones on tape
    DecidedHaltsDetail(StepTypeBig, u32, u32),
    DecidedNotMaxTooManyHaltTransitions,
    DecidedNotMaxNotAllStatesUsed,
    EliminatedPreDecider(PreDeciderReason),
    /// UndecidedReason, stopped after steps, tape size in cells
    Undecided(UndecidedReason, StepTypeBig, u32),
    // UndecidedFastTapeBoundReached,
}

impl MachineStatus {
    pub fn is_bouncer(&self) -> bool {
        if let MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(_)) = self {
            true
        } else {
            false
        }
    }

     pub fn is_cycler(&self) -> bool {
        if let MachineStatus::DecidedNonHalt(NonHaltReason::Cycler(_,_)) = self {
            true
        } else {
            false
        }
    }
}

impl Display for MachineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let locale = user_locale();
        let mut s = String::new();
        match self {
            MachineStatus::DecidedHalts(steps) => s.push_str(
                format!(
                    "Decided: Halts after {} steps",
                    steps.to_formatted_string(&locale)
                )
                .as_str(),
            ),
            MachineStatus::EliminatedPreDecider(reason) => {
                s.push_str(format!("Eliminated Pre-Decider {reason:?}").as_str())
            }
            MachineStatus::NoDecision => s.push_str("No decision"),
            MachineStatus::DecidedNonHalt(endless_reason) => {
                s.push_str(format!("Decided: Endless for {endless_reason:?}").as_str())
            }
            MachineStatus::DecidedNotMaxTooManyHaltTransitions => todo!(),
            MachineStatus::DecidedNotMaxNotAllStatesUsed => {
                s.push_str("Decided: Not max as not all states are used.")
            }
            MachineStatus::DecidedHaltsDetail(steps, tape_size, ones) => s.push_str(
                format!(
                    "Decided: Halts after {} steps, {ones} ones written, tape_size (approx): {tape_size}",
                    steps.to_formatted_string(&locale)
                )
                .as_str(),
            ),
            MachineStatus::Undecided(reason, steps, tape_size_limit) => {
                match reason {
                            UndecidedReason::DeciderNoResult => s.push_str("Undecided: No result"),
                            UndecidedReason::TapeLimitLeftBoundReached => s.push_str(
                                format!("Undecided: Tape bound reached (right {tape_size_limit} steps) after {steps} steps").as_str(),
                            ),
                            UndecidedReason::TapeLimitRightBoundReached => s.push_str(
                                format!("Undecided: Tape bound reached (left {tape_size_limit} steps) after {steps} steps").as_str(),
                            ),
                            UndecidedReason::StepLimit => s.push_str(
                                format!(
                                    "Undecided: Step limit reached, machine did not halt for {steps} steps."
                                )
                                .as_str(),
                            ),
                            UndecidedReason::TapeSizeLimit => {
                                let s_limit =if *tape_size_limit > 128{
                                format!("Undecided: Tape size limit {tape_size_limit} (blocks: {}) reached: {steps} steps", 
                                tape_size_limit.div_ceil(32))
                                    
                                } else {
                                format!("Undecided: Tape size or bound limit {tape_size_limit} reached: {steps} steps")
                                };
                                    s.push_str(&s_limit)
                            }
                            UndecidedReason::Undefined => todo!(),
                            UndecidedReason::NoSinusRhythmIdentified => {
                                s.push_str(
                                                    format!("Undecided: No sinus rhythm reached: left {steps} steps").as_str(),
                                                )
                            },
                        }
                // s.push_str(format!(
                // "Safety stop reached, machine did not halt for {steps} steps or tape length limit {tape_len}").as_str());
            } // MachineStatus::UndecidedFastTapeBoundReached => {
              //     s.push_str("Undecided as fast tape size limit was reached.")
              // }
        }
        write!(f, "{s}")
    }
}
