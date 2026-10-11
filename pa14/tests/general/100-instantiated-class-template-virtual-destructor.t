// VALIDATION: compile-pass
// An instantiated class template reuses the PA12 virtual-destructor lifetime
// path.

template<class T>
struct owner
{
  virtual ~owner()
  {
  }
};

owner<int> value;

int main()
{
  return 0;
}
