// N3485 12.1p5: S's defaulted default constructor is deleted because S is
// union-like with a variant member (anonymous union member N) that has a
// non-trivial default constructor; `S s;` must be rejected
struct N { N() {} int x; };
struct S { union { N n; int i; }; };
int main() { S s; (void)s; return 0; }
