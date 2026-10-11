template<class T>
T* f(T* first, T* last, const T& value);

const char16_t* g(const char16_t* s, const char16_t& a) {
  return f(s, s, a);
}
