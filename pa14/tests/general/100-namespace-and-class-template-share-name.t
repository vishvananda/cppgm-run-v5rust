namespace lexer { template<class, class> struct lexer {}; }
template<class A, class B> void f(lexer::lexer<A, B>&) {}
int main() { lexer::lexer<int, int> x; f(x); }
