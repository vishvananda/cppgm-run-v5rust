// N3485 5.16p5, 13.6p25, 5.1.2p6: two captureless closures in ?: convert to
// the common function pointer type int(*)(); expect exit 0
int main() { bool b = true; return (b ? [] { return 1; } : [] { return 2; })() - 1; }
