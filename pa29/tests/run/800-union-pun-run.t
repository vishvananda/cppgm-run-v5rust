// N3485 9.5p1 as GCC and Clang define it: reading a union member other than
// the one last stored reads the stored bytes, so each read after a store
// through another member sees that store
