// VALIDATION: compile-pass
// N3485 focus: 14.8.1 [temp.arg.explicit], 14.8.3 [temp.over]

template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class A, class B>
struct pair {};

template<class P, class I, class O, enable_if_t<true, int> = 0>
pair<I, O> g(I, int, O);

int main()
{
  return 0;
}
