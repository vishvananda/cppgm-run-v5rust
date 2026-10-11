// N3485 14.6p10 (diagnosable, no NDR note): a non-dependent name in a template
// definition must be declared at its point of use; undeclared_fn(1) in an
// uninstantiated template is ill-formed; must be rejected
template<class T> int f() { return undeclared_fn(1); }
int main() { return 0; }
