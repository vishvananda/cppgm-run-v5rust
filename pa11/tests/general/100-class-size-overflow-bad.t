// N3485 1.4p2, Annex B: a class of 2^63 bytes (each level doubling C0's 2^62)
// has a size no x86-64 implementation can represent; GCC and Clang reject it
// rather than wrap its size
struct C0 { long v[1L << 59]; };
struct C1 { C0 a; C0 b; };
struct C2 { C1 a; C1 b; };
int main() { return sizeof(C2) == 0; }
