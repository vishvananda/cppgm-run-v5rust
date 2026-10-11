// VALIDATION: compile-pass
// Hosted intrinsic compatibility: __make_integer_seq count evaluation accepts
// the compiler's synthetic C-style cast spelling for unsigned non-type values.

template<class T, T... I>
struct integer_sequence
{
  static const unsigned size = sizeof...(I);
};

typedef __make_integer_seq<integer_sequence, unsigned, (unsigned int)3ULL> seq;

int main()
{
  return seq::size == 3 ? 0 : 1;
}
