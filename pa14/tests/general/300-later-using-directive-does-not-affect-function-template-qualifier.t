// A using-directive declared after a function does not participate in lookup
// from that function body.
namespace A { namespace D {
template<class> struct X { enum { value = 0 }; };
} }
namespace B { namespace D {} }

int f() { using namespace A; return D::X<int>::value; }
using namespace B;
int main() { return f(); }
