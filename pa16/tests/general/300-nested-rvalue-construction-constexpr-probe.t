struct Leaf {
  Leaf();
  Leaf(Leaf&&);
};

struct Middle {
  explicit Middle(Leaf&& source) :
    leaf(static_cast<Leaf&&>(source))
  {}

  Leaf leaf;
};

struct Outer {
  explicit Outer(Middle&& source) :
    middle(static_cast<Middle&&>(source))
  {}

  Middle middle;
};

void exercise()
{
  Outer value{Middle{Leaf{}}};
  (void)value;
}
