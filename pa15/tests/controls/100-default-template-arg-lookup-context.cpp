// N3485 14.6.3p1, 14.1p10: a non-dependent name in a default template argument
// is bound where the default appears, not where the template is used;
// expect exit 0
char h(long);
template<class T, int N = sizeof(h(0))> struct C { static const int v = N; };
long h(int);
int main() { return C<int>::v == 1 ? 0 : 1; }
