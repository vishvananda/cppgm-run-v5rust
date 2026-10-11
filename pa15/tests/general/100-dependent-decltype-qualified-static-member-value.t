template<class T>
struct wrap {
  static const bool value = decltype(T())::value;
};

struct S {
  static const bool value = true;
};

static_assert(wrap<S>::value, "");

int main() {
  return wrap<S>::value ? 0 : 1;
}
