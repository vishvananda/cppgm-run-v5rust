int constructed;
int destroyed;
int bad_addresses;

struct Box
{
  Box* self;
  int value;
  Box() noexcept : self(this), value(++constructed) {}
  ~Box() noexcept
  {
    if (self != this) ++bad_addresses;
    ++destroyed;
  }
};

struct Ext
{
  Box elements[3];
  int tag;
};

struct Grid
{
  Box elements[2][2];
};

Ext global{};

int main()
{
  if (constructed != 3 || destroyed != 0 || global.tag != 0 ||
      global.elements[2].value != 3) return 1;
  {
    Ext local{};
    if (constructed != 6 || local.tag != 0 ||
        local.elements[0].value != 4 || local.elements[2].value != 6 ||
        local.elements[1].self != &local.elements[1]) return 2;
  }
  if (destroyed != 3 || bad_addresses != 0) return 3;
  {
    Grid grid{};
    if (constructed != 10 || grid.elements[1][1].value != 10 ||
        grid.elements[1][0].self != &grid.elements[1][0]) return 4;
  }
  return destroyed == 7 && bad_addresses == 0 ? 0 : 5;
}
