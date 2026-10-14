// N3485 14.7.3/5: a member class of an explicitly specialized class template
// is defined out of class as struct P<int>::Q {...} (no template<>);
// expect exit 0
template<class T> struct P { };
template<> struct P<int> { struct Q; };
struct P<int>::Q { int v; };
int main() { P<int>::Q q; q.v = 0; return q.v; }
