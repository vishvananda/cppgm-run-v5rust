template<bool B>
struct bool_type;

template<>
struct bool_type<false> {
  typedef int type;
};

template<>
struct bool_type<true> {
  typedef int type;
};

template<unsigned long I, class... T>
typename bool_type<(I >= sizeof...(T))>::type size_check()
{
  return I;
}

int main()
{
  return size_check<0, int>();
}
