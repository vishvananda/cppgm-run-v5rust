template<class ToPad, bool = sizeof(ToPad) == 1>
struct Pad {};

template<class D>
struct Outer {
  using pointer = typename D::pointer;
  using X = Pad<pointer>;
};

template<class D>
void f(Outer<D> & x) {}

int main() {
  return 0;
}
