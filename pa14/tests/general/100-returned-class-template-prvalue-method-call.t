template<class T>
struct P {
  int second;
  int get() { return second; }
};

template<class T>
P<T> make();

int f() {
  return make<int>().get();
}
