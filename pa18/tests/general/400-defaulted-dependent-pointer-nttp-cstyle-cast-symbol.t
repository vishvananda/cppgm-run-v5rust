// VALIDATION: compile-pass
template<class> struct V { typedef void t; };
template<class T, typename V<decltype((T *)0)>::t * = nullptr>
bool f(int) { return false; }
int main() { return f<int>(0); }
