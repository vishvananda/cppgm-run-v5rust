template<typename T>
struct Box {
  int get() { return 1; }
};

template<>
struct Box<int> {
  int get() { return 2; }
};

int main() {
  Box<int> b;
  return b.get();
}
