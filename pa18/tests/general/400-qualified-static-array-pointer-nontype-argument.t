// VALIDATION: compile-pass
template<char const *P> struct tag { static char get() { return *P; } };
template<class> struct holder { static char const value[1]; };
template<class T> char const holder<T>::value[1] = {};
int main() { return tag<holder<int>::value>::get(); }
