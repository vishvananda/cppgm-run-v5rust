// N3485 2.14.8p3: frozen literal selection consumes the indexed lookup order.
// expect exit 0
namespace A { template<char... C> int operator""_x() { return 1; } }
namespace B { template<char... C> int operator""_x() { return 2; } }
using A::operator""_x;
using B::operator""_x;
int before() { return 12_x; }
using A::operator""_x;
int after() { return 12_x; }
int main() { return before() != 2 || after() != 1; }
