// N3485 [class.dtor]: explicit unqualified destructor calls can dispatch virtually.
void operator delete(void*) noexcept;
int trace;

struct Base
{
  Base() noexcept = default;
  virtual ~Base() noexcept { trace = trace * 10 + 1; }
  virtual void destroy() noexcept {}
};

struct Derived : Base
{
  Derived() noexcept = default;
  ~Derived() noexcept override { trace = trace * 10 + 2; }

  void destroy() noexcept override
  {
    this->~Derived();
  }
};

struct Further : Derived
{
  Further() noexcept = default;
  ~Further() noexcept override { trace = trace * 10 + 3; }
};

int main()
{
  Derived* complete = new Derived;
  Base* base = complete;
  base->~Base();
  ::operator delete(complete);
  if (trace != 21) return 1;

  trace = 0;
  complete = new Derived;
  base = complete;
  base->Base::~Base();
  ::operator delete(complete);
  if (trace != 1) return 2;

  trace = 0;
  Further* further = new Further;
  further->destroy();
  ::operator delete(further);
  return trace == 321 ? 0 : 3;
}
