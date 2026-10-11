constexpr int K = 3;

template<int N>
struct Box {
  static_assert(N > 0, "ok");
  int a[N];

  int get() { return sizeof(a); }
};

int main() {
  Box<K + 1> b;
  return b.get();
}
