struct tag {};
template<class T> struct result { result(tag, T&&) {} };
struct item { item(int) {} };
template<class T, int N> struct array { T elements[N]; };
template<class T> result<T> make() { return {tag(), T{item(0), item(1)}}; }
int main() { make<array<item, 2> >(); }
