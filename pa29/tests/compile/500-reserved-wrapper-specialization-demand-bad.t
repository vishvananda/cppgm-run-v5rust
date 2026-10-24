// Demanding an explicit specialization reaches its own unsupported wrapper body.
template<class T> inline int specialized(T);
template<> inline int specialized<int>(int) { return __builtin_unprovided_specialization(); }
int demand() { return specialized(1); }
