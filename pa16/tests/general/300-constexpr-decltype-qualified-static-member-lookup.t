struct S {
  static const bool value = true;
};

constexpr int f() {
  return decltype(S())::value ? 1 : 0;
}

static_assert(f() == 1, "");

int main() {
  return f() == 1 ? 0 : 1;
}
