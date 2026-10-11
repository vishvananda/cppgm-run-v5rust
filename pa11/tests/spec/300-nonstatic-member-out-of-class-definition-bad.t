// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
struct X { int n; };
int X::n;
int main() {}
