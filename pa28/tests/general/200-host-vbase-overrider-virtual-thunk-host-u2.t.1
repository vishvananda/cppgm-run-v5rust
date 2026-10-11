// Itanium ABI 2.5.3/5.1.4.6: an overrider (Derived::get) of a virtual base's
// function is reached through a virtual thunk _ZTv0_n32_ (vcall offset), which
// the defining TU must emit for other TUs' construction vtables;
// expect link ok and exit 0 under every mix of units built by cppgm++ and by
// the host compiler
struct Base { virtual ~Base(); virtual int get() const; int b; };
struct Derived : virtual Base { Derived(); virtual int get() const; int d; };
struct Leaf : Derived { virtual int get() const { return Derived::get() + 100; } };
int main() { Leaf leaf; Base& lb = leaf; return lb.get() == 147 ? 0 : 1; }
