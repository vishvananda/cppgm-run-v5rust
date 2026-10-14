// N3485 14.7.3/5,14.7/4 + 5.4: C<int> names the explicit specialization, whose
// m is a static data member, so (C<int>::m) * q is a multiplication, not a
// cast; expect exit 0
template<class T> struct C { typedef int m; };
template<> struct C<int> { static const int m = 3; };
int main() { int q = 2; return (C<int>::m) * q == 6 ? 0 : 1; }
