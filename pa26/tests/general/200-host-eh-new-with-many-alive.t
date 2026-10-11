// N3485 15.2p1, 5.3.4p18: with nine destructible objects alive, when anything
// in a full-expression containing a new-expression throws, each of those
// objects is destroyed once, in reverse order of construction; a countdown
// throws at each step in turn and the trace records the order
