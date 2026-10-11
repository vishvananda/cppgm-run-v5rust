template<class T>
T&& f(T& x) {
  return static_cast<T&&>(x);
}

unsigned long g(unsigned long& x) {
  return f<unsigned long>(x);
}

int main() {
  unsigned long x = 9;
  return g(x) - 9;
}
