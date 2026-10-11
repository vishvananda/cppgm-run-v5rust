// lowir.md storage=readonly: the lowering's own constant data states
// storage=readonly: a polymorphic class's vtable, the RTTI object its vtable
// points to and that object's name string.
struct Shape
{
  virtual int sides() const { return 0; }
  virtual ~Shape() {}
};
struct Square : Shape
{
  int sides() const { return 4; }
};

int count(const Shape& shape) { return shape.sides(); }

int main()
{
  Square square;
  Shape plain;
  return count(square) + count(plain) == 4 ? 0 : 1;
}
