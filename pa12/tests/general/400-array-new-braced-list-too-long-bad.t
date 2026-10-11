// N3485 5.3.4p7: a constant array new count smaller than the number of
// initializer-clauses in its braced-init-list makes the program ill-formed.
int* f() { return new int[2]{1, 2, 3}; }
