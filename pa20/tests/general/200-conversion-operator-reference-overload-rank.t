struct Context {};

void sink(const Context &);

struct Wrapper
{
  Context & ctx;

  Wrapper(Context & c) : ctx(c) {}

  operator Context &() { return ctx; }
  operator const Context &() const { return ctx; }

  void call()
  {
    sink(*this);
  }
};

void sink(const Context &) {}

int main()
{
  Context c;
  Wrapper w(c);
  w.call();
  return 0;
}
