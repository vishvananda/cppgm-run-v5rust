// N3485 3.4.1p13, 3.4.3p3: names after the declarator-id X::v (initializer,
// array bound, parameters, conversion-type-id) are looked up in X first: (T)+1
// is a cast to X::T; expect exit 0
struct X { typedef int T; static int v; };
int T = 10;
int X::v = (T)+1;
int main() { return X::v == 1 ? 0 : 1; }
