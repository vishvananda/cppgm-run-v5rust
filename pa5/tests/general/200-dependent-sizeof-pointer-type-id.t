template<class T>
struct Holder {
  typedef T value_type;
  char pad[sizeof(value_type*)];
};
