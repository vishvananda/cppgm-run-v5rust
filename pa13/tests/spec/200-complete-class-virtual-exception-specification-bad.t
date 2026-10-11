struct base
{
  virtual void f() noexcept;
};

struct enclosing
{
  struct derived : base
  {
    void f() noexcept(sizeof(enclosing) == 0) override;
  };
  int member;
};

int main() { return 0; }
