namespace std {
template<class T> struct initializer_list {
  const T *first;
  unsigned long count;
  initializer_list(const T *, unsigned long);
};
}

struct S {
  template<class T> S(const std::initializer_list<T> &);
};

int main() { S s = {1, 2, 3}; }
