template<class T>
int f(T t) {
  return []<class U>(U u) -> int {
    return u;
  }(t);
}

int g() { return f(1); }
