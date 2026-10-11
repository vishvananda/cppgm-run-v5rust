template<int N>
struct Box {
  int get() { return 1; }
};

template<>
struct Box<3> {
  int get() { return 5; }
};

int main() {
  Box<3> b;
  return b.get();
}
