// N3485 11p6-p7 (standard example), 11.3p2, 11.7: names in a friend's
// parameter/return types, a friend class's base-specifiers and a nested class
// member's definition are access-checked in that entity's context;
// expect exit 0
class A { typedef int I; class B { public: typedef int J; static J s; int b; }; friend I g(I); friend class X; };
A::I g(A::I p) { return p + 1; }   // course: inaccessible qualified type: I
A::B::J A::B::s = 1;               // course: inaccessible qualified type: B
struct X : A::B { };               // course: inaccessible qualified type: B
int main() { X x; x.b = 0; return g(1) + x.b == 2 ? 0 : 1; }
