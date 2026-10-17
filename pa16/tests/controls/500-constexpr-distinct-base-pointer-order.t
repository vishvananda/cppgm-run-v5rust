// N3485 5.9p2: distinct base subobjects have no specified pointer order.
// expect reject
struct First { int a; };
struct Second { int b; };
struct Object : First, Second {};
Object object;
static_assert(&object.a < &object.b, "unspecified comparison");
