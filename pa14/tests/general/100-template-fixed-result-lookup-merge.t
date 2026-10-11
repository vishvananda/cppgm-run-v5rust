// Different nondependent expressions can name the same result type.
long selected(double);
template<class T> auto result(T) -> decltype(selected(0));
long selected(int);
template<class U> auto result(U) -> decltype(selected(0)) { return 7; }
long first();
long second();
template<class T> auto other(T) -> decltype(first());
template<class U> auto other(U) -> decltype(second()) { return 9; }
int main() { return result(0) == 7 && other(0) == 9 ? 0 : 1; }
