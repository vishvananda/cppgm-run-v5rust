template<bool Value>
struct Flag
{
};

struct Worker
{
  void select(Flag<false>)
  {
  }

  void select(Flag<true>)
  {
  }

  template<class T>
  int run()
  {
    select(Flag<false>());
    return sizeof(T) == sizeof(int) ? 0 : 1;
  }
};

template<class U>
int call_member_template()
{
  typedef U value_type;
  Worker worker;
  return worker.template run<value_type>();
}

int main()
{
  return call_member_template<int>();
}
