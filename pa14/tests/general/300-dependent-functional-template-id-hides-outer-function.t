// Reduced from Boost.Xpressive basic_chset::set.
// In an instantiated template body, a dependent functional template-id naming
// an inner namespace class template must not be resolved as an outer namespace
// function template with the same unqualified name.

namespace outer {
namespace inner {

template<typename T>
struct range {
  range(T, T) {}
};

template<typename T>
void sink(range<T> const &) {}

template<typename T>
struct box {
  void set(T c);
};

template<typename T>
void box<T>::set(T c)
{
  sink(range<T>(c, c));
}

}

template<typename T>
int range(T, T)
{
  return 0;
}

}

int main()
{
  outer::inner::box<int> b;
  b.set(1);
  return 0;
}
