template <unsigned long N, unsigned long D> struct ratio { static const unsigned long num = N; };
template <bool B, class T> struct enable_if {};
template <class T> struct enable_if<true, T> { typedef T type; };
template <class T> T declval();
struct engine {
  template <unsigned long N, unsigned long D,
            typename enable_if<(ratio<N, D>::num > 0xFFFFFFFFFFFFFFFFull / (7 - 2)), int>::type = 0>
  int eval(ratio<N, D>) { return 1; }
  template <unsigned long N, unsigned long D,
            typename enable_if<ratio<N, D>::num <= 0xFFFFFFFFFFFFFFFFull / (7 - 2), int>::type = 0>
  int eval(ratio<N, D>) { return 2; }
};
template <int N> struct box { static const int value = N; };
int pick(int a, int b) { return a > b ? a : b; }
int arr[4];
int main()
{
  engine e;
  return e.eval(ratio<1, 2>()) + box<pick(3 > 2, 1)>::value + box<(arr[3 > 2] > 0)>::value + box<sizeof(int[2 > 1 ? 4 : 8])>::value;
}
