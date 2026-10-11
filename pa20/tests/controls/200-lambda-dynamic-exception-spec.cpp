// N3485 5.1.2p1 (lambda-declarator: ( parameter-declaration-clause )
// mutable(opt) exception-specification(opt)...): a lambda with throw(int) must
// parse; expect exit 0
int main() { auto a = [](int x) throw(int) { return x; }; return a(0); }
