// N3485 14.1p11: a template parameter pack of a primary class (or alias)
// template shall be the last template-parameter; expect reject
template<class... Ts, class U> struct S {};
int main() { return 0; }
