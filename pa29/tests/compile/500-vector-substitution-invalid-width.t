using size_t = decltype(sizeof(0));
template<size_t N> using bytes __attribute__((vector_size(N))) = int;
bytes<12> invalid;
