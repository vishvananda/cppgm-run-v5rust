// Both emitted arms require concrete destructor definitions, even when a
// local condition selects only one of them at runtime.
int constructions;
int destructions;
int invalid_destructions;

struct Left
{
  int value;
  Left() : value(7) { ++constructions; }
  ~Left()
  {
    if (value != 7) ++invalid_destructions;
    ++destructions;
  }
};

struct Right
{
  int value;
  Right() : value(9) { ++constructions; }
  ~Right()
  {
    if (value != 9) ++invalid_destructions;
    ++destructions;
  }
};

int read(const Left & object) { return object.value; }
int read(const Right & object) { return object.value; }

int main()
{
  bool first = false;
  int result = first ? read(Left()) : read(Right());
  return result != 9 || constructions != 1 || destructions != 1 ||
    invalid_destructions != 0;
}
