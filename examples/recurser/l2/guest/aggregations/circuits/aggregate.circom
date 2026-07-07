// AggregatePublics for the L2 example — folds two contiguous block ranges
// (A = older, B = newer). Publics are the ABI encoding of BlocksInfoStruct:
// 8 fields × 8 slots = 64. The field offsets below mirror common's SLOT_*.
template AggregatePublics(nPublicsAgg) {
    signal output aggregated_publics[nPublicsAgg];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];

    var START_BLOCK = 0;
    var END_BLOCK = 8;
    var GLOBAL_EXIT_ROOT = 16;
    var ACCOUNT_ROOT = 24;
    var DEPOSIT_ROOT = 32;
    var PRIORITY_EXIT_ROOT = 40;
    var OLD_GLOBAL_EXIT_ROOT = 48;
    var OLD_ACCOUNT_ROOT = 56;
    var WORD = 8; // slots per 32-byte field

    // Stitch: A chains into B (equality is slot-wise; both use the same encoding).
    for (var i = 0; i < WORD; i++) {
        a_publics[END_BLOCK + i]        === b_publics[START_BLOCK + i];         // contiguous
        a_publics[GLOBAL_EXIT_ROOT + i] === b_publics[OLD_GLOBAL_EXIT_ROOT + i]; // B's pre-state = A's post
        a_publics[ACCOUNT_ROOT + i]     === b_publics[OLD_ACCOUNT_ROOT + i];
    }

    // Merge: older-side values from A, newer-side (end + post-state) from B.
    for (var i = 0; i < WORD; i++) {
        aggregated_publics[START_BLOCK + i]          <== a_publics[START_BLOCK + i];
        aggregated_publics[OLD_GLOBAL_EXIT_ROOT + i] <== a_publics[OLD_GLOBAL_EXIT_ROOT + i];
        aggregated_publics[OLD_ACCOUNT_ROOT + i]     <== a_publics[OLD_ACCOUNT_ROOT + i];

        aggregated_publics[END_BLOCK + i]            <== b_publics[END_BLOCK + i];
        aggregated_publics[GLOBAL_EXIT_ROOT + i]     <== b_publics[GLOBAL_EXIT_ROOT + i];
        aggregated_publics[ACCOUNT_ROOT + i]         <== b_publics[ACCOUNT_ROOT + i];
        aggregated_publics[DEPOSIT_ROOT + i]         <== b_publics[DEPOSIT_ROOT + i];
        aggregated_publics[PRIORITY_EXIT_ROOT + i]   <== b_publics[PRIORITY_EXIT_ROOT + i];
    }
}
