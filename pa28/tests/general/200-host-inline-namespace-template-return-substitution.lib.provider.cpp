namespace abi_case {
inline namespace v2 {

template<class T>
struct traits {
};

template<class T>
struct alloc {
};

template<class T, class Traits, class Alloc>
struct box {
  int value;

  box() : value(0) {}
  explicit box(int input) : value(input) {}
};

template<class T, class Traits, class Alloc>
box<T, Traits, Alloc>
operator+(T const *, box<T, Traits, Alloc> const &);

template<>
box<char, traits<char>, alloc<char> >
operator+<char, traits<char>, alloc<char> >(
    char const *,
    box<char, traits<char>, alloc<char> > const & rhs)
{
  return box<char, traits<char>, alloc<char> >(rhs.value + 1);
}

}
}
