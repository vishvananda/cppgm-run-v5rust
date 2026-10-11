struct base
{
  int value;

  base() : value(3) {}
  int get() const { return value; }
};

struct derived : base
{
  virtual int dispatch() const { return get(); }
};

int through_pointer(derived *object)
{
  base *subobject = object;
  return subobject->get();
}

int through_reference(derived &object)
{
  base &subobject = object;
  return subobject.get();
}

int main()
{
  derived object;
  return object.dispatch() + through_pointer(&object) +
         through_reference(object) - 9;
}
