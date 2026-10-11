// VALIDATION: compile-pass
// N3485 focus: 13.5 [over.oper], 14.5.2 [temp.mem]

namespace active_owner_case
{

template<class Alloc>
struct nonconst_traits
{
  typedef typename Alloc::value_type * pointer;
  typedef long difference_type;
  typedef nonconst_traits<Alloc> nonconst_self;
};

template<class Alloc>
struct const_traits
{
  typedef const typename Alloc::value_type * pointer;
  typedef long difference_type;
  typedef nonconst_traits<Alloc> nonconst_self;
};

template<class Buff, class Traits>
struct iter
{
  typedef iter<Buff, typename Traits::nonconst_self> nonconst_self;
  typedef typename Traits::pointer pointer;
  typedef typename Traits::difference_type difference_type;

  pointer p;

  iter() : p(0) {}
  iter(const nonconst_self & other) : p(other.p) {}
  iter(pointer value) : p(value) {}

  template<class OtherTraits>
  difference_type operator-(const iter<Buff, OtherTraits> & other) const
  {
    return p - other.p;
  }

  iter & operator+=(difference_type n)
  {
    p = p + n;
    return *this;
  }

  iter operator+(difference_type n) const
  {
    iter result(*this);
    result += n;
    return result;
  }
};

template<class T>
struct allocator
{
  typedef T value_type;
};

template<class T, class Alloc>
struct buffer
{
  typedef iter<buffer<T, Alloc>, nonconst_traits<Alloc> > iterator;
  typedef iter<buffer<T, Alloc>, const_traits<Alloc> > const_iterator;

  T data[4];

  iterator begin()
  {
    return iterator(data);
  }

  long rotate(const_iterator next)
  {
    return next - begin();
  }
};

struct item
{
  int value;
};

} // namespace active_owner_case

int main()
{
  active_owner_case::buffer<
      active_owner_case::item,
      active_owner_case::allocator<active_owner_case::item> > b;
  return b.rotate(b.begin() + 2) == 2 ? 0 : 1;
}
