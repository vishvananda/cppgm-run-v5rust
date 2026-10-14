// N3485 14.3.2p1, 8.5.3: a reference non-type parameter is an lvalue naming
// its object; binding a local reference to it must work; expect exit 0
int x = 3;
template<int& R> struct B { static int& ref() { int& r = R; return r; } };
int main() { B<x>::ref() = 11; return x == 11 ? 0 : 1; }
