// N3485 focus: 14.2 [temp.names] simple-template-id in postfix expressions
int f(int x) { return x; }
template<class T> int g(T x) { return f(x); }
int h() { return g<int>(1) < 2; }
