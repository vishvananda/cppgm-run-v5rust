// N3485 15.3p4, 15.1p8: an exception thrown inside a handler is caught by a
// try block nested in the handler, whose catch (...) matches any exception;
// the handler then completes and the enclosing try block's handlers are not
// entered
