struct Empty {};

template<class Tag, class Next = Empty, class Root = void>
struct Chain : Next {
  int value;
  Chain(int v) : Next(), value(v) {}
  Chain(Chain const& other) : Next(other), value(other.value + 1) {}
  Chain(int v, Next const& tail) : Next(tail), value(v) {}
};

int main() {
  Chain<int> first(3);
  Chain<char, Chain<int> > second(5, first);
  return second.value;
}
