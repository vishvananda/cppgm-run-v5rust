template<int N>
struct Holder {
  int count() const;
};

template<int N>
inline int Holder<N>::count() const {
  return static_cast<unsigned _BitInt(N)>(0);
}

int main() { return 0; }
