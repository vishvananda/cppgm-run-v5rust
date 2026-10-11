template<int A>
struct alignas(A) AlignedAsValue {
  char c;
};

AlignedAsValue<16> x;

int main() {
  return alignof(AlignedAsValue<16>) == 16 ? 0 : 1;
}
