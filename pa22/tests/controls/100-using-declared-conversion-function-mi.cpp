// N3485 7.3.3p15, 13.3.1.5p1, 10.2: using A::operator int in D : A, B makes
// A::operator int a member of D that hides B::operator int for the conversion;
// expect exit 0
struct A { operator int() const { return 1; } };
struct B { operator int() const { return 2; } };
struct D : A, B { using A::operator int; };
int main() { D d; int x = d; return x == 1 ? 0 : 1; }
