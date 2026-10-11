// HHC-149
template<class T, T v>
struct integral_constant
{
  typedef integral_constant type;
  static const T value = v;
};

namespace detail_complete
{
template<int N>
struct complete_tag
{
  double d;
  char c[N];
};

template<class T>
complete_tag<sizeof(T)> check(int);

template<class T>
char check(...);
}

template<class T>
struct is_complete
  : integral_constant<bool,
                      (sizeof(detail_complete::check<T>(0)) != sizeof(char))>
{
};

struct sequence
{
};

int main()
{
  typedef typename is_complete<sequence>::type result;
  return result::value ? 0 : 1;
}
