// N3485 14.7.2p1, p3: `template struct O<double>::I;` explicitly instantiates
// a member class of a class template specialization; expect exit 0
template<class T> struct O { struct I { int f() { return sizeof(T); } }; };
template struct O<double>::I;
int main() { O<double>::I i; return i.f() == 8 ? 0 : 1; }
