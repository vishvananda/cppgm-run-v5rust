// VALIDATION: compile-pass
// N3485 focus: 14.7.1 [temp.inst], 7.1.3 [dcl.typedef]

template<class T>
struct lazy_leaf {
  T value;
};

template<class... T>
struct lazy_tuple : lazy_leaf<T>... {};

using lazy_alias = lazy_tuple<int[]>;

int main()
{
  return 0;
}
