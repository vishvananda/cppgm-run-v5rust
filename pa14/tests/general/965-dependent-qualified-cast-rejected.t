struct A { using X = int; };
template<class T> int f() { return T::X(2); }
int main() { return f<A>(); }
