template<class T>
int f(T t) {
  return [&]<int... I>(int) -> int {
    return t;
  }(0);
}

int g() { return f(1); }
