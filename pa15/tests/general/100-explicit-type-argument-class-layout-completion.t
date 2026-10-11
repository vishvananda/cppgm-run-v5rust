template<class A, class B>
struct Pair { A a; B b; };

template<class T>
void check() {
  static_assert(sizeof(T) >= 0, "");
}

int main() {
  check<Pair<unsigned long, const char*>>();
}
