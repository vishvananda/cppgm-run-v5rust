// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
struct X { static int n; };
long X::n;
int main() {}
