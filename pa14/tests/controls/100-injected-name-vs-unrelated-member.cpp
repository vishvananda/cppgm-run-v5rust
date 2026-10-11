// N3485 9p2/3.4.1: inside class template set, set<K> names the class
// (injected-class-name); a member function bits::set of an unrelated class is
// not visible (libstdc++ <bitset> then <set>); expect exit 0
struct bits { int set() { return 1; } };
template<class K> struct set { int f(const set<K>&) const { return 0; } };
int main() { set<int> a; return a.f(a); }
