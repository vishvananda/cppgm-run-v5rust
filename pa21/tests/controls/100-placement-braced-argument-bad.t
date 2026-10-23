// expect reject
// The frozen placement-new path types each placement argument as an expression.
struct Value { Value(int); Value(const Value&) = delete; };
void* operator new(unsigned long, Value);
void f() { new ({7}) int; }
