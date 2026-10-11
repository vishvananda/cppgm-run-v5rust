// An explicit instantiation names the member after its class, so the rest of
// the declarator is in that class's scope: libc++'s extern template list
// writes `basic_string<char>::__init(const value_type*, size_type)`, whose
// parameter types are members of the class named just before them.  The
// out-of-class definition already resolves them that way and the explicit
// instantiation has to agree, which also means completing the named
// specialization -- the instantiation is often the first thing to name it.

template<class T>
struct box
{
  typedef T value_type;
  typedef unsigned long size_type;

  void init(const value_type *, size_type);
  value_type first(const box &) const;
};

template<class T>
void box<T>::init(const value_type *, size_type)
{
}

template<class T>
typename box<T>::value_type box<T>::first(const box &) const
{
  return value_type();
}

// Both spellings of the member types, and a parameter naming the class itself.
extern template void box<char>::init(const value_type *, size_type);
extern template box<char>::value_type box<char>::first(const box &) const;

// The matching definitions, whose declarators name the member types the same
// way, so both sides of the explicit instantiation are exercised.
template void box<char>::init(const value_type *, size_type);
template box<char>::value_type box<char>::first(const box &) const;

int main()
{
  box<char> made;
  made.init(0, 0);
  return made.first(made) == 0 ? 0 : 1;
}
