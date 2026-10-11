// N3485 5.1.2p14 (note): a reference to function captured by copy becomes a
// reference member of the closure; expect exit 0
int two() { return 2; }
int main() { int (&fr)() = two; auto f = [fr]() { return fr(); }; return f() == 2 ? 0 : 1; }
