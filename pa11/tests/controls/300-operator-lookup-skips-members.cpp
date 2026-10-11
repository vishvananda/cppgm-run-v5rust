// N3485 13.3.1.2p3: non-member operator candidates come from unqualified
// lookup with member functions ignored; a class member operator+ must not hide
// M::operator+; expect exit 0
namespace Q { enum E { a = 1 }; }
namespace M {
  int operator+(Q::E, Q::E) { return 42; }
  struct D { int operator+(int) { return 0; } int g() { Q::E x = Q::a; return x + x; } };
}
int main() { M::D d; return d.g() == 42 ? 0 : 1; }
