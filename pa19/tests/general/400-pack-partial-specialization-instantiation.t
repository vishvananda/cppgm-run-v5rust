// HHC-052: intended outcome: should compile successfully and instantiate the partial specialization.
template<class... T>
struct Box;

template<class T, class U, class... Rest>
struct Box<T, U, Rest...> {
  int get() { return 1; }
};

int main() {
  Box<int, int, int> b;
  return b.get();
}
