// N3485 5.1.2p12, 3.2p2: `0 && x` in a captureless lambda odr-uses the
// automatic x (the operand is potentially evaluated though never evaluated):
// ill-formed; expect reject
int main() { int x = 5; return [] { return 0 && x; }(); }
