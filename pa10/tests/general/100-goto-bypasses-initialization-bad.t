// N3485 6.7p3: a jump past a declaration with initialization (int x = 1) into
// its scope is ill-formed; must be rejected
int f() { goto L; int x = 1; L: return x; }
int main() { return f(); }
