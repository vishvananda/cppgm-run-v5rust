struct base {
  virtual void f();
  virtual void g();
};

struct derived : base {
  void f() override;
  void g() final;
};

void h() noexcept(true);
