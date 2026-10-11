enum { A = 1 };

template<int N>
struct Box {
  int get() { return N; }
};

int main() {
  Box<A> b;
  return b.get() - 1;
}
