template <typename T>
struct Holder {
  static const bool __is_signed = true;
  static const int value = __is_signed ? 1 : 0;
};

static_assert(Holder<int>::value == 1, "bare builtin-trait spelling stays an identifier");

int main() { return 0; }
