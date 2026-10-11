template<class R, class T>
struct Wrap {};

template<class R, class T>
Wrap<R, T> mem_fun_wrap(R (T::* f)() const) {
  return Wrap<R, T>();
}

int main() { return 0; }
