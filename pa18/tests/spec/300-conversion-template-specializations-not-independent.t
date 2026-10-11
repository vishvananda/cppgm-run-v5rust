// VALIDATION: compile-pass
// N3485 focus: 12.3.2 [class.conv.fct], 14.8.2.3 [temp.deduct.conv]

struct value {};

struct source
{
  template<class T>
  explicit operator T*() const { return nullptr; }
};

int main()
{
  source x;
  value* first = static_cast<value*>(x);
  const value* second = static_cast<const value*>(x);
  return first == second ? 0 : 1;
}
