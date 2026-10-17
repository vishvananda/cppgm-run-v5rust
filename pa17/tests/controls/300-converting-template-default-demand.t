// Selected converting constructor templates prepare omitted arguments for
// emitted calls. An unused inline body's default stays undemanded. expect exit 0
struct Deferred {
  template<class T> Deferred(T, int = T::missing) {}
};
inline void unused() { Deferred value = 1; }

int default_value() { return 7; }
struct Box {
  int value;
  template<class T> explicit(false) Box(T v, int n = default_value()) : value(v + n) {}
};
inline int used() {
  Box first = 1;
  Box second = 2;
  return first.value + second.value;
}
int main() { return used() - 17; }
