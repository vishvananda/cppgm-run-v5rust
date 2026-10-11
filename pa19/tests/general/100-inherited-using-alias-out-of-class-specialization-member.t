// Reduced from libc++ char_traits<char16_t>. A full specialization can inherit
// a member alias from a class-template base, then use that alias in an
// out-of-class static member definition.
typedef unsigned long size_t;
typedef unsigned short uint_least16_t;

namespace lib {

template<class CharT, class IntT, IntT EofValue>
struct traits_base {
  using char_type = CharT;
  using int_type = IntT;

  static bool lt(char_type lhs, char_type rhs) {
    return lhs < rhs;
  }
};

template<class CharT>
struct traits;

template<>
struct traits<char16_t> : traits_base<char16_t, uint_least16_t, static_cast<uint_least16_t>(0xffff)> {
  static int compare(const char_type* lhs, const char_type* rhs, size_t n);
};

inline int traits<char16_t>::compare(const char_type* lhs,
                                     const char_type* rhs,
                                     size_t n) {
  for (; n; --n, ++lhs, ++rhs) {
    if (lt(*lhs, *rhs)) {
      return -1;
    }
    if (lt(*rhs, *lhs)) {
      return 1;
    }
  }
  return 0;
}

}

int main() {
  char16_t lhs[1] = {0};
  char16_t rhs[1] = {0};
  return lib::traits<char16_t>::compare(lhs, rhs, 1);
}
