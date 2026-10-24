// An unsupported wrapper's emission is rejected at its body, after demand.
inline int wrapper() { return __builtin_unprovided_demand(); }
int demand() { return wrapper(); }
