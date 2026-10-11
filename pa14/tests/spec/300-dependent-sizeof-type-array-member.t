// VALIDATION: compile-pass
// N3485 focus: 14.5.1 [temp.class], dependent class layout

template<class T>
struct real
{
  typedef T type;
};

template<class A, class Options>
struct base
{
  typedef unsigned long size_type;
  typedef A allocator_type;
  typedef int value_type;

  struct long_t
  {
    size_type length;
    A *start;
  };

  typedef long_t long_raw_t;
  static const size_type FinalInternalStorage = sizeof(long_t) / sizeof(value_type);

  struct short_t
  {
    value_type data[FinalInternalStorage];
  };

  union repr_t_size_t
  {
    long_raw_t r;
    short_t s;
  };

  union repr_t
  {
    long_raw_t r_aligner;
    short_t s_aligner;
    unsigned char data[sizeof(repr_t_size_t)];
  };

  struct members_holder : public allocator_type
  {
    repr_t m_repr;
  } members_;
};

template<class T, class Options>
struct box : base<typename real<T>::type, Options>
{
  typedef base<typename real<T>::type, Options> base_t;
  typedef typename base_t::size_type size_type;
  static const size_type npos = size_type(-1);
};

template<class T, class Options>
const typename box<T, Options>::size_type box<T, Options>::npos;

int main()
{
  return 0;
}
