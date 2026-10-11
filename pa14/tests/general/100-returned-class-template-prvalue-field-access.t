template<class T>
struct P {
  int first;
  int second;
};

template<class T>
P<T> make();

int f() {
  return make<int>().second;
}
