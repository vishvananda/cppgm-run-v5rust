namespace std {
template<class T> class initializer_list {
  const T *__begin_;
  unsigned long __size_;
public:
  const T *begin() const { return __begin_; }
};
}

int second(std::initializer_list<int> xs) {
  const int *p = xs.begin();
  return p[1];
}

int main() {
  return second({13, 14, 15}) - 14;
}
