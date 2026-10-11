// HHC-315
template<class T>
unsigned f(T w) {
  return w < 64 ? unsigned(~0) >> 1 : unsigned(0);
}
