// VALIDATION: compile-pass
// Declaring an implicit copy constructor for an enclosing class does not
// instantiate an unused class-template member's copy-constructor body.

struct noncopyable
{
  int value;

  noncopyable()
    : value(7)
  {
  }

  noncopyable(const noncopyable &) = delete;
};

template<class T>
struct deferred_copy
{
  T value;

  deferred_copy()
  {
  }

  deferred_copy(const deferred_copy & other)
    : value(other.value)
  {
  }
};

struct holder
{
  deferred_copy<noncopyable> value;

  holder()
  {
  }
};

int main()
{
  holder value;
  return value.value.value.value == 7 ? 0 : 1;
}
