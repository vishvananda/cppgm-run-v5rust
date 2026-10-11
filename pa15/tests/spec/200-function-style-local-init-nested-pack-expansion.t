// N3485 focus: 14.5.3 [temp.variadic] nested pack expansion in an initializer
int g(int);
template<class T> T id(T);
template<class... A> int f(A... a) { typedef int T; T x(g(id<A>(a)...)); return x; }
int main() { return f(1); }
