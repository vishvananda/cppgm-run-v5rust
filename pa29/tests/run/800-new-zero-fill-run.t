// N3485 8.5p7, 8.5.4p3, 5.3.4p15: the elements of new T[n]() are
// value-initialized, and the elements of new T[n]{...} after those listed are
// value-initialized: scalars and classes without constructors are
// zero-initialized (a null pointer to data member is -1, Itanium C++ ABI 2.3),
// for counts up to 2^20
