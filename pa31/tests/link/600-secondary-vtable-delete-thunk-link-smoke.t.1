extern "C" int printf(const char *, ...);

struct primary_base
{
  virtual ~primary_base() {}
};

struct secondary_a
{
  virtual int tag() const { return 1; }
  virtual ~secondary_a() {}
};

struct secondary_b
{
  virtual int tag() const { return 2; }
  virtual ~secondary_b() {}
};

struct tracked
{
  explicit tracked(int *live) : live_(live)
  {
    ++*live_;
  }

  ~tracked()
  {
    --*live_;
  }

  int *live_;
};

struct multi_secondary : primary_base, secondary_a, secondary_b
{
  explicit multi_secondary(int *live) : marker(live) {}
  ~multi_secondary() {}

  tracked marker;
};

void delete_secondary_b(secondary_b *value)
{
  delete value;
}

int main()
{
  int live = 0;
  multi_secondary *value = new multi_secondary(&live);
  if(live != 1) {
    return 1;
  }
  delete_secondary_b(value);
  printf("%d\n", live);
  return live == 0 ? 0 : 2;
}
