// N3485 focus: 7.3.3 [namespace.udecl] using-declaration
namespace N { typedef int Y; }
using N::Y;
Y x;
typedef const Y CY;
namespace X {
  using ::CY;
  using N::Y;
}
X::CY* ci;
X::Y i;
