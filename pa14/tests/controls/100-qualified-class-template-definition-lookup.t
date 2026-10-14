// N3485 3.4.1/7 (9/11): names in the body of template<class T> struct N::X
// {...} are looked up in the scope of N, so Z finds N::Z; expect exit 0
namespace N { template<class T> struct X; typedef int Z; }
template<class T> struct N::X { Z a; T b; };
int main() { N::X<int> x; x.a = 0; return x.a; }
