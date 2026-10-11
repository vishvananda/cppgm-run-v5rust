struct S {
  S(int);
};

void* operator new(unsigned long, void*) noexcept;

int main() {
  char buf[8];
  S* p = ::new((void*)buf) S(7);
  return p == 0;
}
