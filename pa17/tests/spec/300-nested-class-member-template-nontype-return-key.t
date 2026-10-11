// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem], 14.6.2.1 [temp.dep.type]

template<class Allocator>
struct allocator_property
{
};

template<class T>
struct allocator
{
};

struct context
{
  template<class Allocator, unsigned Bits>
  struct executor;
};

template<class Allocator, unsigned Bits>
struct context::executor
{
  char storage[Bits + 1];

  template<class OtherAllocator>
  executor<OtherAllocator, Bits>
  require(allocator_property<OtherAllocator>) const
  {
    return executor<OtherAllocator, Bits>();
  }

  template<class Function>
  void execute(Function&) const
  {
  }
};

int main()
{
  context::executor<allocator<void>, 4> source;
  allocator_property<allocator<void> > property;
  int function = 0;
  source.require(property).execute(function);
  return 0;
}
