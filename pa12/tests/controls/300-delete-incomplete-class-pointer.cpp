// N3485 5.3.5p5: delete of a pointer to an incomplete class type is
// well-formed (UB only if the complete class has a non-trivial destructor or
// deallocation function); expect exit 0
struct I;
void f(I* p) { delete p; }
int main() { f(0); return 0; }
