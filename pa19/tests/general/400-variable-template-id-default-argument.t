// HHC-055: intended outcome: should parse and analyze the variable-template-id default argument as A<L>, not A < L.
template<int N>
const int A = 1;

template<int L, int X = A<L> >
struct S {};

int main() {
  return 0;
}
