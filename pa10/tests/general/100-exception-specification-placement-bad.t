// N3485 15.4p2 (example): an exception-specification shall not appear in a
// typedef declaration (nor on a pointer-to-pointer, 15.4p2; incompatible
// redeclarations 15.4p3-p4); must be rejected
typedef void (*P)() noexcept;
int main() { return 0; }
