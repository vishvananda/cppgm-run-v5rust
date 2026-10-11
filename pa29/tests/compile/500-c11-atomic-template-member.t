template <class T>
struct box {
  _Atomic(T) value;
};

box<int> g;
