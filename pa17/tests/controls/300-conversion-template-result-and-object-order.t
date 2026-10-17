// N3485 14.5.2p5-p8, 14.8.2.3: named result deduction precedes object
// ranking; hidden base templates and unselected bodies stay unused; expect exit 0
int selected;
struct A {
    template<class T> operator T*() { selected = 1; return 0; }
    template<class T> operator T() { return T::unused; }
};
struct Base {
    template<class T> operator T*() { return T::hidden; }
};
struct Derived : Base {
    template<class U> operator U*() { selected = 2; return 0; }
};
struct Qualified {
    template<class T> operator T*() const { selected = 3; return 0; }
    template<class T> operator T() { selected = 4; return T(); }
};
struct Ordinary {
    operator int*() { selected = 5; return 0; }
    template<class T, class U = typename T::unused> operator T*();
};
template<class Owner> struct Nested {
    template<class T, class... Unused> operator T*() { selected = 6; return 0; }
};
int main() {
    A a;
    Derived d;
    Qualified q;
    const Qualified cq{};
    Ordinary o;
    Nested<long> n;
    typedef int* Pointer;
    if (a.operator Pointer() != 0 || selected != 1) return 1;
    if (d.operator Pointer() != 0 || selected != 2) return 2;
    if (q.operator Pointer() != 0 || selected != 4) return 3;
    if (cq.operator Pointer() != 0 || selected != 3) return 4;
    if (o.operator Pointer() != 0 || selected != 5) return 5;
    if (n.operator Pointer() != 0 || selected != 6) return 6;
    return 0;
}
