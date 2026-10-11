namespace std {
  struct identity {};

  template<class T, class Proj>
  T* __find(T* first, T* last, const T& value, Proj& proj) { return first; }

  template<class T, class U, class Proj, int = 0>
  T* __find(T* first, T* last, const U& value, Proj& proj) { return last; }
}

typedef unsigned long size_t;

const char16_t* g(const char16_t* s, size_t n, const char16_t& a) {
  std::identity proj;
  return std::__find(s, s + n, a, proj);
}
