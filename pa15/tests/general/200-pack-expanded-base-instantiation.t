namespace N {
  struct S {};

  template<class T>
  struct AlignedAsT {};

  template<class... Args>
  struct MaxAlignImpl : AlignedAsT<Args>... {};
}

int k[(alignof(N::MaxAlignImpl<N::S, int*>) > 0) ? 1 : -1];

int main() {
  return 0;
}
