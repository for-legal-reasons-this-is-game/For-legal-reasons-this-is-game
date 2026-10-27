use tradingengine::ids::CommandId;

// the ids are plain u128 wrappers now, the string checks that were here went
// away when CommandId changed from a string key to the contract's 128 bit Id

#[test]
fn distinct_command_ids_are_distinct_map_entries() {
    // the property the command cache needs: a retried command finds its
    // earlier entry and a new command does not
    let mut seen = std::collections::HashSet::new();
    assert!(seen.insert(CommandId::new(1)));
    assert!(seen.insert(CommandId::new(2)));
    assert!(!seen.insert(CommandId::new(1)));
    assert_eq!(seen.len(), 2);
}

#[test]
fn uses_all_128_bits() {
    // backend ids are UUIDs, a u64 would cut off the top half
    let id = CommandId::new(u128::MAX);
    assert_eq!(id.value(), u128::MAX);
}
