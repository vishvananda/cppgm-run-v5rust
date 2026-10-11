struct Data { int value; };
struct Interface { virtual int read() const = 0; };

struct Derived : Data, Interface {
  int read() const { return static_cast<Data const *>(this)->value; }
};

int main()
{
  Derived value;
  value.value = 7;
  Interface const * view = &value;
  return view->read() == 7 ? 0 : 1;
}
