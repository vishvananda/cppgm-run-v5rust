// Base-object constructors and destructors take only `this` and the VTT and
// find their virtual bases through the vtable (Itanium 2.6), and a by-value
// class with virtual bases crosses the boundary with no hidden addresses:
// a g++ class derives from bases whose entries are built here, and by-value
// arguments go both ways.
extern int trace[16];
extern int trace_count;
struct Shared { int shared; Shared(); ~Shared(); };
struct Left : virtual Shared { int left; Left(); ~Left(); };
struct Right : virtual Shared { int right; Right(); ~Right(); };
struct Value : virtual Shared { int value; Value(); Value(const Value& other); };

int host_build();
int our_take(Value value);
int host_take(Value value);
int host_drive_value();
