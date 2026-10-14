// expect reject
struct Invalid {
  unsigned first : -1, second alignas(8) : 2;
};
