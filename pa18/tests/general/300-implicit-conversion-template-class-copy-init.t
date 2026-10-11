// A conversion function template produces the destination class directly.
struct A { int value; A(int n) : value(n) {} };
struct X { template<class T> operator T() const { return T(7); } };
int main() { X source; A result = source; return result.value == 7 ? 0 : 1; }
