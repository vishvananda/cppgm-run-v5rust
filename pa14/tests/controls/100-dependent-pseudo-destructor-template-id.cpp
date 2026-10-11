// N3485 3.4.5p3, 12.4p13: p->~P<T>() through a pointer of dependent type names
// the destructor without the template keyword; expect exit 0
int count = 0;
template <class T> struct P { ~P() { ++count; } };
template <class T> struct H { void destroy_through(P<T>* p) { p->~P<T>(); } };
int main() { { P<int> v; H<int> h; h.destroy_through(&v); } return count == 2 ? 0 : 1; }
