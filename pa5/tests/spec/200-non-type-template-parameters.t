// N3485 focus: 14.1 [temp.param] non-type template parameters
template<int N = 1, unsigned long... I, class T = int, template<class> class TT = Meta>
struct Holder {
  int values[N];
  static const int value = N;
};
