// VALIDATION: compile-pass
// Full collection must not materialize a later member during an earlier initializer.

namespace tag { struct lower {}; }
template<class, class> struct has { static bool const value = false; };
template<bool> struct choose { typedef int type; };

template<class M> struct make {
  static bool const lower = has<M, tag::lower>::value;
  typedef typename choose<lower>::type encoding;
  typedef int tag;
};

typedef make<int>::encoding encoding;
static_assert(sizeof(make<int>) != 0, "");

int main() {}
