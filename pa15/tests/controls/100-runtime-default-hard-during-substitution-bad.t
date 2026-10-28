// expect reject
// A selected function's default is a required instantiation even when the call
// is formed inside another candidate's substitution.
template<class T> int value(int n = T::missing);
template<class T> auto choose(T) -> decltype(value<T>());
int choose(...) { return 0; }
int main() { return choose(0); }
