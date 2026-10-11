// A virtual destructor's deleting entry (D0) and its thunks are defined
// with the destructor (Itanium 5.1.4, as g++ does), whichever unit has the
// key function and emits the vtable; the vtable's unit only names them.
// Derived's and VDerived's vtables are built by the host, their destructors
// here; HDerived's and HVDerived's the other way round.  Deleting through a
// secondary base runs a non-virtual thunk, through a virtual base a virtual
// thunk.
int note_destroyed(int value);

struct Base { virtual int key(); virtual ~Base(); int b; };
struct Second { virtual ~Second(); int s; };
struct Derived : Base, Second { int key(); ~Derived(); int d; };
struct VBase { virtual ~VBase(); int v; };
struct VDerived : virtual VBase { virtual int key(); ~VDerived(); int w; };

struct HBase { virtual int key(); virtual ~HBase(); int b; };
struct HSecond { virtual ~HSecond(); int s; };
struct HDerived : HBase, HSecond { int key(); ~HDerived(); int d; };
struct HVBase { virtual ~HVBase(); int v; };
struct HVDerived : virtual HVBase { virtual int key(); ~HVDerived(); int w; };

Derived* host_make_derived();
VDerived* host_make_vderived();
void host_delete_second(HSecond* value);
void host_delete_vbase(HVBase* value);
