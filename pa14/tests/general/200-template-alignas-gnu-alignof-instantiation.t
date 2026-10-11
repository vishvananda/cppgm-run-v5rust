template<class T>
struct alignas(__alignof(T)) AlignedAsT {
  char c;
};

AlignedAsT<unsigned long long> x;

int main() {
  return alignof(AlignedAsT<unsigned long long>) == alignof(unsigned long long) ? 0 : 1;
}
