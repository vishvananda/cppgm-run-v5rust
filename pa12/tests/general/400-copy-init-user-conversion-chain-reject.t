// A scalar conversion followed by a converting constructor is two user conversions.
struct A { A(int) {} };
struct X { operator int() const { return 7; } };
int main() { X source; A value = source; return 0; }
