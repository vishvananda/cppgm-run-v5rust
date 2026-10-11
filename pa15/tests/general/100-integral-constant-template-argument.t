constexpr int K = 3;

template<int N>
struct Box {
  int get() { return N; }
};

int main() {
  Box<K + 1> b;
  return b.get();
}
