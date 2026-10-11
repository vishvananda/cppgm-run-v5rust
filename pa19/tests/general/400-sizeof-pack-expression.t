// HHC-126
template<class... I>
struct X {
  enum { size = sizeof...(I) };
};

int main() { return X<int, char>::size != 2; }
