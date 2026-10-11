template<class T>
struct Wrap {};

template<class R, class T>
inline Wrap<R T::*> mem_wrap(R T::* p) noexcept {
  return Wrap<R T::*>();
}

int main() { return 0; }
