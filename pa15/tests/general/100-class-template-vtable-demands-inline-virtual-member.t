// VALIDATION: compile-pass
// A concrete class-template vtable requires its inline virtual member body
// even when the source never calls that member directly.

template<class T>
struct box
{
  virtual int value()
  {
    static_assert(sizeof(T) > 0, "virtual member body is instantiated");
    return sizeof(T);
  }
};

box<int> value;

int main()
{
  return 0;
}
