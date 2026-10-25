namespace member_equality {
struct Prefix { long pad; };
struct Base { int data; int f(); int g(); virtual int v(); };
struct Derived : Prefix, Base {};
typedef int (Base::*BasePointer)();
typedef int (Derived::*Pointer)();
Pointer host_pointer(int which);
bool candidate_equal(Pointer left, Pointer right);
bool candidate_unequal(Pointer left, Pointer right);
bool host_check();
}
