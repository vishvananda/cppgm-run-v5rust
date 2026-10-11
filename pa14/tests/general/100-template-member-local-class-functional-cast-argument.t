void sink(...);

template<class>
struct A
{
  void f()
  {
    struct L {};
    sink(L{});
  }
};

int main() { return 0; }
