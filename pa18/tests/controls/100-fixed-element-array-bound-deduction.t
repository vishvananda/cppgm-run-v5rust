// N3485 14.8.2.1, 14.8.2.5p17: fixed element types still deduce
// named non-type bounds through arrays and function types; expect exit 0
template<unsigned N> int extent(char (&)[N]) { return N; }
template<int N, int M> int matrix(int (&)[N][M]) { return N + M; }
template<int N> int pointed(int (*)[N]) { return N; }
template<int N> int returned(int (&(*)())[N]) { return N; }
template<int N> int qualified(const int (&)[(N)]) { return N; }
template<int N, class... T> int with_pack(char (&)[N], T...) { return N; }
template<int N> int together(char (&)[N], char (&)[N]) { return N; }
int together(...) { return -1; }
template<int N> int expression(char (&)[N + 1]) { return N; }
int expression(...) { return -1; }
template<int N> int expression_and_name(char (&)[N + 1], char (&)[N]) { return N; }
template<class T, unsigned N> T (&array_result(char const (&)[N]))[N]
{
  static T result[N] = {};
  return result;
}
int data[6];
int (&get_data())[6] { return data; }
struct Storage
{
  int value;
  template<int N> Storage(int, char (&)[N]) : value(N) {}
};
int main()
{
  char a[4] = {}, b[4] = {}, c[3] = {};
  int m[2][7] = {};
  const int q[5] = {};
  Storage s({}, a);
  return extent(a) == 4 && matrix(m) == 9 && pointed(&data) == 6
      && returned(get_data) == 6 && qualified(q) == 5
      && with_pack(a, 1, 2) == 4 && together(a, b) == 4
      && together(a, c) == -1 && expression(a) == -1
      && expression_and_name(a, c) == 3 && s.value == 4
      && sizeof(array_result<char>("abc")) == 4 ? 0 : 1;
}
