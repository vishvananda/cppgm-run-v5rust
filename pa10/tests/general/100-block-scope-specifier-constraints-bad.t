// N3485 7.1.1p10: mutable applies only to non-static, non-const, non-reference
// class data members; a block-scope `mutable int x` must be rejected (also:
// block-scope extern with initializer 8.5p5, block-scope inline 7.1.2p1,
// mutable/extern in a for-range-declaration 6.5.4p2, mutable const member in
// an instantiation). Must reject.
int main() { mutable int x = 1; return x - 1; }
