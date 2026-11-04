#![allow(unused)]
// ex00: what the contract remake (#53, PR #56) changed for PlaceOrder
//
// Read first: the INTERNAL_ERROR comment in proto/common.proto and
// proto/service.proto, and the INTERNAL_ERROR rows of "Handling command
// responses" and step 2 of "Crash and shutdown" in proto/how_to_use.md.
//
// The PlaceOrder messages did not change. The one new thing is
// REJECT_REASON_INTERNAL_ERROR, and it is NOT a reason for an error that
// happens while a command runs.

use proto::RejectReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Place,
    Cancel,
    Modify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    // the command was taken and run, and something went wrong while running it
    WhileRunning,
    // the command was still waiting in the queue when the engine stopped
    StillQueued,
}

// when does the engine answer INTERNAL_ERROR?
pub fn internal_error_is_for() -> Moment {
    //>>
    Moment::StillQueued
    //<<
}

// is a response with this reject reason stored in the command cache?
// (UNSPECIFIED means accepted)
pub fn goes_in_command_cache(reason: RejectReason) -> bool {
    //>>
    reason != RejectReason::InternalError
    //<<
}

// the command came back with INTERNAL_ERROR. Is the order still working?
// (for a place: did the order ever exist?)
pub fn order_still_working_after_internal_error(command: Command) -> bool {
    //>>
    match command {
        Command::Place => false,
        Command::Cancel | Command::Modify => true,
    }
    //<<
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error_means_never_processed() {
        assert_eq!(internal_error_is_for(), Moment::StillQueued);
    }

    #[test]
    fn internal_error_is_never_cached() {
        assert!(!goes_in_command_cache(RejectReason::InternalError));
        assert!(goes_in_command_cache(RejectReason::Unspecified));
        assert!(goes_in_command_cache(RejectReason::BadPrice));
        assert!(goes_in_command_cache(RejectReason::DuplicateCommand));
    }

    #[test]
    fn a_place_with_internal_error_never_existed() {
        assert!(!order_still_working_after_internal_error(Command::Place));
        assert!(order_still_working_after_internal_error(Command::Cancel));
        assert!(order_still_working_after_internal_error(Command::Modify));
    }
}
