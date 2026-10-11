// N3485 7.1.6.4p4: auto may declare a static data member with an initializer,
// never a non-static data member; must be rejected
struct S { auto m = 1; };
int main() { S s; return s.m - 1; }
