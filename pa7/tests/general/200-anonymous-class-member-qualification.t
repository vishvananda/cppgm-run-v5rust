// Anonymous class members use the class's stable local label in semantic views.
struct { int read() { return 7; } } value;
int result() { return value.read(); }
