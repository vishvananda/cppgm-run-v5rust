extern "C" int puts(const char *);

struct clone_base {
  virtual clone_base const * clone() const = 0;
  virtual void rethrow() const = 0;
  virtual ~clone_base() {}
};

struct info_base {
  info_base();
  info_base(info_base const & other);
  virtual ~info_base() = 0;

  mutable void * data_;
};

struct exception_base {
  virtual ~exception_base() {}
};

struct payload : virtual info_base, exception_base {
  explicit payload(int value);
  ~payload() override;

  int value_;
};

struct empty_tail {
};

template<class E>
struct wrapper : clone_base, E, empty_tail {
  explicit wrapper(E const & e) : E(e)
  {
    copy_from(&e);
  }

  void copy_from(void const *)
  {
  }

  void copy_from(info_base const * p)
  {
    static_cast<info_base &>(*this) = *p;
  }

  clone_base const * clone() const override
  {
    return 0;
  }

  void rethrow() const override
  {
  }
};

template<class E>
void throw_wrapped(E const & e)
{
  throw wrapper<E>(e);
}

template<class E>
E const & set_info(E const & x, void * value)
{
  x.data_ = value;
  return x;
}

template<class E>
E const & add_info(E const & x)
{
  return set_info(x, (void *)&x);
}

void thrower();
