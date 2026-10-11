template<class T1, class T2>
struct choose
{
  typedef T1 type;
};

template<class Ptr>
struct pointer_traits
{
  template<class U>
  struct rebind_pointer
  {
    typedef U * type;
  };
};

template<class Pointer>
struct iter
{
  typedef typename choose<
      typename pointer_traits<Pointer>::template rebind_pointer<int>::type,
      Pointer>::type pointer;
};

typedef iter<int>::pointer result_t;

int expect(int *);

int main()
{
  result_t p = 0;
  return expect(p);
}
