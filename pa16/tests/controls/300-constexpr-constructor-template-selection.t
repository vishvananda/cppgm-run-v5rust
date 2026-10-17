// N3485 7.1.5, 12.1 and 14.8.2.1: empty and single-argument constructor deduction.
// expect exit 0
struct Base { int n; constexpr Base(int n) : n(n) {} };
template<class... T> struct Derived : T... {
    template<class... U> constexpr Derived(U... value) : T(value)... {}
};
struct Default {
    int n;
    template<class T = int> constexpr Default(T n = sizeof(T)) : n(n) {}
};
constexpr Derived<> empty;
constexpr Derived<Base> one(Base(3));
constexpr Default known;
static_assert(one.n == 3 && known.n == sizeof(int), "selected constructor and default context");
int main() { Default runtime; return runtime.n == sizeof(int) ? 0 : 1; }
