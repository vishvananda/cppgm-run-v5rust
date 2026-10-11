// A thunk is defined with the function it adjusts for (Itanium 5.1.4):
// every thunk its class's tables have, shared tables included, so that any
// other unit's vtable or construction vtable can name it; a unit whose
// vtable names a thunk to a function defined elsewhere only refers to it.
// An entry reached through a virtual base from a secondary table inside it
// is a virtual thunk with a fixed part (TD::y from TY in TV).

// TD's vtable is built where its key function TD::k is (the host); TD::y
// and so its virtual thunk are built here.
struct TX { virtual int x(); int a; };
struct TY { virtual int y(); int b; };
struct TV : TX, TY { int c; };
struct TD : virtual TV { virtual int k(); int y(); int d; };

// C1 shares its table with its virtual primary base C0; in the host's
// CE the other C1 loses C0, and CE's construction table for it calls C1::f0
// through the virtual thunk the unit defining C1::f0 provides.
struct C0 { virtual int f0(); };
struct C1 : virtual C0 { C1(); int f0(); virtual int f1(); int m1; };
struct C3 : virtual C0, virtual C1 { virtual int f3(); int m3; };
struct CE : C3, C1 { CE(); int f0(); virtual int f4(); int m6; };

// S3's functions and thunks are the host's; SE, built here, refers to
// S3::f2's thunk for its S0 table.
struct S0 { virtual int f2(); int m0; };
struct S2 { virtual int g(); int m2; };
struct S3 : S2, S0 { int f2(); int m3; };
struct SE : S3 { virtual int f4(); int m6; };

TD* host_make_td();
int host_td_y(TY& value);
CE* host_make_ce();
int host_c0_f0(C0& value);
int host_s0_f2(S0& value);
