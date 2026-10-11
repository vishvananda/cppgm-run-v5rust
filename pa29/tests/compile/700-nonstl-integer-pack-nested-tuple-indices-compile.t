// A dependent alias-template body whose qualifier is a class template-id whose
// first argument is a concrete type name (size_t) and whose second argument is
// a structured __integer_pack expansion. Substituting the scalar bounds must
// preserve that outer expansion for builtin lowering, while the concrete type
// argument continues through ordinary name lookup as unsigned long.
// This mirrors libc++'s __make_indices_imp / __integer_sequence machinery used
// by std::map::operator[] (piecewise tuple construction).
typedef unsigned long size_t;
template <size_t...> struct tuple_indices {};
template <class IdxType, IdxType... Values>
struct integer_sequence {
  template <size_t Sp>
  using to_tuple_indices = tuple_indices<(Values + Sp)...>;
};
template <size_t Ep, size_t Sp>
using make_indices_imp =
    typename integer_sequence<size_t, __integer_pack(Ep - Sp)...>::template to_tuple_indices<Sp>;
typedef make_indices_imp<1, 0> indices;
int main() { return 0; }
