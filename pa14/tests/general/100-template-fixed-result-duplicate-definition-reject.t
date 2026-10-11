// Equal nondependent return types do not permit a second definition.
long first();
long second();
template<class T> auto result(T) -> decltype(first()) { return 7; }
template<class U> auto result(U) -> decltype(second()) { return 9; }
int main() { return 0; }
