// N3485 14.7.3p2: an explicit specialization shall be declared in a namespace
// enclosing the specialized template; expect reject
namespace N { template<class T> struct S {}; }
namespace M { template<> struct N::S<int> {}; }
int main() { return 0; }
