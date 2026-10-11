struct base
{
  virtual void f() noexcept;
};

struct derived : base
{
  void f() noexcept(sizeof(derived) > 0) override;
};

struct enclosing
{
  struct nested : base
  {
    void f() noexcept(sizeof(enclosing) > 0) override;
  };
  int member;
};

int main() { return 0; }
