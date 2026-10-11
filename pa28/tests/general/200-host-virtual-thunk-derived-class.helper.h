// An overrider reached through a virtual base's table is a virtual thunk
// (Itanium 5.1.4) that reads its vcall offset, so a class derived by g++,
// which places the virtual base elsewhere, can name and use it.
struct Root { int root; virtual int value(); virtual ~Root() {} Root(); };
struct Side { int side; virtual int other(); Side(); };
struct Mid : virtual Root, virtual Side { int mid; int value(); Mid(); };
int host_value(Root& root);
int host_check();
