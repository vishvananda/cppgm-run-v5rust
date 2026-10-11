// HHC-050
template<class... T>
struct Box;

template<>
struct Box<> {
  int get() { return 1; }
};

int main() {
  Box<> b;
  return b.get();
}
