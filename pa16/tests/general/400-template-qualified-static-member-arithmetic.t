template <class T>
struct Limits {
  static const int digits = sizeof(T) * 8;
};

constexpr int f() {
  return Limits<unsigned long>::digits - 1;
}

static_assert(f() == static_cast<int>(sizeof(unsigned long) * 8 - 1), "");

int main() {
  return f() == static_cast<int>(sizeof(unsigned long) * 8 - 1) ? 0 : 1;
}
