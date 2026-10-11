// VALIDATION: compile-pass
template<class...> struct list {};
template<class, class> struct append;
template<template<class...> class L, class... T, class U>
struct append<L<T...>, U> { typedef L<T..., U> type; };
template<class L, class T> using append_t = typename append<L, T>::type;

typedef append_t<append_t<append_t<append_t<append_t<append_t<append_t<
        append_t<append_t<append_t<list<>, int>, int>, int>, int>, int>,
        int>, int>, int>, int>, int> actual;
template<class> struct check;
template<> struct check<list<int, int, int, int, int,
                             int, int, int, int, int> > { typedef int type; };
typedef check<actual>::type verified;

int main() { return 0; }
