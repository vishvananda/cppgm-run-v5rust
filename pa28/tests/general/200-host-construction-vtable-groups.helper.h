// A base with virtual bases is constructed with a sub-VTT naming the
// construction vtable group laid out for it in the complete object (Itanium
// 2.6.2-2.6.4): its tables with the complete object's offsets, and its own
// table for a virtual primary base that lies elsewhere there (A, which D's X
// claims, so Z's X and Y lose it).  Every secondary virtual pointer of the
// sub-VTT, a lost primary base's included, is stored where the complete
// object placed its subobject, so calls made during construction run the
// overriders of the class being built.  D's VTT and construction tables are
// built here with host-built base constructors; HD's the other way round.
int note_value(int value);

struct A { virtual int f(); virtual int g(); };
struct X : virtual A { int x; X(); };
struct Y : virtual A { int y; Y(); virtual int h(); };
struct Z : virtual Y, X { int z; Z(); int g(); };
struct D : X, virtual Z { int d; D(); int f(); };

struct HA { virtual int f(); virtual int g(); };
struct HX : virtual HA { int x; HX(); };
struct HY : virtual HA { int y; HY(); virtual int h(); };
struct HZ : virtual HY, HX { int z; HZ(); int g(); };
struct HD : HX, virtual HZ { int d; HD(); int f(); };

D* host_check_d();
HD* host_make_hd();
