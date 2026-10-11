template<class T>
struct Wrap {};

template<class R, class T>
Wrap<R T::*> mem_wrap(R T::* p) {
  return Wrap<R T::*>();
}

int main() { return 0; }
