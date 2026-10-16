// expect reject
// A dependent expression placeholder does not itself imply a parameter pack.
template<class T> void select(decltype(T())... values);
int main() { return 0; }
