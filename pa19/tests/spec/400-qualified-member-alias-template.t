// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias], 14.6.2.1 [temp.dep.type]

template<class F>
struct c
{
  template<class T>
  using alias = T;
};

template<class T>
using z = typename c<T>::template alias<int>;

using w = z<int>;

int main()
{
  return 0;
}
