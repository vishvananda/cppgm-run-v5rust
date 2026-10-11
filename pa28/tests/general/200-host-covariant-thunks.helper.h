// A covariant overrider whose result needs adjusting gets its own primary
// entry, and its base's entry a covariant thunk (Itanium 5.1.4).  The thunk
// adjusts `this` from the base the entry belongs to, as g++ picks it (the
// nearest base below the overrider's class whose own entry is no thunk):
// through the vcall offset when that is a virtual base (Direct, Primary),
// by a fixed offset otherwise (Below).  A result moves to the virtual base
// its overridden result lies in, then by the fixed offset inside it (Inner).
struct Root { virtual Root* self(); };
struct Direct : virtual Root { Direct* self(); int d; };
struct Holder : virtual Root { int h; };
struct Below : Holder { Below* self(); int b; };
struct Primary : virtual Holder { Primary* self(); int p; };
struct Near { virtual Near* self(); };
struct Mid : Near { };
struct Inner : virtual Mid { Inner* self(); int i; };

int host_root(Root& value, Root* expected);
int host_near(Near& value, Near* expected);
