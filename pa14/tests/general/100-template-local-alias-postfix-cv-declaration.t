typedef int I;

template<class T>
int f(T t) {
  if (true) {
    I const x = t;
    return x;
  }
  return 0;
}

int g() { return f(1); }
