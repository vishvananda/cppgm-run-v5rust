// N3485 15.4p2: an array derivation cannot contain the specified function type.
void (*table[2])() throw(int);
