// N3485 12.9, 14.7.1 and 15.4: preserve the frozen inherited-template demand order.
// expect exit 0
int extra() { return 5; }
struct Base {
    int n;
    template<class T> Base(T n, int m = extra()) noexcept : n(n + m) {}
};
struct Derived : Base { using Base::Base; };
struct Further : Derived { using Derived::Derived; };
// The frozen compiler computes the forwarding exception plan before typing defaults.
static_assert(noexcept(Derived(3)), "exception plan before definition demand");
static_assert(noexcept(Further(7)), "forwarding chain");
struct UnusedDefault {
    template<class T> UnusedDefault(T, int = T::missing) noexcept {}
};
struct InheritedDefault : UnusedDefault { using UnusedDefault::UnusedDefault; };
static_assert(noexcept(InheritedDefault(3)), "unevaluated default stays undemanded");
static_assert(noexcept(InheritedDefault(3, 4)), "unevaluated body stays undemanded");
int main() {
    Derived d(3);
    Further f(7);
    InheritedDefault complete(3, 4);
    return d.n == 8 && f.n == 12 ? 0 : 1;
}
