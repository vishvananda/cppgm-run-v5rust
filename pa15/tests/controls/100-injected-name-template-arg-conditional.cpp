// N3485 14.6.1p1, 14.2p1: inside class template NH the injected-class-name
// followed by < is the template; a conditional-expression template argument
// (NH<B, true ? B : P>) must parse (libstdc++ C++14 <bits/parse_numbers.h>
// _Number_help); expect exit 0
template<int B, int P> struct NH { static const int v = P; using next = NH<B, true ? B : P>; };
int main() { return NH<1, 2>::next::v == 1 ? 0 : 1; }
