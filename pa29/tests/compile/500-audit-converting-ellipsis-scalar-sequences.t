// Implicit C(...) conversion retains its selected first scalar sequence.
// Written variadic calls still apply their ordinary default promotions.
struct converted { converted(...); };
int consume(converted);
int from_float(float value) { return consume(value); }
int from_short(short value) { return consume(value); }
int from_bool(bool value) { return consume(value); }
int from_nullptr(decltype(nullptr) value) { return consume(value); }
int written_float(float value) { return consume(converted(value)); }
int written_short(short value) { return consume(converted(value)); }
