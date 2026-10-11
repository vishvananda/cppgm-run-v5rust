// Parameter packs a signature expands: a template template parameter's
// argument list (T_IJDpT0_EE), a function type's trailing parameters
// (PFT_DpT0_E, MT0_KFT_DpT1_E) and an alias over a typedef's value pack
// (8sequenceImJXspT_EE).
namespace n { typedef unsigned long size_type; }

template<class... T> struct tuple {};
template<class T, T... I> struct sequence {};
template<n::size_type... I> using indices = sequence<n::size_type, I...>;
struct X { int m(int, char) const; };
