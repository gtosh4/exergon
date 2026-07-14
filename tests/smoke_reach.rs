//! End-to-end proof of the auto-generated **smoke scenario** path (`scenario_runner::run_smoke`):
//! given only a content id, derive a scenario from the matching e2e baseline and prove the content
//! is actually reachable in a real headless run — no hand-authored steps.
//!
//! The target is `circuit_board`: its crafting recipe lives in the initiation baseline's *terminal* block,
//! which the smoke generator truncates away, so reaching it forces the appended `Craft` step to
//! drive real production off the prefix economy (mined ore → smelt → wire → assemble). This is the
//! whole pipeline — resolve, pick the lowest difficulty, splice onto the baseline, run, verify.

use exergon::save::DifficultyTier;
use scenario_runner::{Target, run_smoke};

#[test]
fn smoke_reaches_a_craftable_item_on_the_lowest_difficulty() {
    let report = run_smoke(&Target::Item("circuit_board".into()), None)
        .expect("planning + preflight must succeed for a real, producible item");

    assert_eq!(
        report.difficulty,
        DifficultyTier::Initiation,
        "circuit_board is a low-tier item, so the smoke picks the lowest difficulty that covers it"
    );
    assert!(
        report.reached,
        "the appended Craft step must produce a circuit_board into the hub off the baseline economy"
    );
}
