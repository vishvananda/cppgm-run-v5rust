struct S {
  S(int&&);
};

void* operator new(unsigned long, void*) noexcept;

namespace std {
template<class T> T&& move(T&&);
}

int main() {
  int x = 0;
  char buf[8];
  S* p = ::new((void*)buf) S(std::move(x));
  return p == 0;
}
