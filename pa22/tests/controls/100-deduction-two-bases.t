// N3485 14.8.2.1p5: two possible deduced A's (B<int> and B<char> bases) make
// deduction fail, so f(...) is chosen; expect exit 0
template<class T> struct B {};
struct D : B<int>, B<char> {};
template<class T> int f(B<T>&) { return 1; }
int f(...) { return 0; }
int main() { D d; return f(d); }
