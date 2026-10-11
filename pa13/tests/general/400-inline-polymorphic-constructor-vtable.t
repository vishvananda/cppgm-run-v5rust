struct Base
{
  virtual void destroy() = 0;

protected:
  Base() noexcept
  {
  }

  virtual ~Base()
  {
  }
};

struct Impl : Base
{
  static Base* create()
  {
    return new Impl();
  }

  Impl() noexcept
  {
  }

  void destroy()
  {
    delete this;
  }
};

int main()
{
  Base* base = Impl::create();
  base->destroy();
  return 0;
}
