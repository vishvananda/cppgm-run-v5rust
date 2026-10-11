template<bool B, class T, class F> struct conditional;
template<class T, class F> struct conditional<true, T, F> { typedef T type; };
template<class T, class F> struct conditional<false, T, F> { typedef F type; };
template<bool B, class T, class F> using conditional_t = typename conditional<B, T, F>::type;

template<class T> struct imp {};

template<class T>
struct holder : conditional_t<true, imp<T>, void> {};

typedef holder<int> X;

int main() { return 0; }
