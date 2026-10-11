struct Base
{
  template<class K>
  unsigned long erase(const K&)
  {
    return 1;
  }
};

struct Derived : Base
{
  typedef Base base_t;

  template<class K>
  unsigned long erase(const K&)
  {
    return 2;
  }

  using base_t::erase;
};

struct Key {};

int main()
{
  Derived d;
  Key k;
  return d.erase(k) == 2 ? 0 : 1;
}
