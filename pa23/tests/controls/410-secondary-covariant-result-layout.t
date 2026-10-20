// The secondary view's covariant thunk needs R's layout although neither
// the body nor its null result needs an R object; expect exit 0
struct P { int p; };
struct Q { int q; };
struct R : P, Q {};
struct First { virtual int first() { return 1; } };
struct Base { virtual Q* get() { return 0; } };
struct Derived : First, Base { R* get() override { return 0; } };
int main() {
    Derived d;
    Base* b = &d;
    return b->get() != 0;
}
