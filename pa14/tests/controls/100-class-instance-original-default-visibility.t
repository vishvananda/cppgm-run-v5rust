// N3485 8.3.6p4, 14.7.1p1: first-declaration defaults remain available when
// a calling template instantiates a class after its definition; expect exit 0
int calls;
int next() { return ++calls; }

template<class T> struct Box {
  int value;
  Box(T, int n = next()) : value(n) {}
  int read(int n = next()) { return value + n; }
  int unused(int n = T::missing) { return n; }
};

template<class T> struct Holder {
  Box<T> box;
  Holder(T t) : box(t) {}
  int read() { return box.read() + box.unused(4); }
};

template<class T> int run(T t) {
  Box<T> box(t);
  Holder<T> holder(t);
  return box.read() + holder.read();
}

int main() {
  return run(0) == 14 && calls == 4 &&
         run('x') == 30 && calls == 8 ? 0 : 1;
}
