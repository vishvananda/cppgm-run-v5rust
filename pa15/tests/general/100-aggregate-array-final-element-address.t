struct Item
{
  Item* self;
  Item(int) noexcept : self(this) {}
  Item(const Item&) noexcept : self(this) {}
  Item(Item&&) noexcept : self(this) {}
};

template<class T, int N> struct Array { T elements[N]; };

int inspect(Array<Item, 2>&& array)
{
  return array.elements[0].self == &array.elements[0] &&
         array.elements[1].self == &array.elements[1] ? 0 : 1;
}

template<class T> int check()
{
  return inspect(T{Item(0), Item(1)});
}

int main() { return check<Array<Item, 2>>(); }
