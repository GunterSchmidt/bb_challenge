use std::fmt::Display;

use crate::StepType;

// use crate::{machine::MachineInfo, permutation::Permutation, turing::StepType};

pub const COUNTER_ARRAY_SIZE: usize = 110;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PreDeciderReason {
    /// No Reason to eliminate machine found.
    None,
    NotAllStatesUsed,
    NotExactlyOneHoldCondition,
    OnlyOneDirection,
    SimpleStartLoop,
    StartRecursive,
    StartStateBandRight,
    WritesOnlyZero,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EndlessReason {
    /// Loop (steps run, number of steps in the loop)
    Loop(StepType, StepType),
    ExpandingSinus(ExpandingSinusReason),
    ExpandingLoop,

    // These have been moved to PreDeciderReason
    OnlyOneDirection,
    NoHoldTransition,
    SimpleStartLoop,
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
pub enum ExpandingSinusReason {
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
    // Running,
    DecidedEndless(EndlessReason),
    /// Hold for fast evaluation
    DecidedHolds(StepType),
    /// Holds after steps, tape size, ones on tape
    DecidedHoldsDetail(StepType, usize, usize),
    DecidedNotMaxTooManyHoldTransitions,
    DecidedNotMaxNotAllStatesUsed,
    EliminatedPreDecider(PreDeciderReason),
    Undecided(UndecidedReason, StepType, usize),
    // UndecidedFastTapeBoundReached,
}

impl Display for MachineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        match self {
            MachineStatus::DecidedHolds(steps) => {
                s.push_str(format!("Decided Holds: after {} steps", steps).as_str())
            }
            MachineStatus::EliminatedPreDecider(reason) => {
                s.push_str(format!("Eliminated Pre-Decider {:?}", reason).as_str())
            }
            // MachineStatus::DecidedHoldsOld(steps, num_ones) => {
            //     s.push_str(
            //         format!(
            //             "Decided Holds: after {} steps, Ones on tape: {}",
            //             steps, num_ones
            //         )
            //         .as_str(),
            //     );
            // }
            MachineStatus::NoDecision => s.push_str("No decision"),
            MachineStatus::DecidedEndless(endless_reason) => {
                s.push_str(format!("Decided Endless for {:?}", endless_reason).as_str())
            }
            MachineStatus::DecidedNotMaxTooManyHoldTransitions => todo!(),
            MachineStatus::DecidedNotMaxNotAllStatesUsed => {
                s.push_str("Decided not max as not all states are used.")
            }
            MachineStatus::DecidedHoldsDetail(_, _, _) => todo!(),
            MachineStatus::Undecided(reason, steps, tape_size_limit) => {
                match reason {
                            UndecidedReason::DeciderNoResult => s.push_str("Undecided: No result"),
                            UndecidedReason::TapeLimitLeftBoundReached => s.push_str(
                                format!("Undecided: Tape bound reached (right 64 steps), {steps} steps").as_str(),
                            ),
                            UndecidedReason::TapeLimitRightBoundReached => s.push_str(
                                format!("Undecided: Tape bound reached (left 64 steps), {steps} steps").as_str(),
                            ),
                            UndecidedReason::StepLimit => s.push_str(
                                format!(
                                    "Undecided: Step limit reached, machine did not hold for {steps} steps."
                                )
                                .as_str(),
                            ),
                            UndecidedReason::TapeSizeLimit => s.push_str(
                                format!("Undecided: Tape Size Limit {tape_size_limit} reached: left {steps} steps")
                                    .as_str(),
                            ),
                            UndecidedReason::Undefined => todo!(),
                            UndecidedReason::NoSinusRhythmIdentified => {
                                s.push_str(
                                                    format!("Undecided: No sinus rhythm reached: left {steps} steps").as_str(),
                                                )
                            },
                        }
                // s.push_str(format!(
                // "Safety stop reached, machine did not hold for {steps} steps or tape length limit {tape_len}").as_str());
            } // MachineStatus::UndecidedFastTapeBoundReached => {
              //     s.push_str("Undecided as fast tape size limit was reached.")
              // }
        }
        write!(f, "{}", s)
    }
}
