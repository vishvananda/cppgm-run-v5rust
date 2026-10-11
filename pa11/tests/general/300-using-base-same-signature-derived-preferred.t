struct Base
{
  typedef int key_type;
  unsigned long erase(const key_type&) { return 1; }
};

struct Derived : Base
{
  typedef Base base_t;
  unsigned long erase(const key_type&) { return 2; }
  using base_t::erase;
};

int main()
{
  Derived d;
  int x = 0;
  return d.erase(x) == 2 ? 0 : 1;
}
