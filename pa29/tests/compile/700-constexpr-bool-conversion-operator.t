template<bool B> struct X {};

struct C {
  constexpr operator bool() const { return true; }
};

constexpr bool b = C{};
static_assert(C{}, "");
X<C{}> x;

int main() {
  static_assert(b, "");
  return 0;
}
