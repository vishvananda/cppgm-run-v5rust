typedef unsigned long size_t;

template<class T, size_t N>
struct take {
  typedef T type;
};

template<class L, class J>
using take_from_value = typename take<L, size_t{J::value}>::type;

template<int N>
struct index {
  static const int value = N;
};

take_from_value<int, index<1> > *run()
{
  return 0;
}
