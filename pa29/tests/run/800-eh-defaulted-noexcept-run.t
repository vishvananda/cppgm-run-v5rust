// N3485 8.4.2p2, 15.4p14, 12.4p3: a special member function explicitly
// defaulted on its first declaration has the implicit declaration's
// exception-specification; one defaulted after its first declaration keeps
// that declaration's (none: potentially throwing), and so do the implicit
// members that call it; a destructor without one is noexcept wherever it is
// defaulted. The GNU traits agree
