// N3485 focus: 14.3.2 [temp.arg.nontype] converted constant expression.
// A bound integral non-type argument may be reused as a wider integral
// non-type argument when its constant value is representable in the target.
struct C {};

template <unsigned long long A, unsigned long long CValue>
struct pick {
  static const unsigned long long value = A + CValue;
};

template <class UIntType, UIntType A, UIntType C>
struct engine {
  static const unsigned long long value = pick<A, C>::value;
};

static_assert(engine<unsigned, 48271u, 0u>::value == 48271ull, "converted value");

int main()
{
  return 0;
}
