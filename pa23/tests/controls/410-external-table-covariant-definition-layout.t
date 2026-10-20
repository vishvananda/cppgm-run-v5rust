// A definition owns its thunk even when another unit owns its table;
// no R object or Derived table is needed by main; expect exit 0
struct P { int p; };
struct Q { int q; };
struct R : P, Q {};
struct First { virtual int first(); };
struct Base { virtual Q* get(); };
struct Derived : First, Base {
    virtual int key();
    R* get() override;
};
R* Derived::get() { return 0; }
int main() { return 0; }
