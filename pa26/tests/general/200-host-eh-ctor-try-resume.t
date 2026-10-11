// N3485 15.3p4, 15.3p15, 15.2p2: an exception from a member initializer or
// from the body of a constructor whose body is a function-try-block destroys
// the subobjects already constructed and enters the handler, which rethrows
// when it reaches its end
