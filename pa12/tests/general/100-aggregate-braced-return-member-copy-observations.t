int copies;
int moves;
int returned_copies;
int returned_moves;

struct Monitor
{
  ~Monitor() noexcept
  {
    returned_copies = copies;
    returned_moves = moves;
  }
};

struct Value
{
  Value* self;
  int number;
  Value(int number) noexcept : self(this), number(number) {}
  Value(const Value& value) noexcept : self(this), number(value.number)
  { ++copies; }
  Value(Value&& value) noexcept : self(this), number(value.number)
  { ++moves; }
};

struct Aggregate { int prefix; Value first; Value second; };

Aggregate make(Value& first, Value& second)
{
  Monitor monitor;
  return {1, first, second};
}

int main()
{
  Value first(3), second(4);
  Aggregate result = make(first, second);
  return returned_copies == 2 && returned_moves == 0 && result.prefix == 1 &&
         result.first.number == 3 && result.second.number == 4 &&
         result.first.self == &result.first && result.second.self == &result.second
         ? 0 : 1;
}
