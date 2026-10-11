// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct adaptor
{
  template<bool>
  struct range;

  typedef range<false> const_range;

  const_range data() const;
};

template<class T>
template<bool Mutable>
struct adaptor<T>::range
{
  struct iterator;

  iterator begin() const;

  T value;
};

template<class T>
template<bool Mutable>
struct adaptor<T>::range<Mutable>::iterator
{
  iterator();

  T value;
};

template<class T>
template<bool Mutable>
adaptor<T>::range<Mutable>::iterator::iterator()
  : value()
{
}

template<class T>
template<bool Mutable>
typename adaptor<T>::template range<Mutable>::iterator
adaptor<T>::range<Mutable>::begin() const
{
  return iterator();
}

template<class T>
typename adaptor<T>::const_range adaptor<T>::data() const
{
  return const_range();
}

template struct adaptor<int>;

int main()
{
  adaptor<int> value;
  return value.data().begin().value;
}
