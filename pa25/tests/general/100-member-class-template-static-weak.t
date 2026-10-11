// N3485 14.7.1/3.2p6 + Itanium 5.2 (vague linkage): C<int>::I::b, a static
// data member of a member class of a class template, is instantiated in each
// TU that uses it and must be emitted weak/COMDAT
