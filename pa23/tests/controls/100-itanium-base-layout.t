// Itanium C++ ABI 2.4: the tail padding of a non-POD base is reused and a
// primary base is placed first (full program: exit 8)
// expect exit 8
struct A { int x; };
struct P { P(); int x; char c; }; P::P() {}
struct D4 : P { char d; };
struct Q { int x; char c; };
struct D5 : Q { char d; };
struct V { virtual void f(); char c; }; void V::f() {}
struct D6 : V { char d; };
struct M { private: int x; public: char c; };
struct D7 : M { char d; };
struct F { F() = default; int x; char c; };
struct D8 : F { char d; };
struct D9 : A, V { };
int main() { return sizeof(D4) * 1 + sizeof(D5) * 0 + (sizeof(D5) == 12 ? 0 : 50) + (sizeof(D6) == 16 ? 0 : 60) + (sizeof(D7) == 8 ? 0 : 70) + (sizeof(D8) == 12 ? 0 : 80) + (sizeof(D9) == 16 ? 0 : 90); } // 8
