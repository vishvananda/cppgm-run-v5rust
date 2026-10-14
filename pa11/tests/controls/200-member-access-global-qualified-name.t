// N3485 5.2.5p1 (id-expression may be a qualified-id), 3.4.5p4: s.::B::s names
// B's member through a nested-name-specifier starting with ::; expect exit 0
struct B { int x; static int s; };
int B::s = 4;
struct S : B { };
int main() { S s; s.x = 3; return s.::B::s + s.::B::x == 7 ? 0 : 1; }
