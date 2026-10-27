// A target can deduce trailing packs; without one, a lone template-id
// designates its already-formed specialization with those packs empty.
template<class... T>
int count(T...)
{
  return sizeof...(T);
}

template<class T = int, class... U>
int prefix(T, U...)
{
  return 1 + sizeof...(U);
}

struct C
{
  template<class... T>
  int member(T...)
  {
    return sizeof...(T);
  }

  template<class... T>
  static int shared(T...)
  {
    return sizeof...(T);
  }
};

auto empty_address = &count<>;
auto empty_name = count<>;
auto default_prefix = &prefix<>;
int (*two_arguments)(int, char) = &count<>;
auto empty_member = &C::member<>;
auto empty_static = &C::shared<>;
int (C::*two_members)(int, char) = &C::member<>;

int main()
{
  C object;
  return empty_address() + empty_name() + default_prefix(1) - 1
       + two_arguments(1, 'a') - 2 + (object.*empty_member)()
       + empty_static() + (object.*two_members)(1, 'a') - 2;
}
