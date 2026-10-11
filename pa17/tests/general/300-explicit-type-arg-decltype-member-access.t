// Reduced from Boost.Asio's any_completion_handler allocator path. The
// explicit class-template type argument is a decltype of a local aggregate
// member inside an instantiated class-template member function.

struct allocator
{
  int value;
};

struct handler
{
  typedef allocator alloc_type;
};

template<class T>
struct traits
{
  typedef T type;

  static int check(T&)
  {
    return 0;
  }
};

template<class Handler>
struct impl
{
  struct deleter
  {
    typename Handler::alloc_type alloc;
  };

  template<class H>
  static int create(H)
  {
    deleter d{typename Handler::alloc_type()};
    typename traits<decltype(d.alloc)>::type* p = &d.alloc;
    return traits<decltype(d.alloc)>::check(*p);
  }
};

int main()
{
  return impl<handler>::create(0);
}
