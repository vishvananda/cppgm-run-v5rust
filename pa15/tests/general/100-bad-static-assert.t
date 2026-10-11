template<int N>
struct Box {
  static_assert(N > 0, "bad");
  int a[N];
};

int main() {
  Box<0> b;
  return 0;
}
