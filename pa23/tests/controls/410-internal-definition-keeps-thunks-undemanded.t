// Internal definitions emit thunks only when their tables demand them.
// The unused result layout must stay undemanded too; expect exit 0
namespace {
struct P { int p; };
struct Q { int q; };
struct R : P, Q { char unused[1152921504606846976]; };
struct First { virtual int first(); };
struct Base { virtual Q* get(); };
struct Derived : First, Base {
    virtual int key();
    R* get();
};
R* Derived::get() { return 0; }
}
int main() { return 0; }
