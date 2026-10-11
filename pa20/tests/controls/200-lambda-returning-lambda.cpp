// N3485 5.1.2p4, 12.8: a lambda returning a closure object by value (copied
// into the deduced return type); expect exit 0
int main() { return [] { return [] { return 5; }; }()() - 5; }
