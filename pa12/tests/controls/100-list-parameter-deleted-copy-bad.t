// expect reject
// A list containing a same-class lvalue must still select its copy constructor.
struct Value { Value(int); Value(const Value&) = delete; };
void take(Value);
void f(Value& value) { take({value}); }
