// N3485 7.2p11, 5.1.1p8: an unscoped enumerator is declared in the scope that
// contains its enumeration, and a nested-name-specifier naming the
// enumeration may still qualify it.
enum E { X = 1, Y };
enum F : unsigned char { P = 3 };
namespace N { enum G { Q = 5 }; }
struct S { enum H { R = 7 }; };
static_assert(E::X == 1 && E::Y == X + 1, "enumeration-qualified");
static_assert(F::P == 3, "fixed underlying type");
static_assert(N::G::Q == N::Q, "namespace member");
static_assert(S::H::R == S::R, "class member");
