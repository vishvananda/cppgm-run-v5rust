// N3485 3.6.2p2 (first bullet): a namespace-scope reference bound to an lvalue
// of static storage duration is constant-initialized before any dynamic
// initialization (early reads it); expect exit 0
int value;
extern int& r;
int* early = &r;
int& r = value;
int main() { return early == &value ? 0 : 1; }
