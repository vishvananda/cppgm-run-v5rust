struct cons
{
  int car;
};

struct iter
{
  iter(struct cons & value) : cons(value) {}
  struct cons & cons;
};

int main()
{
  cons value;
  value.car = 11;
  iter it(value);
  it.cons.car = 23;
  return value.car == 23 ? 0 : 1;
}
