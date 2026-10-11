template<class ToPad, bool = sizeof(ToPad) == 1>
struct Pad {};

template<class D>
struct Outer {
  using pointer = typename D::pointer;
  Pad<pointer> pad;
};

template<class D>
void f(Outer<D> & x) {}

int main() {
  return 0;
}
