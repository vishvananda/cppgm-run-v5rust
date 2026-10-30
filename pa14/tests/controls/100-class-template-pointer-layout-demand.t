// Pointer scaling needs a specialization's layout without a declared object.
template<class T> struct Cell { T first; T second; };
long delta(Cell<long>* a, const Cell<long>* b) { return b-a; }
Cell<char>* advance(Cell<char>* p) { return p+1; }
Cell<int>* next(Cell<int>*& p) { return p++; }
int main() { return 0; }
