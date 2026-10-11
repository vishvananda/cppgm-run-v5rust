template<int N>
struct Box {
  int get() { return N; }
};

int main() {
  Box<3> b;
  return b.get();
}
