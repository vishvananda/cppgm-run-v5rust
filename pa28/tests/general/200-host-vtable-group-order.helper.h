// A vtable group lays out the primary table, the non-virtual bases'
// secondary tables, then the virtual bases' tables in inheritance graph
// order (Itanium 2.5.2): here B1's before B2's, though the primary base P
// brings B1.  The host builds T's constructor inline against the vtable
// group built here, storing each table's address point as an offset into it.
struct B1 { virtual int f(); int m1; };
struct B2 { virtual int g(); int m2; };
struct P : virtual B1 { virtual int p(); int mp; };
struct T : virtual B1, virtual B2, P { virtual int t(); int f(); int g(); int mt; };

T* host_make();
int host_f(B1& value);
int host_g(B2& value);
