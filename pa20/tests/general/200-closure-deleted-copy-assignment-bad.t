// N3485 5.1.2p19: the closure type has a deleted copy assignment operator (and
// deleted default constructor); expect reject
template <class F> int asg(F a) { F b(a); a = b; return a(); }
int main() { return asg([] { return 3; }) - 3; }
