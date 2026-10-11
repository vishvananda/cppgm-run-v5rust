// N3485 7p3: struct { int a; }; at namespace scope declares nothing and is
// ill-formed; must be rejected
struct { int a; };
int main() { return 0; }
