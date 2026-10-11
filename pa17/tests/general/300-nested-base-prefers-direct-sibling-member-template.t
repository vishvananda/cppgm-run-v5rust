// VALIDATION: compile-pass
// An unqualified member-template base checks the enclosing class's direct
// members before inherited members of the same name.

template <typename>
struct facade
{
  template <typename, typename>
  struct advance {};
};

template <typename Tag>
struct iterator : facade<iterator<Tag> >
{
  template <typename, typename>
  struct advance
  {
    typedef int type;
  };

  template <typename It>
  struct next : advance<It, int> {};
};

typedef iterator<int>::next<iterator<int> >::type selected;

int main()
{
  return 0;
}
