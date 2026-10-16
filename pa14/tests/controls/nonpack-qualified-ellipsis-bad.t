// expect reject
// Pack formation visits the dependent qualifier, whose T is not a pack.
template<class T> void select(typename T::type... values);
int main() { return 0; }
