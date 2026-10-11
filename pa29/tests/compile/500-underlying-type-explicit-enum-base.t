template<class A, class B>
struct same {
  static const bool value = false;
};

template<class A>
struct same<A, A> {
  static const bool value = true;
};

enum class ScopedChar : unsigned char {
  A
};

enum class ScopedShort : unsigned short {
  A
};

enum PlainChar : unsigned char {
  PlainA
};

typedef __underlying_type(ScopedChar) ScopedCharBase;
typedef __underlying_type(ScopedShort) ScopedShortBase;
typedef __underlying_type(PlainChar) PlainCharBase;

static_assert(same<ScopedCharBase, unsigned char>::value,
              "scoped enum explicit char base");
static_assert(same<ScopedShortBase, unsigned short>::value,
              "scoped enum explicit short base");
static_assert(same<PlainCharBase, unsigned char>::value,
              "unscoped enum explicit char base");

int main() {
  return 0;
}
