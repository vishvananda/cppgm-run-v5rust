// N3485 7.3.1.2p3: an unqualified friend class declaration in a::b::A names
// a::b::X (innermost namespace), not the outer a::X; expect exit 0
namespace a { class X { public: int f(); };
  namespace b { class A { friend class X; int p = 1; }; class X { public: int g(A x) { return x.p; } }; }
}
int main() { return a::b::X().g(a::b::A()) == 1 ? 0 : 1; }
