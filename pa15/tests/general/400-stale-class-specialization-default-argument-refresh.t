// VALIDATION: compile-pass
// N3485 focus: 14.6.4.1 [temp.point], 14.7.3 [temp.expl.spec]

template<class T>
struct traits;

template<class T, class Traits = traits<T> >
struct holder
{
  typedef typename Traits::value_type value_type;
};

typedef holder<char> early_holder;

template<>
struct traits<char>
{
  typedef char value_type;
};

int main()
{
  early_holder::value_type c = 0;
  return c;
}
