struct D {
  D(int);
};

template <class T>
struct Pad {
  T v;
  char c;
};

int main() {
  return sizeof(Pad<D>) - 2;
}
