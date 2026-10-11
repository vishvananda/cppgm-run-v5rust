// N3485 15.2p1: when an exception leaves the initializer of a declared object,
// every automatic object constructed since the try block was entered is
// destroyed exactly once, whichever path the initializer took: GNU statement
// expressions in a conditional's or a logical operator's operand, a
// delete-expression whose destructor throws, a new-expression whose allocation
// function returns null; a countdown throws at each step in turn and the trace
// records the order
