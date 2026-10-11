// N3485 5.3.4p18, 15.2p2, 15.3p2: an exception from an element constructor of
// an array new-expression, or of a local array, inside nested try blocks
// destroys the elements already constructed, frees the storage, and is caught
// by the innermost handler that matches it, also for a new-expression inside a
// handler
