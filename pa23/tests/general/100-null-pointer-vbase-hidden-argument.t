struct V
{
  V() : value(0) {}
  int value;
};

struct A : virtual V
{
  A() {}
};

int read(A * p)
{
  return p ? p->value : 0;
}

int main()
{
  return read(nullptr);
}

// VALIDATION: compile-pass
