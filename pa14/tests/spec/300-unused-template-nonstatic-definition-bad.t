// VALIDATION: compile-fail
// An unused definition still needs a matching static data member declaration.
template<class T> struct X { int n; };
template<class T> int X<T>::n;
int main() {}
