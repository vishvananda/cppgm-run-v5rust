// VALIDATION: compile-pass
template<class T, T... I> struct sequence {};
template<unsigned long... I> using indices = sequence<unsigned long, I...>;
template<unsigned long I> struct leaf {};
template<class, class...> struct impl;
template<unsigned long... I, class... T>
struct impl<indices<I...>, T...> : leaf<I>... {};

impl<indices<0, 1>, int, char> value;
leaf<0>& first = value;
leaf<1>& second = value;
int main() { return 0; }
